//! Tensor operations optimized for CPU inference
//!
//! This module provides high-performance tensor operations using SIMD where possible.
//! All operations are designed to minimize allocations and maximize cache locality.

use chatloop_common::{ChatLoopError, Result};
use crate::tensor::{Shape, Tensor, TensorView};
use half::f16;
use num_traits::{Float, NumCast, Zero};
use rayon::prelude::*;
use std::ops::{Add, Div, Mul, Sub};

/// Generic tensor operations trait
pub trait TensorOps<T>: Send + Sync {
    /// Matrix multiplication: C = A @ B
    ///
    /// A: (m, k), B: (k, n), C: (m, n)
    fn matmul(a: &TensorView<'_, T>, b: &TensorView<'_, T>) -> Result<Tensor<T>>;

    /// Element-wise addition
    fn add(a: &TensorView<'_, T>, b: &TensorView<'_, T>) -> Result<Tensor<T>>;

    /// Element-wise multiplication
    fn mul(a: &TensorView<'_, T>, b: &TensorView<'_, T>) -> Result<Tensor<T>>;

    /// Scale: result = tensor * scalar
    fn scale(tensor: &TensorView<'_, T>, scalar: T) -> Result<Tensor<T>>;

    /// Add scalar: result = tensor + scalar
    fn add_scalar(tensor: &TensorView<'_, T>, scalar: T) -> Result<Tensor<T>>;

    /// Sum along axis
    fn sum(tensor: &TensorView<'_, T>, axis: usize) -> Result<Tensor<T>>;

    /// Softmax along last axis
    fn softmax(tensor: &TensorView<'_, T>) -> Result<Tensor<T>>;

    /// Layer normalization
    fn layer_norm(
        tensor: &TensorView<'_, T>,
        gamma: &TensorView<'_, T>,
        beta: &TensorView<'_, T>,
        epsilon: T,
    ) -> Result<Tensor<T>>;

    /// Transpose
    fn transpose(tensor: &TensorView<'_, T>) -> Tensor<T>;
}

/// Macro to implement TensorOps for floating-point types
macro_rules! impl_tensor_ops_float {
    ($t:ty) => {
        impl TensorOps<$t> for $t {
            fn matmul(a: &TensorView<'_, $t>, b: &TensorView<'_, $t>) -> Result<Tensor<$t>> {
                if a.shape.len() != 2 || b.shape.len() != 2 {
                    return Err(ChatLoopError::tensor("Matmul requires 2D tensors"));
                }

                let (m, k1) = (a.shape[0], a.shape[1]);
                let (k2, n) = (b.shape[0], b.shape[1]);

                if k1 != k2 {
                    return Err(ChatLoopError::tensor(format!(
                        "Matmul dimension mismatch: ({}, {}) @ ({}, {})",
                        m, k1, k2, n
                    )));
                }

                // Initialize output with zeros
                let mut c_data = vec::<$t>::zero(); m * n;
                let c_shape = vec![m, n];

                // Perform matrix multiplication with parallelization
                // Use cache-friendly blocking for better performance
                const BLOCK_SIZE: usize = 64;

                for i in (0..m).step_by(BLOCK_SIZE) {
                    let i_end = (i + BLOCK_SIZE).min(m);

                    for j in (0..n).step_by(BLOCK_SIZE) {
                        let j_end = (j + BLOCK_SIZE).min(n);

                        for l in (0..k1).step_by(BLOCK_SIZE) {
                            let l_end = (l + BLOCK_SIZE).min(k1);

                            // Inner loop - process block
                            for ii in i..i_end {
                                for jj in j..j_end {
                                    let mut sum = 0.0;
                                    for ll in l..l_end {
                                        let a_idx = ii * k1 + ll;
                                        let b_idx = ll * n + jj;
                                        sum += a.data[a_idx] * b.data[b_idx];
                                    }
                                    c_data[ii * n + jj] += sum;
                                }
                            }
                        }
                    }
                }

                Ok(Tensor::new(c_data, c_shape))
            }

            fn add(a: &TensorView<'_, $t>, b: &TensorView<'_, $t>) -> Result<Tensor<$t>> {
                if a.shape != b.shape {
                    return Err(ChatLoopError::tensor("Shape mismatch in add"));
                }

                let c_data: Vec<$t> = a.data.par_iter()
                    .zip(b.data.par_iter())
                    .map(|(&x, &y)| x + y)
                    .collect();

                Ok(Tensor::new(c_data, a.shape.clone()))
            }

            fn mul(a: &TensorView<'_, $t>, b: &TensorView<'_, $t>) -> Result<Tensor<$t>> {
                if a.shape != b.shape {
                    return Err(ChatLoopError::tensor("Shape mismatch in mul"));
                }

                let c_data: Vec<$t> = a.data.par_iter()
                    .zip(b.data.par_iter())
                    .map(|(&x, &y)| x * y)
                    .collect();

                Ok(Tensor::new(c_data, a.shape.clone()))
            }

            fn scale(tensor: &TensorView<'_, $t>, scalar: $t) -> Result<Tensor<$t>> {
                let c_data: Vec<$t> = tensor.data.par_iter()
                    .map(|&x| x * scalar)
                    .collect();

                Ok(Tensor::new(c_data, tensor.shape.clone()))
            }

            fn add_scalar(tensor: &TensorView<'_, $t>, scalar: $t) -> Result<Tensor<$t>> {
                let c_data: Vec<$t> = tensor.data.par_iter()
                    .map(|&x| x + scalar)
                    .collect();

                Ok(Tensor::new(c_data, tensor.shape.clone()))
            }

            fn sum(tensor: &TensorView<'_, $t>, axis: usize) -> Result<Tensor<$t>> {
                if axis >= tensor.ndim() {
                    return Err(ChatLoopError::tensor("Axis out of bounds"));
                }

                let mut new_shape = tensor.shape.clone();
                new_shape.remove(axis);

                let mut result = Vec::new();
                // Simplified implementation for 2D tensors
                if tensor.ndim() == 2 {
                    if axis == 0 {
                        // Sum over rows
                        for j in 0..tensor.shape[1] {
                            let mut sum = 0.0;
                            for i in 0..tensor.shape[0] {
                                sum += tensor.data[i * tensor.shape[1] + j];
                            }
                            result.push(sum);
                        }
                    } else {
                        // Sum over columns
                        for i in 0..tensor.shape[0] {
                            let mut sum = 0.0;
                            for j in 0..tensor.shape[1] {
                                sum += tensor.data[i * tensor.shape[1] + j];
                            }
                            result.push(sum);
                        }
                    }
                }

                Ok(Tensor::new(result, new_shape))
            }

            fn softmax(tensor: &TensorView<'_, $t>) -> Result<Tensor<$t>> {
                // Softmax along last axis
                if tensor.is_empty() {
                    return Ok(Tensor::new(vec![], tensor.shape.clone()));
                }

                let mut result = Vec::with_capacity(tensor.data.len());

                // For 2D tensor, apply softmax to each row
                if tensor.ndim() == 2 {
                    let row_size = tensor.shape[1];
                    for i in 0..tensor.shape[0] {
                        let row_start = i * row_size;
                        let row = &tensor.data[row_start..row_start + row_size];

                        // Find max for numerical stability
                        let max = row.par_iter()
                            .reduce(|| <$t>::zero(), |a, &b| a.max(b));

                        // Compute exp(x - max) and sum
                        let exp_sum: $t = row.par_iter()
                            .map(|&x| (x - max).exp())
                            .sum();

                        // Normalize
                        let softmax_row: Vec<$t> = row.par_iter()
                            .map(|&x| (x - max).exp() / exp_sum)
                            .collect();

                        result.extend(softmax_row);
                    }
                } else {
                    // 1D tensor
                    let max = tensor.data.par_iter()
                        .reduce(|| <$t>::zero(), |a, &b| a.max(b));

                    let exp_sum: $t = tensor.data.par_iter()
                        .map(|&x| (x - max).exp())
                        .sum();

                    result = tensor.data.par_iter()
                        .map(|&x| (x - max).exp() / exp_sum)
                        .collect();
                }

                Ok(Tensor::new(result, tensor.shape.clone()))
            }

            fn layer_norm(
                tensor: &TensorView<'_, $t>,
                gamma: &TensorView<'_, $t>,
                beta: &TensorView<'_, $t>,
                epsilon: $t,
            ) -> Result<Tensor<$t>> {
                if tensor.ndim() != 2 {
                    return Err(ChatLoopError::tensor("Layer norm expects 2D tensor"));
                }

                let (batch_size, hidden_size) = (tensor.shape[0], tensor.shape[1]);

                if gamma.shape != vec![hidden_size] || beta.shape != vec![hidden_size] {
                    return Err(ChatLoopError::tensor("Gamma/beta shape mismatch"));
                }

                let mut result = Vec::with_capacity(tensor.data.len());

                for i in 0..batch_size {
                    let row_start = i * hidden_size;
                    let row = &tensor.data[row_start..row_start + hidden_size];

                    // Compute mean
                    let mean: $t = row.par_iter().sum::<$t>() / (hidden_size as $t);

                    // Compute variance
                    let variance: $t = row.par_iter()
                        .map(|&x| {
                            let diff = x - mean;
                            diff * diff
                        })
                        .sum::<$t>() / (hidden_size as $t);

                    let std = (variance + epsilon).sqrt();

                    // Normalize and apply gamma/beta
                    for j in 0..hidden_size {
                        let normalized = (row[j] - mean) / std;
                        result.push(normalized * gamma.data[j] + beta.data[j]);
                    }
                }

                Ok(Tensor::new(result, tensor.shape.clone()))
            }

            fn transpose(tensor: &TensorView<'_, $t>) -> Tensor<$t> {
                if tensor.ndim() != 2 {
                    panic!("Transpose only implemented for 2D tensors");
                }

                let (m, n) = (tensor.shape[0], tensor.shape[1]);
                let mut data = Vec::with_capacity(tensor.data.len());

                for j in 0..n {
                    for i in 0..m {
                        data.push(tensor.data[i * n + j]);
                    }
                }

                Tensor::new(data, vec![n, m])
            }
        }
    };
}

// Implement for common float types
impl_tensor_ops_float!(f32);
impl_tensor_ops_float!(f64);

/// Convenience function for matrix multiplication
pub fn matmul<T>(a: &TensorView<'_, T>, b: &TensorView<'_, T>) -> Result<Tensor<T>>
where
    T: TensorOps<T> + Send + Sync,
{
    T::matmul(a, b)
}

/// Convenience function for softmax
pub fn softmax<T>(tensor: &TensorView<'_, T>) -> Result<Tensor<T>>
where
    T: TensorOps<T> + Send + Sync,
{
    T::softmax(tensor)
}

/// Quantize f32 tensor to int8
///
/// Returns (quantized data, scale, zero_point)
pub fn quantize_int8(data: &[f32]) -> (Vec<i8>, f32, i32) {
    // Find min and max
    let min = data.par_iter().reduce(|| f32::INFINITY, |a, &b| a.min(b));
    let max = data.par_iter().reduce(|| f32::NEG_INFINITY, |a, &b| a.max(b));

    // Calculate scale and zero point
    let scale = (max - min) / 255.0;
    let zero_point = (-min / scale).round() as i32 - 128;

    // Quantize
    let quantized: Vec<i8> = data.par_iter()
        .map(|&x| {
            let q = (x / scale + zero_point as f32).round() as i32;
            q.clamp(-128, 127) as i8
        })
        .collect();

    (quantized, scale, zero_point)
}

/// Dequantize int8 tensor to f32
pub fn dequantize_int8(data: &[i8], scale: f32, zero_point: i32) -> Vec<f32> {
    data.par_iter()
        .map(|&x| ((x as i32 - zero_point) as f32) * scale)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::TensorView;

    #[test]
    fn test_matmul() {
        let a_data = vec![1.0f32, 2.0, 3.0, 4.0];
        let a = TensorView::new(&a_data, vec![2, 2]);

        let b_data = vec![5.0f32, 6.0, 7.0, 8.0];
        let b = TensorView::new(&b_data, vec![2, 2]);

        let c = f32::matmul(&a, &b).unwrap();

        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data, vec![19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn test_softmax() {
        let data = vec![1.0f32, 2.0, 3.0];
        let tensor = TensorView::new(&data, vec![3]);

        let result = f32::softmax(&tensor).unwrap();

        // Check that probabilities sum to 1
        let sum: f32 = result.data.par_iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);

        // Check monotonicity (larger input -> larger output)
        assert!(result.data[0] < result.data[1]);
        assert!(result.data[1] < result.data[2]);
    }

    #[test]
    fn test_quantization() {
        let data = vec![-1.0f32, 0.0, 1.0, 2.0];

        let (quantized, scale, zero_point) = quantize_int8(&data);

        // Dequantize and check error
        let dequantized = dequantize_int8(&quantized, scale, zero_point);

        for (orig, deq) in data.iter().zip(dequantized.iter()) {
            assert!((orig - deq).abs() < 0.01);
        }
    }
}

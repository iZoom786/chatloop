//! Tensor operations and data structures
//!
//! This module provides tensor types and operations optimized for CPU inference.
//! All operations are designed to be SIMD-friendly and minimize allocations.

pub mod safetensors;
pub mod ops;

pub use safetensors::{SafeTensorBuffer, SafeTensorView, TensorDType};
pub use ops::{TensorOps, matmul, quantize_int8, dequantize_int8};

use std::fmt;

/// Tensor shape
pub type Shape = Vec<usize>;

/// Tensor strides
pub type Strides = Vec<isize>;

/// A tensor view with reference semantics
///
/// This type provides a zero-copy view into tensor data.
#[derive(Debug, Clone)]
pub struct TensorView<'a, T> {
    pub data: &'a [T],
    pub shape: Shape,
    pub strides: Strides,
}

impl<'a, T> TensorView<'a, T>
where
    T: Copy,
{
    /// Create a new tensor view
    pub fn new(data: &'a [T], shape: Shape) -> Self {
        let strides = compute_strides(&shape);
        Self { data, shape, strides }
    }

    /// Get the total number of elements
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the tensor is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Calculate the linear index from multi-dimensional indices
    pub fn index(&self, indices: &[usize]) -> usize {
        indices
            .iter()
            .zip(self.strides.iter())
            .map(|(&i, &s)| (i as isize * s) as usize)
            .sum()
    }

    /// Get a value at the given indices
    pub fn get(&self, indices: &[usize]) -> T {
        let idx = self.index(indices);
        self.data[idx]
    }

    /// Reshape the tensor (no-op if size matches)
    pub fn reshape(&self, new_shape: Shape) -> Option<TensorView<'a, T>> {
        let total_size: usize = new_shape.iter().product();
        if total_size != self.len() {
            return None;
        }
        Some(TensorView::new(self.data, new_shape))
    }

    /// Transpose the tensor (reverse dimensions)
    pub fn transpose(&self) -> TensorView<'a, T> {
        let mut new_shape = self.shape.clone();
        new_shape.reverse();

        let mut new_strides = self.strides.clone();
        new_strides.reverse();

        TensorView {
            data: self.data,
            shape: new_shape,
            strides: new_strides,
        }
    }
}

/// Compute row-major strides from shape
fn compute_strides(shape: &[usize]) -> Strides {
    let mut strides = Vec::with_capacity(shape.len());
    let mut stride: isize = 1;

    for &dim in shape.iter().rev() {
        strides.push(stride);
        stride *= dim as isize;
    }

    strides.reverse();
    strides
}

/// Owned tensor with heap-allocated data
///
/// This is used for intermediate activations and when ownership is needed.
#[derive(Debug, Clone)]
pub struct Tensor<T> {
    pub data: Vec<T>,
    pub shape: Shape,
}

impl<T> Tensor<T>
where
    T: Copy,
{
    /// Create a new tensor from data and shape
    pub fn new(data: Vec<T>, shape: Shape) -> Self {
        Self { data, shape }
    }

    /// Create a zero tensor
    pub fn zeros(shape: Shape) -> Self
    where
        T: Default,
    {
        let size: usize = shape.iter().product();
        let data = vec![T::default(); size];
        Self { data, shape }
    }

    /// Get the total number of elements
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the tensor is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get a view of this tensor
    pub fn view(&self) -> TensorView<'_, T> {
        TensorView::new(&self.data, self.shape.clone())
    }

    /// Reshape the tensor
    pub fn reshape(mut self, new_shape: Shape) -> Option<Self> {
        let total_size: usize = new_shape.iter().product();
        if total_size != self.len() {
            return None;
        }
        self.shape = new_shape;
        Some(self)
    }
}

impl<T> fmt::Display for Tensor<T>
where
    T: fmt::Display + Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tensor(shape={:?}, size={})", self.shape, self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_view() {
        let data = vec![1.0f32, 2.0, 3.0, 4.0];
        tensor = TensorView::new(&data, vec![2, 2]);

        assert_eq!(tensor.len(), 4);
        assert_eq!(tensor.ndim(), 2);
        assert_eq!(tensor.get(&[0, 0]), 1.0);
        assert_eq!(tensor.get(&[1, 1]), 4.0);
    }

    #[test]
    fn test_tensor_reshape() {
        let data = vec![1.0f32, 2.0, 3.0, 4.0];
        let tensor = Tensor::new(data, vec![2, 2]);

        let reshaped = tensor.reshape(vec![4]);
        assert!(reshaped.is_some());
        assert_eq!(reshaped.unwrap().shape, vec![4]);
    }
}

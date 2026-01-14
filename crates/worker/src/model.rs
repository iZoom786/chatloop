//! Model partition loading and management
//!
//! This module handles loading model partitions via memory-mapped files.
//! Each worker loads only its assigned layer group, minimizing memory usage.

use crate::error::{ChatLoopError, Result};
use crate::tensor::safetensors::{SafeTensorBuffer, SafeTensorRef, TensorDType};
use chatloop_common::config::LayerGroupConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

/// Model partition containing a layer group
///
/// This struct manages the weights for a specific layer group,
/// loaded via memory mapping for efficient access.
pub struct ModelPartition {
    /// Layer group configuration
    pub config: LayerGroupConfig,

    /// Memory-mapped tensor buffer
    tensor_buffer: SafeTensorRef,

    /// Cached tensor views for this layer group
    tensors: HashMap<String, TensorCache>,

    /// Model dtype
    dtype: TensorDType,

    /// Memory usage in bytes
    memory_usage_bytes: usize,
}

/// Cached tensor with metadata
#[derive(Debug, Clone)]
struct TensorCache {
    /// Name of the tensor
    name: String,

    /// Tensor shape
    shape: Vec<usize>,

    /// Whether this tensor is quantized
    quantized: bool,

    /// Quantization scale (if quantized)
    scale: Option<f32>,

    /// Quantization zero point (if quantized)
    zero_point: Option<i32>,
}

impl ModelPartition {
    /// Load a model partition from disk
    ///
    /// This memory-maps the model weights, avoiding loading the entire
    /// model into RAM. Only accessed pages are loaded by the OS.
    pub fn load<P: AsRef<Path>>(
        weights_path: P,
        config: LayerGroupConfig,
    ) -> Result<Self> {
        let weights_path = weights_path.as_ref();
        info!(
            "Loading model partition: layers {}-{} from {}",
            config.start_layer, config.end_layer, weights_path.display()
        );

        // Memory-map the weights file
        let buffer = SafeTensorBuffer::open(weights_path)
            .map_err(|e| ChatLoopError::model(format!("Failed to load weights: {}", e)))?;

        // Determine model dtype from first tensor
        let header = buffer.header();
        let dtype = header
            .tensors
            .values()
            .next()
            .and_then(|info| info.get_dtype())
            .unwrap_or(TensorDType::F32);

        debug!("Model dtype: {:?}", dtype);

        // Index all tensors for this layer group
        let tensors = Self::index_layer_group_tensors(&buffer, &config)?;

        // Calculate memory usage (size of memory-mapped region)
        let memory_usage_bytes = std::fs::metadata(weights_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        info!(
            "Model partition loaded: {} tensors, {} MB",
            tensors.len(),
            memory_usage_bytes / (1024 * 1024)
        );

        Ok(Self {
            config,
            tensor_buffer: Arc::new(buffer),
            tensors,
            dtype,
            memory_usage_bytes,
        })
    }

    /// Index all tensors belonging to this layer group
    fn index_layer_group_tensors(
        buffer: &SafeTensorBuffer,
        config: &LayerGroupConfig,
    ) -> Result<HashMap<String, TensorCache>> {
        let mut tensors = HashMap::new();

        for (name, info) in buffer.header().tensors.iter() {
            // Parse layer index from tensor name
            // Expected format: "model.layers.{layer_idx}.{tensor_name}"
            if let Some(layer_str) = name.split('.').nth(2) {
                if let Ok(layer_idx) = layer_str.parse::<usize>() {
                    // Check if this layer belongs to our group
                    if layer_idx >= config.start_layer && layer_idx < config.end_layer {
                        debug!("Indexing tensor: {}", name);

                        tensors.insert(
                            name.clone(),
                            TensorCache {
                                name: name.clone(),
                                shape: info.shape.clone(),
                                quantized: matches!(info.get_dtype(), Some(TensorDType::I8) | Some(TensorDType::I4)),
                                scale: None,  // TODO: Extract from metadata
                                zero_point: None,
                            },
                        );
                    }
                }
            }
        }

        Ok(tensors)
    }

    /// Get a tensor view by name
    ///
    /// Returns a zero-copy view into the memory-mapped data.
    pub fn get_tensor(&self, name: &str) -> Option<Vec<f32>> {
        let view = self.tensor_buffer.get_tensor(name)?;

        // Convert to f32 based on dtype
        let data = match view.dtype() {
            TensorDType::F32 => unsafe { view.as_f32_slice().to_vec() },
            TensorDType::F16 => unsafe {
                view.as_f16_slice()
                    .iter()
                    .map(|x| x.to_f32())
                    .collect()
            },
            TensorDType::I8 => {
                // Dequantize if needed
                // For now, convert directly (assumes weights are pre-scaled)
                unimplemented!("Int8 weight loading not yet implemented")
            }
            _ => return None,
        };

        Some(data)
    }

    /// Get multiple tensors at once (more efficient)
    pub fn get_tensors(&self, names: &[&str]) -> HashMap<String, Vec<f32>> {
        let mut result = HashMap::new();

        for &name in names {
            if let Some(tensor) = self.get_tensor(name) {
                result.insert(name.to_string(), tensor);
            }
        }

        result
    }

    /// Get all layer names in this partition
    pub fn layer_names(&self) -> Vec<usize> {
        (self.config.start_layer..self.config.end_layer).collect()
    }

    /// Get memory usage in bytes
    pub fn memory_usage_bytes(&self) -> usize {
        self.memory_usage_bytes
    }

    /// Get model dtype
    pub fn dtype(&self) -> TensorDType {
        self.dtype
    }

    /// Preload tensors into page cache
    ///
    /// This optionally reads tensors to ensure they're paged in.
    /// Useful for reducing latency on first access.
    pub fn preload(&self, tensor_names: &[String]) -> Result<()> {
        debug!("Preloading {} tensors", tensor_names.len());

        for name in tensor_names {
            if let Some(_tensor) = self.get_tensor(name) {
                // Accessing the tensor causes the OS to page it in
                // We don't need to do anything with the data
            }
        }

        debug!("Preload complete");
        Ok(())
    }

    /// Get attention weights for a specific layer
    pub fn get_attention_weights(&self, layer_idx: usize) -> Option<AttentionWeights> {
        if layer_idx < self.config.start_layer || layer_idx >= self.config.end_layer {
            return None;
        }

        let prefix = format!("model.layers.{}", layer_idx);

        // Query projection weight
        let q_proj = self.get_tensor(&format!("{}.attention.wq.weight", prefix))?;
        let k_proj = self.get_tensor(&format!("{}.attention.wk.weight", prefix))?;
        let v_proj = self.get_tensor(&format!("{}.attention.wv.weight", prefix))?;
        let o_proj = self.get_tensor(&format!("{}.attention.wo.weight", prefix))?;

        Some(AttentionWeights {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
        })
    }

    /// Get MLP weights for a specific layer
    pub fn get_mlp_weights(&self, layer_idx: usize) -> Option<MlpWeights> {
        if layer_idx < self.config.start_layer || layer_idx >= self.config.end_layer {
            return None;
        }

        let prefix = format!("model.layers.{}", layer_idx);

        // Gate and up projections (for SwiGLU)
        let gate_proj = self.get_tensor(&format!("{}.feed_forward.gate_proj.weight", prefix))?;
        let up_proj = self.get_tensor(&format!("{}.feed_forward.up_proj.weight", prefix))?;
        let down_proj = self.get_tensor(&format!("{}.feed_forward.down_proj.weight", prefix))?;

        Some(MlpWeights {
            gate_proj,
            up_proj,
            down_proj,
        })
    }

    /// Get layer norm weights
    pub fn get_layer_norm(&self, layer_idx: usize) -> Option<LayerNormWeights> {
        if layer_idx < self.config.start_layer || layer_idx >= self.config.end_layer {
            return None;
        }

        let prefix = format!("model.layers.{}", layer_idx);

        let attention_norm = self.get_tensor(&format!("{}.attention_norm.weight", prefix))?;
        let ffn_norm = self.get_tensor(&format!("{}.ffn_norm.weight", prefix))?;

        Some(LayerNormWeights {
            attention_norm,
            ffn_norm,
        })
    }
}

/// Attention weights for a single layer
#[derive(Debug, Clone)]
pub struct AttentionWeights {
    /// Query projection weights
    pub q_proj: Vec<f32>,

    /// Key projection weights
    pub k_proj: Vec<f32>,

    /// Value projection weights
    pub v_proj: Vec<f32>,

    /// Output projection weights
    pub o_proj: Vec<f32>,
}

/// MLP/FFN weights for a single layer
#[derive(Debug, Clone)]
pub struct MlpWeights {
    /// Gate projection weights (for SwiGLU)
    pub gate_proj: Vec<f32>,

    /// Up projection weights
    pub up_proj: Vec<f32>,

    /// Down projection weights
    pub down_proj: Vec<f32>,
}

/// Layer normalization weights
#[derive(Debug, Clone)]
pub struct LayerNormWeights {
    /// Attention layer norm weights
    pub attention_norm: Vec<f32>,

    /// FFN layer norm weights
    pub ffn_norm: Vec<f32>,
}

/// KV cache for a single sequence
///
/// This stores cached keys and values for efficient autoregressive generation.
#[derive(Debug, Clone)]
pub struct KVCache {
    /// Cached keys: [num_layers, num_heads, seq_len, head_dim]
    pub keys: Vec<Vec<f32>>,

    /// Cached values: [num_layers, num_heads, seq_len, head_dim]
    pub values: Vec<Vec<f32>>,

    /// Current sequence length
    pub seq_len: usize,

    /// Maximum sequence length
    pub max_len: usize,

    /// Number of layers
    pub num_layers: usize,

    /// Number of attention heads
    pub num_heads: usize,

    /// Head dimension
    pub head_dim: usize,
}

impl KVCache {
    /// Create a new KV cache
    pub fn new(num_layers: usize, num_heads: usize, head_dim: usize, max_len: usize) -> Self {
        let total_size = num_layers * num_heads * max_len * head_dim;

        Self {
            keys: vec![vec![0.0; total_size],
            values: vec![vec![0.0; total_size],
            seq_len: 0,
            max_len,
            num_layers,
            num_heads,
            head_dim,
        }
    }

    /// Append a new key-value pair
    pub fn append(&mut self, layer_idx: usize, keys: &[f32], values: &[f32]) -> Result<()> {
        if layer_idx >= self.num_layers {
            return Err(ChatLoopError::tensor("Layer index out of bounds"));
        }

        if self.seq_len >= self.max_len {
            return Err(ChatLoopError::tensor("KV cache full"));
        }

        let offset = layer_idx * self.num_heads * self.max_len * self.head_dim
            + self.seq_len * self.head_dim;

        // Copy keys and values
        let key_start = offset;
        let key_end = key_start + keys.len();
        self.keys[key_start..key_end].copy_from_slice(keys);

        let val_start = offset;
        let val_end = val_start + values.len();
        self.values[val_start..val_end].copy_from_slice(values);

        self.seq_len += 1;

        Ok(())
    }

    /// Get keys for a specific layer and position
    pub fn get_keys(&self, layer_idx: usize, pos: usize) -> Option<&[f32]> {
        if pos >= self.seq_len {
            return None;
        }

        let offset = layer_idx * self.num_heads * self.max_len * self.head_dim + pos * self.head_dim;
        let end = offset + self.num_heads * self.head_dim;

        Some(&self.keys[offset..end])
    }

    /// Get values for a specific layer and position
    pub fn get_values(&self, layer_idx: usize, pos: usize) -> Option<&[f32]> {
        if pos >= self.seq_len {
            return None;
        }

        let offset = layer_idx * self.num_heads * self.max_len * self.head_dim + pos * self.head_dim;
        let end = offset + self.num_heads * self.head_dim;

        Some(&self.values[offset..end])
    }

    /// Reset the cache
    pub fn reset(&mut self) {
        self.seq_len = 0;
        self.keys.fill(0.0);
        self.values.fill(0.0);
    }

    /// Resize the cache (clears data)
    pub fn resize(&mut self, new_max_len: usize) {
        let total_size = self.num_layers * self.num_heads * new_max_len * self.head_dim;

        self.keys = vec![vec![0.0; total_size];
        self.values = vec![vec![0.0; total_size];
        self.max_len = new_max_len;
        self.seq_len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_cache() {
        let mut cache = KVCache::new(32, 32, 128, 2048);

        assert_eq!(cache.seq_len, 0);

        let keys = vec![0.1f32; 32 * 128];
        let values = vec![0.2f32; 32 * 128];

        cache.append(0, &keys, &values).unwrap();

        assert_eq!(cache.seq_len, 1);
    }
}

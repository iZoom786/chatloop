//! Configuration structures for ChatLoop
//!
//! This module defines all configuration types used across workers and coordinators.
//! Configurations are loaded from YAML files and can be overridden by environment variables.

use crate::error::{ChatLoopError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Top-level configuration for ChatLoop components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatLoopConfig {
    /// Mode: either "worker" or "coordinator"
    pub mode: String,

    /// Server binding address
    pub bind_address: String,

    /// Server port
    pub port: u16,

    /// Worker-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker: Option<WorkerConfig>,

    /// Coordinator-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinator: Option<CoordinatorConfig>,

    /// Model configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelConfig>,

    /// Performance tuning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceConfig>,

    /// Observability configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observability: Option<ObservabilityConfig>,
}

/// Worker-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Unique worker identifier
    pub worker_id: String,

    /// Layer group this worker is responsible for
    pub layer_group: LayerGroupConfig,

    /// Next worker in pipeline (if any)
    pub next_worker_endpoint: Option<String>,

    /// Previous worker in pipeline (if any)
    pub prev_worker_endpoint: Option<String>,

    /// Batching configuration
    pub batching: BatchingConfig,

    /// Model weights path
    pub weights_path: PathBuf,

    /// Number of worker threads (0 = CPU count)
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,

    /// Enable CPU pinning
    #[serde(default = "default_cpu_pinning")]
    pub enable_cpu_pinning: bool,

    /// CPU cores to pin to (comma-separated list or ranges)
    pub cpu_cores: Option<String>,

    /// NUMA node to allocate memory from (if applicable)
    pub numa_node: Option<u32>,
}

/// Layer group configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerGroupConfig {
    /// Starting layer index (0-based)
    pub start_layer: usize,

    /// Ending layer index (exclusive)
    pub end_layer: usize,

    /// Total number of layers in model
    pub total_layers: usize,

    /// Number of attention heads
    pub num_heads: usize,

    /// Head dimension
    pub head_dim: usize,

    /// Hidden dimension
    pub hidden_dim: usize,

    /// Intermediate dimension (FFN)
    pub intermediate_dim: usize,
}

/// Batching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    /// Maximum batch size
    pub max_batch_size: usize,

    /// Batching window in milliseconds
    pub batching_window_ms: u64,

    /// Maximum queue size
    pub max_queue_size: usize,

    /// Timeout for queue operations
    pub queue_timeout_ms: u64,
}

/// Coordinator-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// List of worker endpoints
    pub worker_endpoints: Vec<String>,

    /// Worker discovery method
    #[serde(default = "default_discovery_method")]
    pub discovery_method: String,

    /// Health check interval in seconds
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,

    /// Worker failure threshold before marking unhealthy
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,

    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name/identifier
    pub model_id: String,

    /// Model architecture type (e.g., "llama", "gpt2")
    pub architecture: String,

    /// Vocabulary size
    pub vocab_size: usize,

    /// Maximum sequence length
    pub max_sequence_length: usize,

    /// Quantization type
    #[serde(default = "default_quantization")]
    pub quantization: QuantizationType,

    /// Number of transformer layers
    pub num_layers: usize,

    /// Layer groups for pipeline parallelism
    pub layer_groups: Vec<LayerGroupConfig>,
}

/// Quantization type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuantizationType {
    /// No quantization (FP32/FP16)
    None,

    /// 8-bit integer quantization
    Int8,

    /// 4-bit integer quantization
    Int4,
}

impl Default for QuantizationType {
    fn default() -> Self {
        QuantizationType::None
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable SIMD operations
    #[serde(default = "default_simd")]
    pub enable_simd: bool,

    /// Enable NUMA-aware allocations
    #[serde(default = "default_numa")]
    pub enable_numa: bool,

    /// Cache size for KV cache (in MB, 0 = unlimited)
    #[serde(default = "default_cache_size")]
    pub kv_cache_mb: usize,

    /// Pre-allocate activation memory
    #[serde(default = "default_preallocate")]
    pub preallocate_activations: bool,

    /// Memory allocation strategy
    #[serde(default = "default_allocator")]
    pub allocator: AllocatorType,
}

/// Memory allocator type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AllocatorType {
    /// Standard system allocator
    System,

    /// Arena allocator (faster, no fragmentation)
    Arena,

    /// Pool allocator (pre-allocated pools)
    Pool,
}

impl Default for AllocatorType {
    fn default() -> Self {
        AllocatorType::System
    }
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Enable Prometheus metrics
    #[serde(default = "default_metrics")]
    pub enable_metrics: bool,

    /// Metrics port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// Enable structured logging
    #[serde(default = "default_structured_logging")]
    pub structured_logging: bool,

    /// OpenTelemetry endpoint (optional)
    pub otel_endpoint: Option<String>,
}

/// Default value functions
fn default_worker_threads() -> usize {
    0 // Means use CPU count
}

fn default_cpu_pinning() -> bool {
    true
}

fn default_discovery_method() -> String {
    "static".to_string()
}

fn default_health_check_interval() -> u64 {
    5
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_request_timeout() -> u64 {
    30
}

fn default_quantization() -> QuantizationType {
    QuantizationType::None
}

fn default_simd() -> bool {
    true
}

fn default_numa() -> bool {
    false
}

fn default_cache_size() -> usize {
    512 // 512 MB default
}

fn default_preallocate() -> bool {
    true
}

fn default_allocator() -> AllocatorType {
    AllocatorType::Arena
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_metrics() -> bool {
    true
}

fn default_metrics_port() -> u16 {
    9091
}

fn default_structured_logging() -> bool {
    true
}

impl ChatLoopConfig {
    /// Load configuration from a YAML file
    pub fn from_file<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ChatLoopError::Config(format!("Failed to read config file {}: {}", path.display(), e)))?;

        let config: ChatLoopConfig = serde_yaml::from_str(&content)
            .map_err(|e| ChatLoopError::Config(format!("Failed to parse config file {}: {}", path.display(), e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // This is a simplified version - in production, you'd use env-specific overrides
        Ok(ChatLoopConfig {
            mode: std::env::var("CHATLOOP_MODE").unwrap_or_else(|_| "worker".to_string()),
            bind_address: std::env::var("CHATLOOP_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("CHATLOOP_PORT")
                .unwrap_or_else(|_| "50051".to_string())
                .parse()
                .map_err(|_| ChatLoopError::Config("Invalid port number".to_string()))?,
            worker: None,
            coordinator: None,
            model: None,
            performance: None,
            observability: None,
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        match self.mode.as_str() {
            "worker" => {
                if self.worker.is_none() {
                    return Err(ChatLoopError::config("Worker config required for worker mode"));
                }
            }
            "coordinator" => {
                if self.coordinator.is_none() {
                    return Err(ChatLoopError::config("Coordinator config required for coordinator mode"));
                }
            }
            _ => {
                return Err(ChatLoopError::config(format!("Invalid mode: {}", self.mode)));
            }
        }
        Ok(())
    }

    /// Get batching window as Duration
    pub fn batching_window(&self) -> Result<Duration> {
        let worker = self.worker.as_ref()
            .ok_or_else(|| ChatLoopError::config("Worker config not found"))?;

        Ok(Duration::from_millis(worker.batching.batching_window_ms))
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Result<Duration> {
        let coordinator = self.coordinator.as_ref()
            .ok_or_else(|| ChatLoopError::config("Coordinator config not found"))?;

        Ok(Duration::from_secs(coordinator.request_timeout_secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = ChatLoopConfig {
            mode: "worker".to_string(),
            bind_address: "0.0.0.0".to_string(),
            port: 50051,
            worker: Some(WorkerConfig {
                worker_id: "test-worker".to_string(),
                layer_group: LayerGroupConfig {
                    start_layer: 0,
                    end_layer: 16,
                    total_layers: 32,
                    num_heads: 32,
                    head_dim: 128,
                    hidden_dim: 4096,
                    intermediate_dim: 11008,
                },
                next_worker_endpoint: Some("http://localhost:50052".to_string()),
                prev_worker_endpoint: None,
                batching: BatchingConfig {
                    max_batch_size: 32,
                    batching_window_ms: 5,
                    max_queue_size: 512,
                    queue_timeout_ms: 100,
                },
                weights_path: PathBuf::from("/models/weights"),
                worker_threads: 0,
                enable_cpu_pinning: true,
                cpu_cores: None,
                numa_node: None,
            }),
            coordinator: None,
            model: None,
            performance: None,
            observability: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_missing_worker() {
        let config = ChatLoopConfig {
            mode: "worker".to_string(),
            bind_address: "0.0.0.0".to_string(),
            port: 50051,
            worker: None,
            coordinator: None,
            model: None,
            performance: None,
            observability: None,
        };

        assert!(config.validate().is_err());
    }
}

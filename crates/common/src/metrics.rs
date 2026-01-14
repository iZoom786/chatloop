//! Metrics collection for ChatLoop
//!
//! This module provides Prometheus metrics for observability.
//! All metrics are carefully designed to minimize overhead in the hot path.

use lazy_static::lazy_static;
use prometheus::{
    core::AtomicU64 as U64, core::AtomicF64 as F64,
    Histogram, IntCounter, IntGauge, Registry,
};
use std::sync::Arc;

/// Metrics registry for ChatLoop
#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    pub registry: Arc<Registry>,
    pub inference: InferenceMetrics,
    pub worker: WorkerMetrics,
    pub coordinator: CoordinatorMetrics,
}

/// Inference-related metrics
#[derive(Debug, Clone)]
pub struct InferenceMetrics {
    /// Total number of inference requests
    pub requests_total: IntCounter,

    /// Total number of successful requests
    pub requests_success: IntCounter,

    /// Total number of failed requests
    pub requests_failed: IntCounter,

    /// Request duration histogram
    pub request_duration: Histogram,

    /// Prompt processing duration
    pub prompt_duration: Histogram,

    /// Token generation duration
    pub generation_duration: Histogram,

    /// Tokens generated total
    pub tokens_generated_total: IntCounter,

    /// Tokens per second
    pub tokens_per_second: Histogram,

    /// Current active requests
    pub active_requests: IntGauge,
}

/// Worker-specific metrics
#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    /// Forward pass duration
    pub forward_duration: Histogram,

    /// Queue wait time
    pub queue_time: Histogram,

    /// Current queue depth
    pub queue_depth: IntGauge,

    /// Batch size histogram
    pub batch_size: Histogram,

    /// CPU utilization percentage
    pub cpu_utilization: IntGauge,

    /// Memory usage in bytes
    pub memory_used: IntGauge,

    /// KV cache size in bytes
    pub kv_cache_size: IntGauge,

    /// Active sequences
    pub active_sequences: IntGauge,
}

/// Coordinator-specific metrics
#[derive(Debug, Clone)]
pub struct CoordinatorMetrics {
    /// Requests routed
    pub requests_routed: IntCounter,

    /// Active workers
    pub active_workers: IntGauge,

    /// Unhealthy workers
    pub unhealthy_workers: IntGauge,

    /// Worker response time
    pub worker_response_time: Histogram,

    /// Load balancing decisions
    pub load_balancing_decisions: IntCounter,

    /// Failed requests due to no workers
    pub no_workers_available: IntCounter,
}

lazy_static! {
    /// Global metrics registry instance
    pub static ref METRICS: MetricsRegistry = MetricsRegistry::new();
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        let registry = Arc::new(Registry::new());

        // Inference metrics
        let requests_total = IntCounter::new(
            "inference_requests_total",
            "Total number of inference requests"
        ).unwrap();

        let requests_success = IntCounter::new(
            "inference_requests_success_total",
            "Total number of successful inference requests"
        ).unwrap();

        let requests_failed = IntCounter::new(
            "inference_requests_failed_total",
            "Total number of failed inference requests"
        ).unwrap();

        let request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "inference_request_duration_seconds",
                "Inference request duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        ).unwrap();

        let prompt_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "inference_prompt_duration_seconds",
                "Prompt processing duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5])
        ).unwrap();

        let generation_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "inference_generation_duration_seconds",
                "Token generation duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5])
        ).unwrap();

        let tokens_generated_total = IntCounter::new(
            "inference_tokens_generated_total",
            "Total number of tokens generated"
        ).unwrap();

        let tokens_per_second = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "inference_tokens_per_second",
                "Tokens generated per second"
            ).buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
        ).unwrap();

        let active_requests = IntGauge::new(
            "inference_active_requests",
            "Current number of active inference requests"
        ).unwrap();

        // Worker metrics
        let forward_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "worker_forward_duration_seconds",
                "Worker forward pass duration in seconds"
            ).buckets(vec![0.0001, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1])
        ).unwrap();

        let queue_time = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "worker_queue_time_seconds",
                "Time requests spend in queue before processing"
            ).buckets(vec![0.0001, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025])
        ).unwrap();

        let queue_depth = IntGauge::new(
            "worker_queue_depth",
            "Current depth of worker request queue"
        ).unwrap();

        let batch_size = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "worker_batch_size",
                "Batch size distribution"
            ).buckets(vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0])
        ).unwrap();

        let cpu_utilization = IntGauge::new(
            "worker_cpu_utilization_percent",
            "Worker CPU utilization percentage"
        ).unwrap();

        let memory_used = IntGauge::new(
            "worker_memory_used_bytes",
            "Worker memory usage in bytes"
        ).unwrap();

        let kv_cache_size = IntGauge::new(
            "worker_kv_cache_size_bytes",
            "Worker KV cache size in bytes"
        ).unwrap();

        let active_sequences = IntGauge::new(
            "worker_active_sequences",
            "Current number of active sequences"
        ).unwrap();

        // Coordinator metrics
        let requests_routed = IntCounter::new(
            "coordinator_requests_routed_total",
            "Total number of requests routed"
        ).unwrap();

        let active_workers = IntGauge::new(
            "coordinator_active_workers",
            "Current number of active workers"
        ).unwrap();

        let unhealthy_workers = IntGauge::new(
            "coordinator_unhealthy_workers",
            "Current number of unhealthy workers"
        ).unwrap();

        let worker_response_time = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "coordinator_worker_response_time_seconds",
                "Worker response time"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0])
        ).unwrap();

        let load_balancing_decisions = IntCounter::new(
            "coordinator_load_balancing_decisions_total",
            "Total number of load balancing decisions"
        ).unwrap();

        let no_workers_available = IntCounter::new(
            "coordinator_no_workers_available_total",
            "Total requests rejected due to no workers"
        ).unwrap();

        // Register all metrics
        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(requests_success.clone())).unwrap();
        registry.register(Box::new(requests_failed.clone())).unwrap();
        registry.register(Box::new(request_duration.clone())).unwrap();
        registry.register(Box::new(prompt_duration.clone())).unwrap();
        registry.register(Box::new(generation_duration.clone())).unwrap();
        registry.register(Box::new(tokens_generated_total.clone())).unwrap();
        registry.register(Box::new(tokens_per_second.clone())).unwrap();
        registry.register(Box::new(active_requests.clone())).unwrap();

        registry.register(Box::new(forward_duration.clone())).unwrap();
        registry.register(Box::new(queue_time.clone())).unwrap();
        registry.register(Box::new(queue_depth.clone())).unwrap();
        registry.register(Box::new(batch_size.clone())).unwrap();
        registry.register(Box::new(cpu_utilization.clone())).unwrap();
        registry.register(Box::new(memory_used.clone())).unwrap();
        registry.register(Box::new(kv_cache_size.clone())).unwrap();
        registry.register(Box::new(active_sequences.clone())).unwrap();

        registry.register(Box::new(requests_routed.clone())).unwrap();
        registry.register(Box::new(active_workers.clone())).unwrap();
        registry.register(Box::new(unhealthy_workers.clone())).unwrap();
        registry.register(Box::new(worker_response_time.clone())).unwrap();
        registry.register(Box::new(load_balancing_decisions.clone())).unwrap();
        registry.register(Box::new(no_workers_available.clone())).unwrap();

        let inference = InferenceMetrics {
            requests_total,
            requests_success,
            requests_failed,
            request_duration,
            prompt_duration,
            generation_duration,
            tokens_generated_total,
            tokens_per_second,
            active_requests,
        };

        let worker = WorkerMetrics {
            forward_duration,
            queue_time,
            queue_depth,
            batch_size,
            cpu_utilization,
            memory_used,
            kv_cache_size,
            active_sequences,
        };

        let coordinator = CoordinatorMetrics {
            requests_routed,
            active_workers,
            unhealthy_workers,
            worker_response_time,
            load_balancing_decisions,
            no_workers_available,
        };

        MetricsRegistry {
            registry,
            inference,
            worker,
            coordinator,
        }
    }

    /// Gather all metrics as text
    pub fn gather(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait for measuring latency
pub trait LatencyTimer {
    /// Observe the duration of a closure
    fn observe<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R;
}

impl LatencyTimer for Histogram {
    fn observe<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed().as_secs_f64();
        self.observe(duration);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry() {
        let metrics = MetricsRegistry::new();

        // Record some metrics
        metrics.inference.requests_total.inc();
        metrics.inference.active_requests.inc();
        metrics.worker.queue_depth.set(10);

        // Gather metrics
        let output = metrics.gather();
        assert!(output.contains("inference_requests_total"));
        assert!(output.contains("worker_queue_depth"));
    }
}

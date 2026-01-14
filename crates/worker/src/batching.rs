//! Token-level batching with configurable window
//!
//! This module implements efficient request batching to maximize throughput
//! while maintaining low latency. Uses lock-free queues for minimal overhead.

use crate::error::{ChatLoopError, Result};
use chatloop_common::config::BatchingConfig;
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

/// Request identifier
pub type RequestId = String;

/// Sequence identifier for tracking multi-step generation
pub type SequenceId = u64;

/// A single inference request waiting to be batched
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    /// Unique request identifier
    pub request_id: RequestId,

    /// Sequence identifier
    pub sequence_id: SequenceId,

    /// Input tokens
    pub tokens: Vec<i32>,

    /// Generation parameters
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub max_tokens: u32,

    /// Request arrival time
    pub arrival_time: Instant,

    /// Metadata
    pub metadata: serde_json::Value,
}

/// Batched requests ready for processing
#[derive(Debug)]
pub struct RequestBatch {
    /// Requests in this batch
    pub requests: Vec<InferenceRequest>,

    /// Batch creation time
    pub creation_time: Instant,

    /// Maximum sequence length in batch
    pub max_seq_len: usize,
}

impl RequestBatch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            creation_time: Instant::now(),
            max_seq_len: 0,
        }
    }

    /// Add a request to the batch
    pub fn add(&mut self, request: InferenceRequest) {
        self.max_seq_len = self.max_seq_len.max(request.tokens.len());
        self.requests.push(request);
    }

    /// Get the batch size
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    /// Check if the batch is empty
    pub is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    /// Get the age of the batch (time since creation)
    pub fn age(&self) -> Duration {
        self.creation_time.elapsed()
    }
}

/// Batch scheduler with configurable window
///
/// This scheduler collects requests into batches, waiting up to the configured
/// batching window before dispatching. Implements backpressure when full.
pub struct BatchScheduler {
    /// Configuration
    config: BatchingConfig,

    /// Request queue (lock-free)
    queue: Arc<SegQueue<InferenceRequest>>,

    /// Current queue depth (atomic for metrics)
    queue_depth: Arc<AtomicUsize>,

    /// Shutdown flag
    shutdown: Arc<AtomicBool>,

    /// Notification for new requests
    notify: Arc<Notify>,
}

impl BatchScheduler {
    /// Create a new batch scheduler
    pub fn new(config: BatchingConfig) -> Self {
        Self {
            config,
            queue: Arc::new(SegQueue::clone()),
            queue_depth: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Submit a request to the scheduler
    ///
    /// Returns error if the queue is full (backpressure).
    pub fn submit(&self, request: InferenceRequest) -> Result<()> {
        // Check queue depth
        if self.queue_depth.load(Ordering::Relaxed) >= self.config.max_queue_size {
            return Err(ChatLoopError::queue_full(
                "Request queue is full, rejecting new request",
            ));
        }

        self.queue.push(request);
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
        self.notify.notify_one();

        trace!(
            "Request submitted, queue depth: {}",
            self.queue_depth.load(Ordering::Relaxed)
        );

        Ok(())
    }

    /// Get the next batch of requests
    ///
    /// This waits for up to the batching window to collect requests.
    /// Returns immediately if max_batch_size is reached.
    pub async fn next_batch(&self) -> Result<Option<RequestBatch>> {
        let batching_window = Duration::from_millis(self.config.batching_window_ms);
        let mut batch = RequestBatch::new();

        // Wait for first request
        loop {
            // Check shutdown
            if self.shutdown.load(Ordering::Relaxed) {
                return Ok(None);
            }

            // Try to get a request
            if let Some(req) = self.queue.pop() {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                batch.add(req);
                break;
            }

            // Wait for notification
            timeout(batching_window, self.notify.notified())
                .await
                .map_err(|_| ChatLoopError::timeout("Batching window timeout"))?;

            // If still no request after timeout, return empty batch
            if let Some(req) = self.queue.pop() {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                batch.add(req);
                break;
            } else {
                return Ok(None);
            }
        }

        // Collect more requests within batching window
        let start = Instant::now();
        while batch.len() < self.config.max_batch_size && start.elapsed() < batching_window {
            if let Some(req) = self.queue.pop() {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                batch.add(req);
            } else {
                // Wait a bit for more requests
                let remaining = batching_window.saturating_sub(start.elapsed());
                if remaining.is_zero() {
                    break;
                }

                let _ = timeout(remaining, self.notify.notified()).await;
            }
        }

        debug!(
            "Created batch: {} requests, max_seq_len: {}, age: {:?}",
            batch.len(),
            batch.max_seq_len,
            batch.age()
        );

        Ok(Some(batch))
    }

    /// Get the current queue depth
    pub fn queue_depth(&self) -> usize {
        self.queue_depth.load(Ordering::Relaxed)
    }

    /// Check if the queue is healthy (not saturated)
    pub fn is_healthy(&self) -> bool {
        let depth = self.queue_depth.load(Ordering::Relaxed);
        depth < (self.config.max_queue_size * 9 / 10) // 90% threshold
    }

    /// Shutdown the scheduler
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.notify.notify_waiters();
    }
}

/// Priority-based request scheduler
///
/// This implements priority queues for different request classes.
/// Not used by default but can be enabled for multi-tenant scenarios.
pub struct PriorityScheduler {
    /// High-priority queue (e.g., admin requests)
    high_priority: Arc<SegQueue<InferenceRequest>>,

    /// Normal priority queue
    normal_priority: Arc<SegQueue<InferenceRequest>>,

    /// Low-priority queue (e.g., background jobs)
    low_priority: Arc<SegQueue<InferenceRequest>>,

    /// Shutdown flag
    shutdown: Arc<AtomicBool>,

    /// Notification
    notify: Arc<Notify>,
}

impl PriorityScheduler {
    /// Create a new priority scheduler
    pub fn new() -> Self {
        Self {
            high_priority: Arc::new(SegQueue::clone()),
            normal_priority: Arc::new(SegQueue::clone()),
            low_priority: Arc::new(SegQueue::clone()),
            shutdown: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Submit a request with priority
    pub fn submit(&self, request: InferenceRequest, priority: Priority) -> Result<()> {
        let queue = match priority {
            Priority::High => &self.high_priority,
            Priority::Normal => &self.normal_priority,
            Priority::Low => &self.low_priority,
        };

        queue.push(request);
        self.notify.notify_one();

        Ok(())
    }

    /// Get next batch considering priorities
    ///
    /// Always processes high-priority requests first.
    pub async fn next_batch(&self, max_batch_size: usize) -> Result<Option<RequestBatch>> {
        let mut batch = RequestBatch::new();

        // Check shutdown
        if self.shutdown.load(Ordering::Relaxed) {
            return Ok(None);
        }

        // Priority order: high -> normal -> low
        let queues = [&self.high_priority, &self.normal_priority, &self.low_priority];

        for queue in &queues {
            while batch.len() < max_batch_size {
                if let Some(req) = queue.pop() {
                    batch.add(req);
                } else {
                    break;
                }
            }

            if !batch.is_empty() {
                break;
            }
        }

        if batch.is_empty() {
            // Wait for notification
            self.notify.notified().await;
            return self.next_batch(max_batch_size).await;
        }

        Ok(Some(batch))
    }

    /// Shutdown the scheduler
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.notify.notify_waiters();
    }
}

/// Request priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// High priority (e.g., admin, paid users)
    High,

    /// Normal priority
    Normal,

    /// Low priority (e.g., free tier, background jobs)
    Low,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_scheduler() {
        let config = BatchingConfig {
            max_batch_size: 4,
            batching_window_ms: 10,
            max_queue_size: 100,
            queue_timeout_ms: 1000,
        };

        let scheduler = BatchScheduler::new(config);

        // Submit some requests
        for i in 0..3 {
            let request = InferenceRequest {
                request_id: format!("req-{}", i),
                sequence_id: i as u64,
                tokens: vec![1, 2, 3],
                temperature: 1.0,
                top_p: 0.9,
                top_k: 50,
                max_tokens: 100,
                arrival_time: Instant::now(),
                metadata: serde_json::json!({}),
            };

            scheduler.submit(request).unwrap();
        }

        // Get next batch
        let batch = scheduler.next_batch().await.unwrap().unwrap();

        assert_eq!(batch.len(), 3);
    }

    #[tokio::test]
    async fn test_batch_backpressure() {
        let config = BatchingConfig {
            max_batch_size: 4,
            batching_window_ms: 10,
            max_queue_size: 5,
            queue_timeout_ms: 1000,
        };

        let scheduler = BatchScheduler::new(config);

        // Fill the queue
        for i in 0..10 {
            let request = InferenceRequest {
                request_id: format!("req-{}", i),
                sequence_id: i as u64,
                tokens: vec![1, 2, 3],
                temperature: 1.0,
                top_p: 0.9,
                top_k: 50,
                max_tokens: 100,
                arrival_time: Instant::now(),
                metadata: serde_json::json!({}),
            };

            let result = scheduler.submit(request);
            if i < 5 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }

        assert_eq!(scheduler.queue_depth(), 5);
    }
}

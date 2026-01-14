//! Request routing and load balancing
//!
//! This module implements intelligent request routing across worker nodes
//! based on queue depth and health status.

use crate::worker_client::WorkerClient;
use chatloop_common::error::{ChatLoopError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Worker information for routing decisions
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Worker endpoint
    pub endpoint: String,

    /// Worker ID
    pub worker_id: String,

    /// Layer group this worker handles
    pub layer_group: (usize, usize),

    /// Current queue depth
    pub queue_depth: usize,

    /// Health status
    pub healthy: bool,

    /// Last health check time
    pub last_health_check: std::time::Instant,

    /// Number of consecutive failures
    pub failure_count: u32,
}

impl WorkerInfo {
    /// Create new worker info
    pub fn new(endpoint: String, worker_id: String, layer_group: (usize, usize)) -> Self {
        Self {
            endpoint,
            worker_id,
            layer_group,
            queue_depth: 0,
            healthy: true,
            last_health_check: std::time::Instant::now(),
            failure_count: 0,
        }
    }

    /// Calculate load score for routing (lower is better)
    pub fn load_score(&self) -> f64 {
        if !self.healthy {
            return f64::INFINITY;
        }

        // Simple load score based on queue depth
        // Could be enhanced with latency, throughput, etc.
        self.queue_depth as f64
    }

    /// Check if worker needs health check
    pub fn needs_health_check(&self, interval: std::time::Duration) -> bool {
        self.last_health_check.elapsed() > interval
    }
}

/// Router for distributing requests across workers
pub struct Router {
    /// Registered workers
    workers: Arc<RwLock<HashMap<String, WorkerInfo>>>,

    /// Worker clients
    clients: Arc<RwLock<HashMap<String, WorkerClient>>>,

    /// Health check interval
    health_check_interval: std::time::Duration,

    /// Failure threshold before marking unhealthy
    failure_threshold: u32,
}

impl Router {
    /// Create a new router
    pub fn new(health_check_interval_secs: u64, failure_threshold: u32) -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            health_check_interval: std::time::Duration::from_secs(health_check_interval_secs),
            failure_threshold,
        }
    }

    /// Register a worker
    pub async fn register_worker(&self, worker_info: WorkerInfo) -> Result<()> {
        let endpoint = worker_info.endpoint.clone();

        // Create client for this worker
        let client = WorkerClient::connect(&endpoint)
            .await
            .map_err(|e| ChatLoopError::Connection(format!("Failed to connect to {}: {}", endpoint, e)))?;

        // Store worker info and client
        {
            let mut workers = self.workers.write().await;
            workers.insert(endpoint.clone(), worker_info.clone());
        }

        {
            let mut clients = self.clients.write().await;
            clients.insert(endpoint.clone(), client);
        }

        info!("Registered worker: {} at {}", worker_info.worker_id, endpoint);

        Ok(())
    }

    /// Unregister a worker
    pub async fn unregister_worker(&self, endpoint: &str) -> Result<()> {
        {
            let mut workers = self.workers.write().await;
            workers.remove(endpoint);
        }

        {
            let mut clients = self.clients.write().await;
            clients.remove(endpoint);
        }

        info!("Unregistered worker: {}", endpoint);

        Ok(())
    }

    /// Select the best worker for a request
    ///
    /// Uses least-loaded routing based on queue depth.
    pub async fn select_worker(&self) -> Result<String> {
        let workers = self.workers.read().await;

        if workers.is_empty() {
            return Err(ChatLoopError::worker_unavailable("No workers available"));
        }

        // Find worker with lowest load score
        let best_worker = workers
            .values()
            .filter(|w| w.healthy)
            .min_by(|a, b| {
                a.load_score()
                    .partial_cmp(&b.load_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        match best_worker {
            Some(worker) => {
                debug!(
                    "Selected worker {} with queue depth {}",
                    worker.worker_id, worker.queue_depth
                );
                Ok(worker.endpoint.clone())
            }
            None => Err(ChatLoopError::worker_unavailable(
                "No healthy workers available",
            )),
        }
    }

    /// Get a worker client by endpoint
    pub async fn get_client(&self, endpoint: &str) -> Option<WorkerClient> {
        let clients = self.clients.read().await;
        clients.get(endpoint).cloned()
    }

    /// Update worker queue depth
    pub async fn update_queue_depth(&self, endpoint: &str, depth: usize) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(endpoint) {
            worker.queue_depth = depth;
            debug!("Updated worker {} queue depth to {}", endpoint, depth);
        }
    }

    /// Mark worker as failed
    pub async fn mark_failed(&self, endpoint: &str) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(endpoint) {
            worker.failure_count += 1;

            if worker.failure_count >= self.failure_threshold {
                worker.healthy = false;
                warn!(
                    "Worker {} marked as unhealthy after {} failures",
                    endpoint, worker.failure_count
                );
            }
        }
    }

    /// Mark worker as healthy
    pub async fn mark_healthy(&self, endpoint: &str) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(endpoint) {
            worker.healthy = true;
            worker.failure_count = 0;
            worker.last_health_check = std::time::Instant::now();
            debug!("Worker {} marked as healthy", endpoint);
        }
    }

    /// Get all worker endpoints
    pub async fn get_worker_endpoints(&self) -> Vec<String> {
        let workers = self.workers.read().await;
        workers.keys().cloned().collect()
    }

    /// Get number of healthy workers
    pub async fn healthy_worker_count(&self) -> usize {
        let workers = self.workers.read().await;
        workers.values().filter(|w| w.healthy).count()
    }

    /// Start background health check task
    pub fn start_health_checks(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.health_check_interval);

            loop {
                interval.tick().await;

                let endpoints = self.get_worker_endpoints().await;

                for endpoint in endpoints {
                    // Perform health check
                    match self.perform_health_check(&endpoint).await {
                        Ok(healthy) => {
                            if healthy {
                                self.mark_healthy(&endpoint).await;
                            } else {
                                self.mark_failed(&endpoint).await;
                            }
                        }
                        Err(e) => {
                            warn!("Health check failed for {}: {}", endpoint, e);
                            self.mark_failed(&endpoint).await;
                        }
                    }
                }
            }
        })
    }

    /// Perform health check on a worker
    async fn perform_health_check(&self, endpoint: &str) -> Result<bool> {
        let client = self
            .get_client(endpoint)
            .await
            .ok_or_else(|| ChatLoopError::Connection("No client for endpoint".to_string()))?;

        client.health_check().await.map_err(|e| {
            ChatLoopError::Connection(format!("Health check failed: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_worker_selection() {
        let router = Router::new(5, 3);

        // Register workers
        router
            .register_worker(WorkerInfo::new(
                "http://localhost:50051".to_string(),
                "worker-1".to_string(),
                (0, 16),
            ))
            .await
            .unwrap();

        router
            .register_worker(WorkerInfo::new(
                "http://localhost:50052".to_string(),
                "worker-2".to_string(),
                (16, 32),
            ))
            .await
            .unwrap();

        // Select worker
        let selected = router.select_worker().await.unwrap();

        assert!(selected.contains("localhost:5005"));
    }

    #[tokio::test]
    async fn test_healthy_worker_count() {
        let router = Router::new(5, 3);

        assert_eq!(router.healthy_worker_count().await, 0);

        router
            .register_worker(WorkerInfo::new(
                "http://localhost:50051".to_string(),
                "worker-1".to_string(),
                (0, 16),
            ))
            .await
            .unwrap();

        assert_eq!(router.healthy_worker_count().await, 1);
    }
}

//! Worker gRPC client for communicating with next worker in pipeline

use chatloop_common::{Result, ChatLoopError};
use chatloop_proto::InferenceRequest;
use tracing::{debug, warn};

/// gRPC client for next worker in pipeline
pub struct WorkerClient {
    endpoint: String,
}

impl WorkerClient {
    /// Create a new worker client
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    /// Forward inference request to next worker
    pub async fn forward(&self, _request: InferenceRequest) -> Result<()> {
        debug!("Forwarding request to next worker at {}", self.endpoint);

        // For now, just return an error indicating not implemented
        // TODO: Implement actual gRPC client
        warn!("gRPC client not yet implemented");
        Err(ChatLoopError::NotImplemented("gRPC client not yet implemented".to_string()))
    }
}

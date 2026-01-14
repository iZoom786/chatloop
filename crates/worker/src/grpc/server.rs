//! Worker gRPC server implementation

use chatloop_common::{Result, ChatLoopError};
use chatloop_proto::InferenceRequest;
use tonic::transport::Server;
use std::net::SocketAddr;
use tracing::{info, error};

/// Worker gRPC server
pub struct WorkerServer {
    bind_address: String,
    port: u16,
}

impl WorkerServer {
    /// Create a new worker server
    pub fn new(bind_address: String, port: u16) -> Self {
        Self {
            bind_address,
            port,
        }
    }

    /// Start the gRPC server
    pub async fn serve(&self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.bind_address, self.port)
            .parse()
            .map_err(|e| ChatLoopError::Configuration(format!("Invalid bind address: {}", e)))?;

        info!("Worker gRPC server listening on {}", addr);

        // For now, just return an error indicating not implemented
        // TODO: Implement actual gRPC service
        Err(ChatLoopError::NotImplemented("gRPC server not yet implemented".to_string()))
    }
}

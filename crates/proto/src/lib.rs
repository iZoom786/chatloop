//! ChatLoop Protocol Buffers
//!
//! This is a simplified placeholder for the gRPC protocol definitions.
//! In production, this would be generated from .proto files using tonic-build.

use serde::{Deserialize, Serialize};

// Inference Service types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model_id: String,
    pub prompt: String,
    pub max_tokens: i32,
    pub temperature: f32,
    pub top_p: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub text: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

// Worker service types
#[derive(Debug, Clone)]
pub struct ForwardRequest {
    pub request_id: String,
    pub sequence_id: u64,
    pub hidden_states: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct ForwardResponse {
    pub request_id: String,
    pub hidden_states: Vec<f32>,
}

// Health check
#[derive(Debug, Clone)]
pub struct HealthCheckRequest {
    pub service: String,
}

#[derive(Debug, Clone)]
pub struct HealthCheckResponse {
    pub serving: bool,
}

pub mod inference {
    pub use super::*;
}

pub mod worker {
    pub use super::*;
}

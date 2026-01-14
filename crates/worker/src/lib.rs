//! ChatLoop Worker
//!
//! Distributed LLM inference worker that processes a partition of model layers.
//! Uses memory-mapped weights and efficient batching for low-latency inference.

pub mod batching;
pub mod inference;
pub mod model;
pub mod tensor;

pub use batching::{BatchScheduler, InferenceRequest, Priority, RequestBatch};
pub use inference::InferenceEngine;
pub use model::{KVCache, ModelPartition};

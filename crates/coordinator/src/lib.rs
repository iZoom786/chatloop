//! ChatLoop Coordinator
//!
//! Stateless coordinator for routing inference requests to worker nodes.

pub mod router;
pub mod worker_client;

pub use router::{Router, WorkerInfo};
pub use worker_client::WorkerClient;

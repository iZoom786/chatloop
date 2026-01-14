//! ChatLoop common library
//!
//! This crate contains shared code used across ChatLoop components.

pub mod config;
pub mod error;
pub mod metrics;

// Re-export commonly used types
pub use error::{ChatLoopError, Result};
pub use metrics::{MetricsRegistry, METRICS};

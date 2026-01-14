//! Common error types for ChatLoop
//!
//! This module defines all error types used across the ChatLoop system.

use std::net::AddrParseError;
use thiserror::Error;

/// Main error type for ChatLoop
#[derive(Error, Debug)]
pub enum ChatLoopError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// gRPC communication errors
    #[error("gRPC error: {0}")]
    Grpc(String),

    /// gRPC transport errors
    #[error("gRPC transport error: {0}")]
    GrpcTransport(String),

    /// Connection errors
    #[error("Connection error: {0}")]
    Connection(String),

    /// Model loading errors
    #[error("Model error: {0}")]
    Model(String),

    /// Tensor operation errors
    #[error("Tensor error: {0}")]
    Tensor(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Memory mapping errors
    #[error("Memory mapping error: {0}")]
    MemoryMap(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Queue full (backpressure)
    #[error("Queue full: {0}")]
    QueueFull(String),

    /// Timeout
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Worker unavailable
    #[error("Worker unavailable: {0}")]
    WorkerUnavailable(String),

    /// System overloaded
    #[error("System overloaded: {0}")]
    Overloaded(String),

    /// NUMA allocation error
    #[error("NUMA allocation error: {0}")]
    Numa(String),

    /// Parsing error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<AddrParseError> for ChatLoopError {
    fn from(err: AddrParseError) -> Self {
        ChatLoopError::Parse(err.to_string())
    }
}

impl ChatLoopError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        ChatLoopError::Config(msg.into())
    }

    /// Create a model error
    pub fn model(msg: impl Into<String>) -> Self {
        ChatLoopError::Model(msg.into())
    }

    /// Create a tensor error
    pub fn tensor(msg: impl Into<String>) -> Self {
        ChatLoopError::Tensor(msg.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        ChatLoopError::InvalidInput(msg.into())
    }

    /// Create a queue full error
    pub fn queue_full(msg: impl Into<String>) -> Self {
        ChatLoopError::QueueFull(msg.into())
    }

    /// Create a timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        ChatLoopError::Timeout(msg.into())
    }

    /// Create a worker unavailable error
    pub fn worker_unavailable(msg: impl Into<String>) -> Self {
        ChatLoopError::WorkerUnavailable(msg.into())
    }

    /// Create an overloaded error
    pub fn overloaded(msg: impl Into<String>) -> Self {
        ChatLoopError::Overloaded(msg.into())
    }
}

/// Result type alias for ChatLoop operations
pub type Result<T> = std::result::Result<T, ChatLoopError>;

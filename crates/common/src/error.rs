//! Common error types for ChatLoop
//!
//! This module defines all error types used across the ChatLoop system.
//! All errors are convertible to gRPC status codes for proper error propagation.

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
    Grpc(#[from] tonic::Status),

    /// gRPC transport errors
    #[error("gRPC transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

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
    /// Convert error to gRPC status code
    pub fn to_status(&self) -> tonic::Status {
        match self {
            ChatLoopError::Grpc(status) => status.clone(),
            ChatLoopError::GrpcTransport(_) => {
                tonic::Status::unavailable("Transport error")
            }
            ChatLoopError::Connection(msg) => {
                tonic::Status::unavailable(format!("Connection error: {}", msg))
            }
            ChatLoopError::Config(msg) => {
                tonic::Status::internal(format!("Configuration error: {}", msg))
            }
            ChatLoopError::Model(msg) => {
                tonic::Status::internal(format!("Model error: {}", msg))
            }
            ChatLoopError::Tensor(msg) => {
                tonic::Status::internal(format!("Tensor operation error: {}", msg))
            }
            ChatLoopError::InvalidInput(msg) => {
                tonic::Status::invalid_argument(format!("Invalid input: {}", msg))
            }
            ChatLoopError::QueueFull(msg) => {
                tonic::Status::resource_exhausted(format!("Queue full: {}", msg))
            }
            ChatLoopError::Timeout(msg) => {
                tonic::Status::deadline_exceeded(format!("Timeout: {}", msg))
            }
            ChatLoopError::WorkerUnavailable(msg) => {
                tonic::Status::unavailable(format!("Worker unavailable: {}", msg))
            }
            ChatLoopError::Overloaded(msg) => {
                tonic::Status::resource_exhausted(format!("System overloaded: {}", msg))
            }
            ChatLoopError::Io(err) => {
                tonic::Status::internal(format!("I/O error: {}", err))
            }
            ChatLoopError::MemoryMap(msg) => {
                tonic::Status::internal(format!("Memory mapping error: {}", msg))
            }
            ChatLoopError::Numa(msg) => {
                tonic::Status::internal(format!("NUMA error: {}", msg))
            }
            ChatLoopError::Parse(msg) => {
                tonic::Status::invalid_argument(format!("Parse error: {}", msg))
            }
            ChatLoopError::Serialization(err) => {
                tonic::Status::internal(format!("Serialization error: {}", err))
            }
            ChatLoopError::Internal(msg) => {
                tonic::Status::internal(format!("Internal error: {}", msg))
            }
        }
    }

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

//! gRPC server and client for worker communication

pub mod server;
pub mod client;

pub use server::WorkerServer;
pub use client::WorkerClient;

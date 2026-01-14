//! ChatLoop Coordinator - Main Entry Point
//!
//! This is the main entry point for the ChatLoop coordinator.
//! It routes inference requests to worker nodes and manages load balancing.

use chatloop_common::{ChatLoopConfig, ChatLoopError, Result};
use chatloop_coordinator::{Router, WorkerInfo};
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chatloop_coordinator=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting ChatLoop Coordinator");

    // Load configuration
    let config_path = std::env::var("CHATLOOP_CONFIG")
        .unwrap_or_else(|_| "configs/coordinator-config.yaml".to_string());

    let config = ChatLoopConfig::from_file(&config_path)?;
    config.validate()?;

    info!(
        "Coordinator configuration loaded: mode={}, bind={}:{}",
        config.mode, config.bind_address, config.port
    );

    // Get coordinator-specific config
    let coordinator_config = config.coordinator.as_ref()
        .ok_or_else(|| ChatLoopError::config("Coordinator config not found"))?;

    // Create router
    let router = Router::new(
        coordinator_config.health_check_interval_secs,
        coordinator_config.failure_threshold,
    );

    // Register initial workers
    for endpoint in &coordinator_config.worker_endpoints {
        let worker_info = WorkerInfo::new(
            endpoint.clone(),
            format!("worker-{}", endpoint),
            (0, 32), // Would be loaded from config in production
        );

        match router.register_worker(worker_info).await {
            Ok(_) => info!("Registered worker: {}", endpoint),
            Err(e) => error!("Failed to register worker {}: {}", endpoint, e),
        }
    }

    // Start health check task
    let router_handle = Arc::new(router);
    let health_check_handle = router_handle.clone().start_health_checks();

    info!("ChatLoop Coordinator running");

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        result = health_check_handle => {
            result?;
        }
    }

    info!("ChatLoop Coordinator shutdown complete");
    Ok(())
}

use std::sync::Arc;

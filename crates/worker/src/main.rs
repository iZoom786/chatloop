//! ChatLoop Worker - Main Entry Point
//!
//! This is the main entry point for the ChatLoop inference worker.
//! It loads a model partition, starts the gRPC server, and processes inference requests.

use chatloop_common::{ChatLoopConfig, ChatLoopError, Result};
use chatloop_worker::{BatchScheduler, InferenceEngine, ModelPartition};
use std::time::Duration;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chatloop_worker=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting ChatLoop Worker");

    // Load configuration
    let config_path = std::env::var("CHATLOOP_CONFIG")
        .unwrap_or_else(|_| "configs/worker-config.yaml".to_string());

    let config = ChatLoopConfig::from_file(&config_path)?;
    config.validate()?;

    info!(
        "Worker configuration loaded: mode={}, bind={}:{}",
        config.mode, config.bind_address, config.port
    );

    // Get worker-specific config
    let worker_config = config.worker.as_ref()
        .ok_or_else(|| ChatLoopError::config("Worker config not found"))?;

    // Load model partition
    info!(
        "Loading model partition: layers {}-{}",
        worker_config.layer_group.start_layer,
        worker_config.layer_group.end_layer
    );

    let model_partition = ModelPartition::load(
        &worker_config.weights_path,
        worker_config.layer_group.clone(),
    )?;

    info!(
        "Model partition loaded: {:.2} GB",
        model_partition.memory_usage_bytes() as f64 / (1024.0 * 1024.0 * 1024.0)
    );

    // Create inference engine
    let inference_engine = InferenceEngine::new(
        model_partition,
        worker_config.layer_group.clone(),
    );

    // Create batch scheduler
    let batch_scheduler = BatchScheduler::new(worker_config.batching.clone());

    // Start worker tasks
    let worker_handle = tokio::spawn(run_worker_loop(
        inference_engine,
        batch_scheduler,
        worker_config.clone(),
    ));

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        result = worker_handle => {
            result??;
        }
    }

    info!("ChatLoop Worker shutdown complete");
    Ok(())
}

/// Main worker processing loop
async fn run_worker_loop(
    mut inference_engine: InferenceEngine,
    batch_scheduler: BatchScheduler,
    worker_config: chatloop_common::config::WorkerConfig,
) -> Result<()> {
    info!("Starting worker processing loop");

    loop {
        // Get next batch of requests
        match batch_scheduler.next_batch().await {
            Ok(Some(batch)) => {
                // Process the batch
                let start = std::time::Instant::now();

                match inference_engine.forward_batch(&batch) {
                    Ok(outputs) => {
                        let duration = start.elapsed();

                        info!(
                            "Processed batch of {} requests in {:?} ({:.2} ms/request)",
                            batch.len(),
                            duration,
                            duration.as_millis() as f64 / batch.len() as f64
                        );

                        // In production, would send outputs to next worker or return to client
                        for (i, output) in outputs.iter().enumerate() {
                            trace!(
                                "Request {} output: {} elements, first={}",
                                i,
                                output.len(),
                                output.first().unwrap_or(&0.0)
                            );
                        }
                    }
                    Err(e) => {
                        error!("Error processing batch: {}", e);
                        // Continue processing other batches
                    }
                }
            }
            Ok(None) => {
                // No batch available (timeout or shutdown)
                trace!("No batch available, continuing");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => {
                if matches!(e, ChatLoopError::Timeout(_)) {
                    trace!("Batch timeout, continuing");
                } else {
                    error!("Error getting batch: {}", e);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
}

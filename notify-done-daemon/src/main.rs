mod config;
mod ebpf_loader;
mod event_processor;
mod notifier;
mod process_tracker;
mod user_session;

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::signal;
use tokio::time::interval;

use config::DaemonConfig;
use ebpf_loader::EbpfLoader;
use event_processor::EventProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    log::info!("notify-done daemon starting");

    // Load configuration
    let config = DaemonConfig::load().context("Failed to load configuration")?;
    log::info!(
        "Configuration: threshold={}s",
        config.threshold_seconds
    );

    // Load and attach eBPF programs
    let mut ebpf = EbpfLoader::load().context("Failed to load eBPF programs")?;
    ebpf.attach().context("Failed to attach eBPF programs")?;

    // Get the events ring buffer
    let mut ring_buf = ebpf.events_ring_buf()?;

    // Create event processor
    let mut processor = EventProcessor::new(config);

    log::info!("notify-done daemon running");

    // Set up cleanup interval (every hour)
    let mut cleanup_interval = interval(Duration::from_secs(3600));

    // Main event loop
    loop {
        tokio::select! {
            // Process events from ring buffer
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if let Err(e) = processor.process_events(&mut ring_buf).await {
                    log::error!("Error processing events: {}", e);
                }
            }

            // Periodic cleanup
            _ = cleanup_interval.tick() => {
                log::debug!("Running periodic cleanup");
                processor.cleanup();
            }

            // Handle shutdown signals
            _ = signal::ctrl_c() => {
                log::info!("Received SIGINT, shutting down");
                break;
            }
        }
    }

    log::info!("notify-done daemon stopped");
    Ok(())
}

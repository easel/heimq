//! heimq: A fast, lightweight, single-node Kafka-compatible API server
//!
//! Focused on transport, simplicity, and speed at the expense of durability.

mod config;
mod consumer_group;
mod error;
mod handler;
mod protocol;
mod server;
mod storage;

use clap::Parser;
use config::Config;
use server::Server;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "heimq=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse configuration
    let config = Config::parse();

    info!(
        "Starting heimq v{} on {}:{}",
        env!("CARGO_PKG_VERSION"),
        config.host,
        config.port
    );

    if config.memory_only {
        info!("Running in memory-only mode (no persistence)");
    } else {
        info!("Data directory: {}", config.data_dir.display());
    }

    // Create and run server
    let server = Server::new(config)?;
    server.run().await?;

    Ok(())
}

//! Raft node implementation - leader election only
//!
//! This is a simplified Raft implementation focusing solely on leader election
//! and term management, without log replication.

mod config;
mod raft;
mod rpc;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    // Initialize logging
    init_logging()?;

    tracing::info!("Starting Raft node");

    // TODO: Load configuration, start Raft node
    tracing::info!("Raft node initialization complete");

    Ok(())
}

/// Initializes the tracing subscriber for structured logging
///
/// Respects the RUST_LOG environment variable for filtering.
/// Default level is INFO.
fn init_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,raft_node=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    Ok(())
}

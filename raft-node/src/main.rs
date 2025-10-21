//! Raft node implementation - leader election only
//!
//! This is a simplified Raft implementation focusing solely on leader election
//! and term management, without log replication.

mod config;
mod http;
mod raft;
mod rpc;
mod threading;

use anyhow::{Context, Result};
use clap::Parser;
use crossbeam_channel::unbounded;
use raft::event_loop::run_event_loop;
use raft::RaftState;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Raft consensus node
#[derive(Parser, Debug)]
#[command(name = "raft-node")]
#[command(about = "Raft consensus node - leader election implementation", long_about = None)]
#[command(version)]
struct Args {
    /// Path to .env file (optional, defaults to .env in current directory)
    #[arg(short, long, value_name = "FILE")]
    env: Option<String>,
}

fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Load .env file if specified
    if let Some(env_file) = args.env {
        dotenvy::from_filename(&env_file)
            .with_context(|| format!("Failed to load env file: {}", env_file))?;
    }

    // Initialize logging
    init_logging()?;

    tracing::info!("Starting Raft node");

    // Load configuration from environment
    let config = config::RaftConfig::from_env()
        .context("Failed to load configuration from environment")?;

    tracing::info!(
        node_id = %config.node_id,
        address = %config.node_address,
        peers = config.peers.len(),
        "Configuration loaded"
    );

    // Create HTTP client transport
    let mut peer_addresses = HashMap::new();
    for peer in &config.peers {
        peer_addresses.insert(
            peer.node_id.clone(),
            format!("http://{}", peer.address),
        );
    }
    let transport = Arc::new(http::client::HttpClient::new(peer_addresses));

    // Create Raft state (one for event loop, one shared with HTTP server)
    let peer_ids: Vec<_> = config.peers.iter().map(|p| p.node_id.clone()).collect();
    let raft_state = RaftState::new(config.node_id.clone(), peer_ids.clone());
    let raft_state_shared = Arc::new(Mutex::new(RaftState::new(config.node_id.clone(), peer_ids)));

    // Create event channel (for sending events to Raft core)
    let (event_tx, event_rx) = unbounded();

    // Create atomic flags for control
    let election_reset = Arc::new(AtomicBool::new(false));
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // Spawn HTTP server thread
    tracing::info!("Starting HTTP server on {}", config.node_address);
    let _server_handle = http::server::spawn_server(
        config.node_address.clone(),
        raft_state_shared.clone(),
        event_tx.clone(),
    );

    // Spawn election timer thread
    tracing::info!(
        timeout_ms = ?config.election_timeout.as_millis(),
        "Starting election timer"
    );
    let _election_timer_handle = threading::timers::spawn_election_timer(
        event_tx.clone(),
        config.election_timeout.as_millis() as u64,
        election_reset.clone(),
        shutdown_flag.clone(),
    );

    // Spawn heartbeat timer thread (always running, but only leaders send heartbeats)
    tracing::info!(
        interval_ms = ?config.heartbeat_interval.as_millis(),
        "Starting heartbeat timer"
    );
    let _heartbeat_timer_handle = threading::timers::spawn_heartbeat_timer(
        event_tx.clone(),
        config.heartbeat_interval.as_millis() as u64,
        shutdown_flag.clone(),
    );

    // Set up Ctrl+C handler for graceful shutdown
    let shutdown_tx = event_tx.clone();
    ctrlc::set_handler(move || {
        tracing::info!("Received shutdown signal (Ctrl+C)");
        let _ = shutdown_tx.send(raft::RaftEvent::Shutdown);
    })
    .context("Failed to set Ctrl+C handler")?;

    tracing::info!("Raft node fully initialized, entering event loop");

    // Run the main event loop (blocks until shutdown)
    run_event_loop(raft_state, event_rx, transport, election_reset);

    // Signal shutdown to timer threads
    shutdown_flag.store(true, std::sync::atomic::Ordering::Relaxed);

    tracing::info!("Raft node stopped");
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

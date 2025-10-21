//! Configuration module for Raft node
//!
//! Loads configuration from environment variables following 12-factor app principles.

use crate::raft::node_id::NodeId;
use anyhow::{Context, Result};
use std::time::Duration;

/// Peer information in the cluster
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    /// The peer's node ID
    pub node_id: NodeId,
    /// The peer's network address (e.g., "127.0.0.1:8001")
    pub address: String,
}

impl PeerInfo {
    /// Creates a new PeerInfo
    #[must_use]
    pub fn new(node_id: NodeId, address: String) -> Self {
        Self { node_id, address }
    }
}

/// Configuration for a Raft node
#[derive(Debug, Clone)]
pub struct RaftConfig {
    /// This node's unique identifier
    pub node_id: NodeId,
    /// This node's network address
    pub node_address: String,
    /// List of cluster peers (excluding this node)
    pub peers: Vec<PeerInfo>,
    /// Election timeout duration (randomized between this and 2x this value)
    pub election_timeout: Duration,
    /// Heartbeat interval (how often leaders send heartbeats)
    pub heartbeat_interval: Duration,
}

impl RaftConfig {
    /// Loads configuration from environment variables
    ///
    /// Required environment variables:
    /// - `NODE_ID`: This node's unique identifier
    /// - `NODE_ADDRESS`: This node's network address (e.g., "127.0.0.1:8001")
    /// - `CLUSTER_PEERS`: Comma-separated list of peer node definitions (format: "nodeX@address")
    ///
    /// Optional environment variables:
    /// - `ELECTION_TIMEOUT_MS`: Election timeout in milliseconds (default: 150)
    /// - `HEARTBEAT_INTERVAL_MS`: Heartbeat interval in milliseconds (default: 50)
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing or invalid
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (doesn't fail if missing)
        let _ = dotenvy::dotenv();

        let node_id = std::env::var("NODE_ID")
            .context("NODE_ID environment variable not set")?
            .into();

        let node_address =
            std::env::var("NODE_ADDRESS").context("NODE_ADDRESS environment variable not set")?;

        let peers_str =
            std::env::var("CLUSTER_PEERS").context("CLUSTER_PEERS environment variable not set")?;

        let peers = Self::parse_peers(&peers_str)?;

        let election_timeout_ms = std::env::var("ELECTION_TIMEOUT_MS")
            .unwrap_or_else(|_| "150".to_string())
            .parse::<u64>()
            .context("ELECTION_TIMEOUT_MS must be a valid number")?;

        let heartbeat_interval_ms = std::env::var("HEARTBEAT_INTERVAL_MS")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<u64>()
            .context("HEARTBEAT_INTERVAL_MS must be a valid number")?;

        Ok(Self {
            node_id,
            node_address,
            peers,
            election_timeout: Duration::from_millis(election_timeout_ms),
            heartbeat_interval: Duration::from_millis(heartbeat_interval_ms),
        })
    }

    /// Parses peer string in format "node1@127.0.0.1:8001,node2@127.0.0.1:8002"
    fn parse_peers(peers_str: &str) -> Result<Vec<PeerInfo>> {
        peers_str
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|peer| {
                let parts: Vec<&str> = peer.trim().split('@').collect();
                if parts.len() != 2 {
                    anyhow::bail!("Invalid peer format: '{}'. Expected 'nodeId@address'", peer);
                }
                Ok(PeerInfo::new(
                    parts[0].trim().into(),
                    parts[1].trim().to_string(),
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_info_new() {
        let peer = PeerInfo::new("node1".into(), "127.0.0.1:8001".to_string());
        assert_eq!(peer.node_id.as_str(), "node1");
        assert_eq!(peer.address, "127.0.0.1:8001");
    }

    #[test]
    fn test_parse_peers_valid() {
        let peers_str = "node1@127.0.0.1:8001,node2@127.0.0.1:8002";
        let peers = RaftConfig::parse_peers(peers_str).expect("Failed to parse peers");

        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].node_id.as_str(), "node1");
        assert_eq!(peers[0].address, "127.0.0.1:8001");
        assert_eq!(peers[1].node_id.as_str(), "node2");
        assert_eq!(peers[1].address, "127.0.0.1:8002");
    }

    #[test]
    fn test_parse_peers_with_spaces() {
        let peers_str = "node1 @ 127.0.0.1:8001 , node2 @ 127.0.0.1:8002";
        let peers = RaftConfig::parse_peers(peers_str).expect("Failed to parse peers");

        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].node_id.as_str(), "node1");
        assert_eq!(peers[0].address, "127.0.0.1:8001");
    }

    #[test]
    fn test_parse_peers_empty() {
        let peers_str = "";
        let peers = RaftConfig::parse_peers(peers_str).expect("Failed to parse peers");
        assert_eq!(peers.len(), 0);
    }

    #[test]
    fn test_parse_peers_invalid_format() {
        let peers_str = "node1:127.0.0.1:8001";
        let result = RaftConfig::parse_peers(peers_str);
        assert!(result.is_err());
    }
}

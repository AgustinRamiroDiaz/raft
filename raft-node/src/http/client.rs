//! HTTP client for sending RPCs to other Raft nodes
//!
//! Uses reqwest in blocking mode to implement the Transport trait.

use crate::raft::node_id::NodeId;
use crate::rpc::messages::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use crate::rpc::transport::Transport;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client implementing the Transport trait
///
/// Uses reqwest in blocking mode to send RPCs to other nodes.
/// Node addresses are looked up from a peer map (NodeId -> address).
#[derive(Debug, Clone)]
pub struct HttpClient {
    /// Map of node IDs to HTTP addresses (e.g., "http://127.0.0.1:8001")
    peer_addresses: HashMap<NodeId, String>,

    /// Underlying HTTP client
    client: reqwest::blocking::Client,
}

impl HttpClient {
    /// Creates a new HTTP client
    ///
    /// # Arguments
    /// * `peer_addresses` - Map of node IDs to base HTTP URLs (e.g., "http://127.0.0.1:8001")
    ///
    /// # Example
    /// ```ignore
    /// let mut peers = HashMap::new();
    /// peers.insert("node1".into(), "http://127.0.0.1:8001".to_string());
    /// peers.insert("node2".into(), "http://127.0.0.1:8002".to_string());
    ///
    /// let client = HttpClient::new(peers);
    /// ```
    pub fn new(peer_addresses: HashMap<NodeId, String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(500)) // 500ms timeout for RPCs
            .build()
            .expect("Failed to build HTTP client");

        Self {
            peer_addresses,
            client,
        }
    }

    /// Looks up the base URL for a peer node
    fn peer_url(&self, target: &NodeId) -> Option<&str> {
        self.peer_addresses.get(target).map(|s| s.as_str())
    }
}

impl Transport for HttpClient {
    fn send_request_vote(
        &self,
        target: &NodeId,
        request: &RequestVoteRequest,
    ) -> Result<RequestVoteResponse> {
        let base_url = self
            .peer_url(target)
            .ok_or_else(|| anyhow::anyhow!("Unknown peer: {}", target))?;

        let url = format!("{}/raft/request_vote", base_url);

        tracing::debug!(target = %target, url = %url, "Sending RequestVote RPC");

        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .with_context(|| format!("Failed to send RequestVote to {}", target))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "RequestVote to {} failed with status {}",
                target,
                response.status()
            );
        }

        let vote_response: RequestVoteResponse = response
            .json()
            .with_context(|| format!("Failed to parse RequestVote response from {}", target))?;

        tracing::debug!(
            target = %target,
            vote_granted = vote_response.vote_granted,
            response_term = %vote_response.term,
            "Received RequestVote response"
        );

        Ok(vote_response)
    }

    fn send_append_entries(
        &self,
        target: &NodeId,
        request: &AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse> {
        let base_url = self
            .peer_url(target)
            .ok_or_else(|| anyhow::anyhow!("Unknown peer: {}", target))?;

        let url = format!("{}/raft/append_entries", base_url);

        tracing::trace!(target = %target, url = %url, "Sending AppendEntries RPC");

        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .with_context(|| format!("Failed to send AppendEntries to {}", target))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "AppendEntries to {} failed with status {}",
                target,
                response.status()
            );
        }

        let append_response: AppendEntriesResponse = response
            .json()
            .with_context(|| format!("Failed to parse AppendEntries response from {}", target))?;

        tracing::trace!(
            target = %target,
            success = append_response.success,
            response_term = %append_response.term,
            "Received AppendEntries response"
        );

        Ok(append_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::term::Term;

    #[test]
    fn test_http_client_creation() {
        let mut peers = HashMap::new();
        peers.insert("node1".into(), "http://127.0.0.1:8001".to_string());
        peers.insert("node2".into(), "http://127.0.0.1:8002".to_string());

        let client = HttpClient::new(peers);

        assert_eq!(
            client.peer_url(&"node1".into()),
            Some("http://127.0.0.1:8001")
        );
        assert_eq!(
            client.peer_url(&"node2".into()),
            Some("http://127.0.0.1:8002")
        );
        assert_eq!(client.peer_url(&"node3".into()), None);
    }

    #[test]
    fn test_unknown_peer_error() {
        let peers = HashMap::new(); // Empty peer map
        let client = HttpClient::new(peers);

        let request = RequestVoteRequest::new(Term::new(1), "candidate".into());
        let result = client.send_request_vote(&"unknown".into(), &request);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown peer"));
    }
}

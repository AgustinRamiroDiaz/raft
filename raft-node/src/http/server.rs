//! HTTP server for receiving RPCs from other Raft nodes
//!
//! Uses Rouille (synchronous HTTP server) to provide REST endpoints for consensus and operations.

use crate::raft::events::RaftEvent;
use crate::raft::node::RaftState;
use crate::rpc::messages::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::{Arc, Mutex};

/// Shared state for HTTP server
///
/// Allows HTTP endpoints to access Raft state and send events to the core.
#[derive(Clone)]
pub struct ServerState {
    /// Shared Raft state (read-only access for status endpoints)
    pub raft_state: Arc<Mutex<RaftState>>,

    /// Channel to send events to Raft core thread
    pub event_sender: Sender<RaftEvent>,
}

/// Node status response for /ops/status endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub state: String, // "Follower", "Candidate", or "Leader"
    pub current_term: u64,
    pub leader_id: Option<String>,
    pub cluster_peers: Vec<String>,
}

/// Health check response for /ops/health endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String, // "healthy"
}

/// Spawns the HTTP server in a dedicated thread using Rouille (synchronous)
///
/// # Arguments
/// * `address` - Address to bind to (e.g., "127.0.0.1:8001")
/// * `raft_state` - Shared Raft state
/// * `event_sender` - Channel to send events to Raft core
///
/// # Returns
/// A join handle for the spawned thread
pub fn spawn_server(
    address: String,
    raft_state: Arc<Mutex<RaftState>>,
    event_sender: Sender<RaftEvent>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        tracing::info!(address = %address, "Starting HTTP server");

        let server_state = ServerState {
            raft_state,
            event_sender,
        };

        // Start the synchronous HTTP server
        rouille::start_server(address, move |request| {
            let state = server_state.clone();

            // Route based on path and method
            match (request.method(), request.url().as_str()) {
                // Consensus endpoints
                ("POST", "/raft/request_vote") => handle_request_vote(request, state),
                ("POST", "/raft/append_entries") => handle_append_entries(request, state),

                // Operations endpoints
                ("GET", "/ops/status") => handle_get_status(state),
                ("GET", "/ops/health") => handle_get_health(),

                // 404 for everything else
                _ => rouille::Response::empty_404(),
            }
        });
    })
}

/// Handle POST /raft/request_vote
fn handle_request_vote(request: &rouille::Request, state: ServerState) -> rouille::Response {
    // Parse JSON request body
    let mut body = String::new();
    if let Some(mut data) = request.data() {
        if let Err(e) = data.read_to_string(&mut body) {
            tracing::error!(error = %e, "Failed to read request body");
            return rouille::Response::text("Failed to read request body").with_status_code(400);
        }
    } else {
        return rouille::Response::text("No request body").with_status_code(400);
    }

    let request_data: RequestVoteRequest = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(error = %e, "Failed to parse JSON");
            return rouille::Response::text(format!("Invalid JSON: {}", e)).with_status_code(400);
        }
    };

    tracing::debug!(
        candidate = %request_data.candidate_id,
        term = %request_data.term,
        "Received RequestVote RPC"
    );

    // Call handler on Raft state
    let mut raft_state = state.raft_state.lock().unwrap();
    let response = raft_state.handle_request_vote(&request_data);

    tracing::debug!(
        vote_granted = response.vote_granted,
        response_term = %response.term,
        "Sending RequestVote response"
    );

    // Return JSON response
    rouille::Response::json(&response)
}

/// Handle POST /raft/append_entries
fn handle_append_entries(request: &rouille::Request, state: ServerState) -> rouille::Response {
    // Parse JSON request body
    let mut body = String::new();
    if let Some(mut data) = request.data() {
        if let Err(e) = data.read_to_string(&mut body) {
            tracing::error!(error = %e, "Failed to read request body");
            return rouille::Response::text("Failed to read request body").with_status_code(400);
        }
    } else {
        return rouille::Response::text("No request body").with_status_code(400);
    }

    let request_data: AppendEntriesRequest = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(error = %e, "Failed to parse JSON");
            return rouille::Response::text(format!("Invalid JSON: {}", e)).with_status_code(400);
        }
    };

    tracing::trace!(
        leader = %request_data.leader_id,
        term = %request_data.term,
        "Received AppendEntries RPC"
    );

    // Call handler on Raft state
    let mut raft_state = state.raft_state.lock().unwrap();
    let response = raft_state.handle_append_entries(&request_data);

    tracing::trace!(
        success = response.success,
        response_term = %response.term,
        "Sending AppendEntries response"
    );

    // Return JSON response
    rouille::Response::json(&response)
}

/// Handle GET /ops/status
fn handle_get_status(state: ServerState) -> rouille::Response {
    let raft_state = state.raft_state.lock().unwrap();

    let status = NodeStatus {
        node_id: raft_state.node_id.to_string(),
        state: format!("{:?}", raft_state.state),
        current_term: raft_state.current_term.value(),
        leader_id: raft_state.leader_id.as_ref().map(|id| id.to_string()),
        cluster_peers: raft_state
            .cluster_peers
            .iter()
            .map(|id| id.to_string())
            .collect(),
    };

    rouille::Response::json(&status)
}

/// Handle GET /ops/health
fn handle_get_health() -> rouille::Response {
    let health = HealthResponse {
        status: "healthy".to_string(),
    };

    rouille::Response::json(&health)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_status_serialization() {
        let status = NodeStatus {
            node_id: "node1".to_string(),
            state: "Leader".to_string(),
            current_term: 5,
            leader_id: Some("node1".to_string()),
            cluster_peers: vec!["node2".to_string(), "node3".to_string()],
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"node_id\":\"node1\""));
        assert!(json.contains("\"state\":\"Leader\""));
        assert!(json.contains("\"current_term\":5"));
    }

    #[test]
    fn test_health_response_serialization() {
        let health = HealthResponse {
            status: "healthy".to_string(),
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
    }
}

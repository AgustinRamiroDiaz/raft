//! Raft core event loop
//!
//! This module provides the main event loop that drives the Raft state machine.
//! The event loop is single-threaded to avoid race conditions on state.

use crate::raft::events::RaftEvent;
use crate::raft::node::RaftState;
use crate::raft::node_id::NodeId;
use crate::raft::state::NodeState;
use crate::rpc::messages::{AppendEntriesRequest, RequestVoteRequest};
use crate::rpc::transport::Transport;
use crossbeam_channel::Receiver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Commands sent from Raft core to the RPC sender thread
#[derive(Debug, Clone)]
pub enum RaftCommand {
    /// Send RequestVote RPC to a specific peer
    SendRequestVote {
        target: NodeId,
        request: RequestVoteRequest,
    },

    /// Send AppendEntries RPC to a specific peer
    SendAppendEntries {
        target: NodeId,
        request: AppendEntriesRequest,
    },
}

/// Runs the main Raft event loop
///
/// This loop processes events from:
/// - Election timer thread (ElectionTimeout)
/// - Heartbeat timer thread (HeartbeatTimeout)
/// - HTTP server thread (RPC requests)
/// - HTTP client thread (RPC responses)
///
/// # Arguments
/// * `raft_state` - The Raft state machine (owned by this thread)
/// * `event_receiver` - Channel receiving events from other threads
/// * `command_sender` - Channel sending RPC commands to HTTP client thread
/// * `election_reset` - Atomic flag to reset election timer
///
/// # Returns
/// Returns when Shutdown event is received
pub fn run_event_loop<T: Transport>(
    mut raft_state: RaftState,
    event_receiver: Receiver<RaftEvent>,
    transport: Arc<T>,
    election_reset: Arc<AtomicBool>,
) {
    tracing::info!(
        node_id = %raft_state.node_id,
        "Raft event loop started"
    );

    loop {
        // Wait for next event
        let event = match event_receiver.recv() {
            Ok(event) => event,
            Err(e) => {
                tracing::error!(error = %e, "Event channel closed, shutting down");
                break;
            }
        };

        tracing::trace!("Processing event: {:?}", event);

        // Process event
        match event {
            RaftEvent::ElectionTimeout => {
                handle_election_timeout(&mut raft_state, transport.as_ref());
            }

            RaftEvent::HeartbeatTimeout => {
                handle_heartbeat_timeout(&mut raft_state, transport.as_ref());
            }

            RaftEvent::RequestVoteReceived {
                from,
                request,
                response_tx,
            } => {
                handle_request_vote_received(&mut raft_state, from, request, response_tx);
            }

            RaftEvent::RequestVoteResponse { from, response } => {
                handle_request_vote_response(&mut raft_state, from, response);
            }

            RaftEvent::AppendEntriesReceived {
                from,
                request,
                response_tx,
            } => {
                handle_append_entries_received(
                    &mut raft_state,
                    from,
                    request,
                    response_tx,
                    &election_reset,
                );
            }

            RaftEvent::AppendEntriesResponse { from, response } => {
                handle_append_entries_response(&mut raft_state, from, response);
            }

            RaftEvent::Shutdown => {
                tracing::info!("Received shutdown signal, stopping event loop");
                break;
            }
        }
    }

    tracing::info!(
        node_id = %raft_state.node_id,
        "Raft event loop stopped"
    );
}

/// Handle election timeout - start new election
fn handle_election_timeout<T: Transport>(raft_state: &mut RaftState, transport: &T) {
    tracing::info!(
        node_id = %raft_state.node_id,
        current_term = %raft_state.current_term,
        "Election timeout - starting election"
    );

    // Start election (increments term, becomes Candidate, votes for self)
    raft_state.start_election();

    // Send RequestVote to all peers
    let request = RequestVoteRequest::new(raft_state.current_term, raft_state.node_id.clone());
    let peers = raft_state.cluster_peers.clone();

    for peer in &peers {
        // Send RPC synchronously
        // Note: In a full implementation, we'd spawn threads or use a thread pool
        // For simplicity, we'll send synchronously here (this is a known limitation)
        match transport.send_request_vote(peer, &request) {
            Ok(response) => {
                // Process response immediately
                tracing::debug!(
                    peer = %peer,
                    vote_granted = response.vote_granted,
                    "Received RequestVote response"
                );

                // Update state based on response
                if raft_state.handle_vote_response(peer, &response) {
                    tracing::info!("Became leader in term {}", raft_state.current_term);
                }
            }
            Err(e) => {
                tracing::warn!(
                    peer = %peer,
                    error = %e,
                    "Failed to send RequestVote"
                );
            }
        }
    }
}

/// Handle heartbeat timeout - send heartbeats to all peers (if Leader)
fn handle_heartbeat_timeout<T: Transport>(raft_state: &mut RaftState, transport: &T) {
    // Only leaders send heartbeats
    if raft_state.state != NodeState::Leader {
        tracing::trace!(
            state = ?raft_state.state,
            "Ignoring heartbeat timeout (not leader)"
        );
        return;
    }

    tracing::trace!(
        node_id = %raft_state.node_id,
        term = %raft_state.current_term,
        "Sending heartbeats to all peers"
    );

    // Send AppendEntries (heartbeat) to all peers
    let request = AppendEntriesRequest::new(raft_state.current_term, raft_state.node_id.clone());
    let peers = raft_state.cluster_peers.clone();

    for peer in &peers {
        // Send RPC synchronously
        match transport.send_append_entries(peer, &request) {
            Ok(response) => {
                tracing::trace!(
                    peer = %peer,
                    success = response.success,
                    "Received AppendEntries response"
                );

                // If peer has higher term, step down
                if response.term > raft_state.current_term {
                    raft_state.step_down(response.term);
                }
            }
            Err(e) => {
                tracing::warn!(
                    peer = %peer,
                    error = %e,
                    "Failed to send AppendEntries"
                );
            }
        }
    }
}

/// Handle incoming RequestVote RPC
fn handle_request_vote_received(
    raft_state: &mut RaftState,
    from: NodeId,
    request: RequestVoteRequest,
    response_tx: std::sync::mpsc::Sender<crate::rpc::messages::RequestVoteResponse>,
) {
    tracing::debug!(
        from = %from,
        candidate = %request.candidate_id,
        term = %request.term,
        "Processing RequestVote"
    );

    // Call handler
    let response = raft_state.handle_request_vote(&request);

    // Send response back to HTTP thread
    if response_tx.send(response).is_err() {
        tracing::error!("Failed to send RequestVote response (receiver dropped)");
    }
}

/// Handle RequestVote response from peer
fn handle_request_vote_response(
    raft_state: &mut RaftState,
    from: NodeId,
    response: crate::rpc::messages::RequestVoteResponse,
) {
    tracing::debug!(
        from = %from,
        vote_granted = response.vote_granted,
        response_term = %response.term,
        "Processing RequestVote response"
    );

    // Handle the vote response (includes term checking and leader transition)
    if raft_state.handle_vote_response(&from, &response) {
        tracing::info!("Became leader in term {}", raft_state.current_term);
    }
}

/// Handle incoming AppendEntries RPC (heartbeat)
fn handle_append_entries_received(
    raft_state: &mut RaftState,
    from: NodeId,
    request: AppendEntriesRequest,
    response_tx: std::sync::mpsc::Sender<crate::rpc::messages::AppendEntriesResponse>,
    election_reset: &Arc<AtomicBool>,
) {
    tracing::trace!(
        from = %from,
        leader = %request.leader_id,
        term = %request.term,
        "Processing AppendEntries"
    );

    // Call handler
    let response = raft_state.handle_append_entries(&request);

    // Reset election timer if successful heartbeat
    if response.success {
        election_reset.store(true, Ordering::Relaxed);
        tracing::trace!("Resetting election timer (received valid heartbeat)");
    }

    // Send response back to HTTP thread
    if response_tx.send(response).is_err() {
        tracing::error!("Failed to send AppendEntries response (receiver dropped)");
    }
}

/// Handle AppendEntries response from peer
fn handle_append_entries_response(
    raft_state: &mut RaftState,
    from: NodeId,
    response: crate::rpc::messages::AppendEntriesResponse,
) {
    tracing::trace!(
        from = %from,
        success = response.success,
        response_term = %response.term,
        "Processing AppendEntries response"
    );

    // If response has higher term, step down
    if response.term > raft_state.current_term {
        raft_state.step_down(response.term);
    }
}

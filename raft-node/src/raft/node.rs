//! Raft node state machine
//!
//! This module contains the core RaftState struct that manages the consensus algorithm.

use crate::raft::{node_id::NodeId, state::NodeState, term::Term};
use std::collections::HashSet;

/// The core Raft state machine
///
/// Manages the persistent and volatile state required for Raft consensus.
#[derive(Debug)]
pub struct RaftState {
    /// Current term number (monotonically increasing)
    pub current_term: Term,

    /// Node ID that received vote in current term (None if no vote cast)
    pub voted_for: Option<NodeId>,

    /// Current state (Follower, Candidate, or Leader)
    pub state: NodeState,

    /// Leader's ID (None if leader unknown)
    pub leader_id: Option<NodeId>,

    /// Set of nodes that granted votes in current term (only used when Candidate)
    pub votes_received: HashSet<NodeId>,

    /// This node's ID
    pub node_id: NodeId,

    /// List of peer node IDs in the cluster
    pub cluster_peers: Vec<NodeId>,
}

impl RaftState {
    /// Creates a new RaftState in the Follower state
    #[must_use]
    pub fn new(node_id: NodeId, cluster_peers: Vec<NodeId>) -> Self {
        Self {
            current_term: Term::initial(),
            voted_for: None,
            state: NodeState::Follower,
            leader_id: None,
            votes_received: HashSet::new(),
            node_id,
            cluster_peers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_raft_state() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let state = RaftState::new(node_id, peers.clone());

        assert_eq!(state.current_term, Term::initial());
        assert_eq!(state.voted_for, None);
        assert_eq!(state.state, NodeState::Follower);
        assert_eq!(state.leader_id, None);
        assert_eq!(state.votes_received.len(), 0);
        assert_eq!(state.node_id.as_str(), "node1");
        assert_eq!(state.cluster_peers.len(), 2);
    }

    // T016: Unit test for term incrementing on election start
    // This test should FAIL until start_election() is implemented
    #[test]
    #[ignore = "Not yet implemented - TDD placeholder"]
    fn test_start_election_increments_term() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        let initial_term = state.current_term;
        // state.start_election();  // Will implement later

        // assert_eq!(state.current_term, initial_term.next());
        // assert_eq!(state.state, NodeState::Candidate);
        // assert_eq!(state.voted_for, Some("node1".into()));
        // assert!(state.votes_received.contains(&"node1".into()));
    }

    // T017: Unit test for vote granting logic
    // This test should FAIL until handle_request_vote() is implemented
    #[test]
    #[ignore = "Not yet implemented - TDD placeholder"]
    fn test_vote_granting_logic() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        // Test 1: Grant vote if haven't voted and term is valid
        // let response = state.handle_request_vote(...);
        // assert!(response.vote_granted);

        // Test 2: Deny vote if already voted for someone else
        // state.voted_for = Some("node2".into());
        // let response = state.handle_request_vote(...);
        // assert!(!response.vote_granted);

        // Test 3: Grant vote and update term if request term is higher
        // (implementation needed)
    }
}

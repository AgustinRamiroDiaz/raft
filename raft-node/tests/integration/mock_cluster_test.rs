//! Integration tests using MockTransport (no network required)
//!
//! These tests validate the Raft state machine logic without requiring
//! actual network communication, making them suitable for sandbox environments.

use raft_node::raft::{NodeState, RaftState, Term};
use raft_node::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
    Transport,
};
use std::sync::Arc;

/// Mock transport that simulates successful vote granting
struct GrantingMockTransport;

impl Transport for GrantingMockTransport {
    fn send_request_vote(
        &self,
        _target: &raft_node::raft::NodeId,
        request: &RequestVoteRequest,
    ) -> anyhow::Result<RequestVoteResponse> {
        // Always grant votes with matching term
        Ok(RequestVoteResponse {
            term: request.term,
            vote_granted: true,
        })
    }

    fn send_append_entries(
        &self,
        _target: &raft_node::raft::NodeId,
        request: &AppendEntriesRequest,
    ) -> anyhow::Result<AppendEntriesResponse> {
        // Always accept heartbeats with matching term
        Ok(AppendEntriesResponse {
            term: request.term,
            success: true,
        })
    }
}

/// Mock transport that rejects votes (already voted)
struct RejectingMockTransport {
    current_term: Term,
}

impl Transport for RejectingMockTransport {
    fn send_request_vote(
        &self,
        _target: &raft_node::raft::NodeId,
        _request: &RequestVoteRequest,
    ) -> anyhow::Result<RequestVoteResponse> {
        // Reject votes
        Ok(RequestVoteResponse {
            term: self.current_term,
            vote_granted: false,
        })
    }

    fn send_append_entries(
        &self,
        _target: &raft_node::raft::NodeId,
        request: &AppendEntriesRequest,
    ) -> anyhow::Result<AppendEntriesResponse> {
        Ok(AppendEntriesResponse {
            term: request.term,
            success: true,
        })
    }
}

#[test]
fn test_candidate_becomes_leader_with_majority_votes() {
    // Create a 3-node cluster configuration
    let node_id = "node1".into();
    let peers = vec!["node2".into(), "node3".into()];
    let mut raft = RaftState::new(node_id, peers);

    // Start an election
    raft.start_election();
    assert_eq!(raft.state, NodeState::Candidate);
    assert_eq!(raft.current_term, Term::new(1));

    // Create mock transport that grants votes
    let transport = Arc::new(GrantingMockTransport);

    // Send RequestVote to peers
    let request = RequestVoteRequest {
        term: raft.current_term,
        candidate_id: raft.node_id.clone(),
    };

    // Simulate receiving votes from both peers
    let response1 = transport
        .send_request_vote(&"node2".into(), &request)
        .unwrap();
    raft.handle_vote_response(&"node2".into(), &response1);

    // After receiving one vote (plus self-vote), should have majority (2/3)
    assert_eq!(raft.state, NodeState::Leader, "Should become leader with majority votes");
}

#[test]
fn test_candidate_does_not_become_leader_without_majority() {
    let node_id = "node1".into();
    let peers = vec!["node2".into(), "node3".into()];
    let mut raft = RaftState::new(node_id, peers);

    // Start an election
    raft.start_election();
    assert_eq!(raft.state, NodeState::Candidate);

    // Create mock transport that rejects votes
    let transport = Arc::new(RejectingMockTransport {
        current_term: Term::new(1),
    });

    let request = RequestVoteRequest {
        term: raft.current_term,
        candidate_id: raft.node_id.clone(),
    };

    // Receive rejected vote from first peer
    let response1 = transport
        .send_request_vote(&"node2".into(), &request)
        .unwrap();
    raft.handle_vote_response(&"node2".into(), &response1);

    // Only has self-vote (1/3), not a majority
    assert_eq!(raft.state, NodeState::Candidate, "Should remain candidate without majority");
    assert_eq!(raft.votes_received.len(), 1);
}

#[test]
fn test_follower_grants_vote_once_per_term() {
    let node_id = "node1".into();
    let peers = vec!["node2".into(), "node3".into()];
    let mut raft = RaftState::new(node_id, peers);

    // First vote request from node2
    let request1 = RequestVoteRequest {
        term: Term::new(1),
        candidate_id: "node2".into(),
    };
    let response1 = raft.handle_request_vote(&request1);
    assert!(response1.vote_granted, "Should grant first vote in term");

    // Second vote request from node3 in same term
    let request2 = RequestVoteRequest {
        term: Term::new(1),
        candidate_id: "node3".into(),
    };
    let response2 = raft.handle_request_vote(&request2);
    assert!(!response2.vote_granted, "Should not grant second vote in same term");
}

#[test]
fn test_higher_term_causes_step_down() {
    let node_id = "node1".into();
    let peers = vec!["node2".into(), "node3".into()];
    let mut raft = RaftState::new(node_id, peers);

    // Become candidate in term 1
    raft.start_election();
    assert_eq!(raft.current_term, Term::new(1));
    assert_eq!(raft.state, NodeState::Candidate);

    // Receive vote request with higher term
    let request = RequestVoteRequest {
        term: Term::new(5),
        candidate_id: "node2".into(),
    };
    let response = raft.handle_request_vote(&request);

    // Should step down and grant vote
    assert_eq!(raft.current_term, Term::new(5), "Should update to higher term");
    assert_eq!(raft.state, NodeState::Follower, "Should step down to follower");
    assert!(response.vote_granted, "Should grant vote in new term");
}

#[test]
fn test_leader_heartbeat_resets_election() {
    let node_id = "node1".into();
    let peers = vec!["node2".into(), "node3".into()];
    let mut raft = RaftState::new(node_id, peers);

    // Start in follower state
    assert_eq!(raft.state, NodeState::Follower);

    // Receive heartbeat from leader
    let heartbeat = AppendEntriesRequest {
        term: Term::new(1),
        leader_id: "node2".into(),
    };
    let response = raft.handle_append_entries(&heartbeat);

    // Should accept heartbeat and update term
    assert!(response.success, "Should accept heartbeat");
    assert_eq!(raft.current_term, Term::new(1));
    assert_eq!(raft.leader_id, Some("node2".into()));
    assert_eq!(raft.state, NodeState::Follower);
}

#[test]
fn test_five_node_cluster_requires_three_votes() {
    // Test majority calculation with 5 nodes
    let node_id = "node1".into();
    let peers = vec![
        "node2".into(),
        "node3".into(),
        "node4".into(),
        "node5".into(),
    ];
    let mut raft = RaftState::new(node_id, peers);

    raft.start_election();
    assert_eq!(raft.state, NodeState::Candidate);

    let transport = Arc::new(GrantingMockTransport);
    let request = RequestVoteRequest {
        term: raft.current_term,
        candidate_id: raft.node_id.clone(),
    };

    // Get one vote (total: 2 including self)
    let response1 = transport
        .send_request_vote(&"node2".into(), &request)
        .unwrap();
    raft.handle_vote_response(&"node2".into(), &response1);
    assert_eq!(raft.state, NodeState::Candidate, "2/5 is not majority");

    // Get second vote (total: 3 including self)
    let response2 = transport
        .send_request_vote(&"node3".into(), &request)
        .unwrap();
    raft.handle_vote_response(&"node3".into(), &response2);
    assert_eq!(raft.state, NodeState::Leader, "3/5 is majority");
}

//! Contract tests for RPC message schemas
//!
//! These tests verify that the RPC message schemas remain stable
//! and are compatible for network communication between nodes.

use raft_node::raft::{NodeId, Term};
use raft_node::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};

// T019: Contract test for RequestVote RPC schema
#[test]
fn test_request_vote_request_schema() {
    let request = RequestVoteRequest::new(Term::new(5), "candidate1".into());

    // Serialize to JSON
    let json = serde_json::to_string(&request).expect("Failed to serialize RequestVoteRequest");

    // Verify JSON structure contains expected fields
    assert!(json.contains("\"term\""));
    assert!(json.contains("\"candidate_id\""));

    // Deserialize back
    let deserialized: RequestVoteRequest =
        serde_json::from_str(&json).expect("Failed to deserialize RequestVoteRequest");

    assert_eq!(deserialized.term, Term::new(5));
    assert_eq!(deserialized.candidate_id.as_str(), "candidate1");
}

#[test]
fn test_request_vote_response_schema() {
    let response = RequestVoteResponse::new(Term::new(5), true);

    // Serialize to JSON
    let json = serde_json::to_string(&response).expect("Failed to serialize RequestVoteResponse");

    // Verify JSON structure
    assert!(json.contains("\"term\""));
    assert!(json.contains("\"vote_granted\""));

    // Deserialize back
    let deserialized: RequestVoteResponse =
        serde_json::from_str(&json).expect("Failed to deserialize RequestVoteResponse");

    assert_eq!(deserialized.term, Term::new(5));
    assert!(deserialized.vote_granted);
}

#[test]
fn test_request_vote_backwards_compatibility() {
    // Ensure we can deserialize messages from older versions
    // This JSON represents what an older node might send
    let old_format_json = r#"{"term":3,"candidate_id":"node1"}"#;

    let deserialized: RequestVoteRequest =
        serde_json::from_str(old_format_json).expect("Failed to deserialize old format");

    assert_eq!(deserialized.term, Term::new(3));
    assert_eq!(deserialized.candidate_id.as_str(), "node1");
}

// T020: Contract test for AppendEntries RPC schema
#[test]
fn test_append_entries_request_schema() {
    let request = AppendEntriesRequest::new(Term::new(10), "leader1".into());

    // Serialize to JSON
    let json = serde_json::to_string(&request).expect("Failed to serialize AppendEntriesRequest");

    // Verify JSON structure contains expected fields
    assert!(json.contains("\"term\""));
    assert!(json.contains("\"leader_id\""));

    // Deserialize back
    let deserialized: AppendEntriesRequest =
        serde_json::from_str(&json).expect("Failed to deserialize AppendEntriesRequest");

    assert_eq!(deserialized.term, Term::new(10));
    assert_eq!(deserialized.leader_id.as_str(), "leader1");
}

#[test]
fn test_append_entries_response_schema() {
    let response = AppendEntriesResponse::new(Term::new(10), true);

    // Serialize to JSON
    let json =
        serde_json::to_string(&response).expect("Failed to serialize AppendEntriesResponse");

    // Verify JSON structure
    assert!(json.contains("\"term\""));
    assert!(json.contains("\"success\""));

    // Deserialize back
    let deserialized: AppendEntriesResponse =
        serde_json::from_str(&json).expect("Failed to deserialize AppendEntriesResponse");

    assert_eq!(deserialized.term, Term::new(10));
    assert!(deserialized.success);
}

#[test]
fn test_append_entries_backwards_compatibility() {
    // Ensure we can deserialize messages from older versions
    let old_format_json = r#"{"term":7,"leader_id":"leader2"}"#;

    let deserialized: AppendEntriesRequest =
        serde_json::from_str(old_format_json).expect("Failed to deserialize old format");

    assert_eq!(deserialized.term, Term::new(7));
    assert_eq!(deserialized.leader_id.as_str(), "leader2");
}

#[test]
fn test_term_serialization_format() {
    // Verify that Term serializes as a plain number (not an object)
    let request = RequestVoteRequest::new(Term::new(42), "node1".into());
    let json = serde_json::to_string(&request).expect("Failed to serialize");

    // Term should serialize as a number, not as {"term": {...}}
    assert!(json.contains("\"term\":42"));
}

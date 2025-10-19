//! RPC message types for Raft consensus
//!
//! Defines the RequestVote and AppendEntries RPCs as specified in the Raft paper.
//! For this implementation (leader election only), AppendEntries acts as a heartbeat.

use crate::raft::{node_id::NodeId, term::Term};
use serde::{Deserialize, Serialize};

/// RequestVote RPC request
///
/// Invoked by candidates to gather votes during leader election.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    /// Candidate's term
    pub term: Term,
    /// Candidate requesting vote
    pub candidate_id: NodeId,
}

impl RequestVoteRequest {
    /// Creates a new RequestVote request
    #[must_use]
    pub fn new(term: Term, candidate_id: NodeId) -> Self {
        Self { term, candidate_id }
    }
}

/// RequestVote RPC response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// Current term, for candidate to update itself
    pub term: Term,
    /// True if candidate received vote
    pub vote_granted: bool,
}

impl RequestVoteResponse {
    /// Creates a new RequestVote response
    #[must_use]
    pub const fn new(term: Term, vote_granted: bool) -> Self {
        Self { term, vote_granted }
    }

    /// Creates a response granting the vote
    #[must_use]
    pub const fn granted(term: Term) -> Self {
        Self::new(term, true)
    }

    /// Creates a response denying the vote
    #[must_use]
    pub const fn denied(term: Term) -> Self {
        Self::new(term, false)
    }
}

/// AppendEntries RPC request
///
/// In this implementation (leader election only), this acts as a heartbeat
/// to maintain leadership. No log entries are included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: Term,
    /// Leader's ID (so followers can redirect clients)
    pub leader_id: NodeId,
}

impl AppendEntriesRequest {
    /// Creates a new AppendEntries request (heartbeat)
    #[must_use]
    pub fn new(term: Term, leader_id: NodeId) -> Self {
        Self { term, leader_id }
    }
}

/// AppendEntries RPC response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term, for leader to update itself
    pub term: Term,
    /// True if follower accepted the heartbeat
    pub success: bool,
}

impl AppendEntriesResponse {
    /// Creates a new AppendEntries response
    #[must_use]
    pub const fn new(term: Term, success: bool) -> Self {
        Self { term, success }
    }

    /// Creates a successful response
    #[must_use]
    pub const fn success(term: Term) -> Self {
        Self::new(term, true)
    }

    /// Creates a failure response
    #[must_use]
    pub const fn failure(term: Term) -> Self {
        Self::new(term, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_vote_request() {
        let req = RequestVoteRequest::new(Term::new(5), "candidate1".into());
        assert_eq!(req.term, Term::new(5));
        assert_eq!(req.candidate_id.as_str(), "candidate1");
    }

    #[test]
    fn test_request_vote_response_granted() {
        let resp = RequestVoteResponse::granted(Term::new(5));
        assert_eq!(resp.term, Term::new(5));
        assert!(resp.vote_granted);
    }

    #[test]
    fn test_request_vote_response_denied() {
        let resp = RequestVoteResponse::denied(Term::new(5));
        assert_eq!(resp.term, Term::new(5));
        assert!(!resp.vote_granted);
    }

    #[test]
    fn test_append_entries_request() {
        let req = AppendEntriesRequest::new(Term::new(10), "leader1".into());
        assert_eq!(req.term, Term::new(10));
        assert_eq!(req.leader_id.as_str(), "leader1");
    }

    #[test]
    fn test_append_entries_response_success() {
        let resp = AppendEntriesResponse::success(Term::new(10));
        assert_eq!(resp.term, Term::new(10));
        assert!(resp.success);
    }

    #[test]
    fn test_append_entries_response_failure() {
        let resp = AppendEntriesResponse::failure(Term::new(10));
        assert_eq!(resp.term, Term::new(10));
        assert!(!resp.success);
    }

    #[test]
    fn test_serialization_request_vote() {
        let req = RequestVoteRequest::new(Term::new(5), "candidate1".into());
        let json = serde_json::to_string(&req).expect("Failed to serialize");
        let deserialized: RequestVoteRequest =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(req, deserialized);
    }

    #[test]
    fn test_serialization_append_entries() {
        let req = AppendEntriesRequest::new(Term::new(10), "leader1".into());
        let json = serde_json::to_string(&req).expect("Failed to serialize");
        let deserialized: AppendEntriesRequest =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(req, deserialized);
    }
}

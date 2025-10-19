//! Event types for the Raft state machine
//!
//! The Raft core operates as a single-threaded event loop that processes
//! events from timer threads and RPC handlers.

use crate::raft::node_id::NodeId;
use crate::rpc::messages::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};

/// Events that drive the Raft state machine
#[derive(Debug, Clone)]
pub enum RaftEvent {
    /// Election timeout expired - transition to candidate and start election
    ElectionTimeout,

    /// Heartbeat timeout expired - leader should send heartbeats
    HeartbeatTimeout,

    /// Received RequestVote RPC request
    RequestVoteReceived {
        /// The node that sent the request
        from: NodeId,
        /// The request payload
        request: RequestVoteRequest,
        /// Channel to send response back (sender thread will handle HTTP response)
        response_tx: std::sync::mpsc::Sender<RequestVoteResponse>,
    },

    /// Received RequestVote RPC response
    RequestVoteResponse {
        /// The node that sent the response
        from: NodeId,
        /// The response payload
        response: RequestVoteResponse,
    },

    /// Received AppendEntries RPC request (heartbeat)
    AppendEntriesReceived {
        /// The node that sent the request
        from: NodeId,
        /// The request payload
        request: AppendEntriesRequest,
        /// Channel to send response back
        response_tx: std::sync::mpsc::Sender<AppendEntriesResponse>,
    },

    /// Received AppendEntries RPC response
    AppendEntriesResponse {
        /// The node that sent the response
        from: NodeId,
        /// The response payload
        response: AppendEntriesResponse,
    },

    /// Shutdown signal - clean up and exit
    Shutdown,
}

impl RaftEvent {
    /// Returns true if this is a timeout event
    #[must_use]
    pub const fn is_timeout(&self) -> bool {
        matches!(self, Self::ElectionTimeout | Self::HeartbeatTimeout)
    }

    /// Returns true if this is an RPC request that requires a response
    #[must_use]
    pub const fn requires_response(&self) -> bool {
        matches!(
            self,
            Self::RequestVoteReceived { .. } | Self::AppendEntriesReceived { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::term::Term;

    #[test]
    fn test_election_timeout_is_timeout() {
        let event = RaftEvent::ElectionTimeout;
        assert!(event.is_timeout());
        assert!(!event.requires_response());
    }

    #[test]
    fn test_heartbeat_timeout_is_timeout() {
        let event = RaftEvent::HeartbeatTimeout;
        assert!(event.is_timeout());
        assert!(!event.requires_response());
    }

    #[test]
    fn test_request_vote_received_requires_response() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let event = RaftEvent::RequestVoteReceived {
            from: "node1".into(),
            request: RequestVoteRequest::new(Term::new(1), "node1".into()),
            response_tx: tx,
        };
        assert!(!event.is_timeout());
        assert!(event.requires_response());
    }

    #[test]
    fn test_request_vote_response_no_response() {
        let event = RaftEvent::RequestVoteResponse {
            from: "node1".into(),
            response: RequestVoteResponse::granted(Term::new(1)),
        };
        assert!(!event.is_timeout());
        assert!(!event.requires_response());
    }

    #[test]
    fn test_append_entries_received_requires_response() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let event = RaftEvent::AppendEntriesReceived {
            from: "leader1".into(),
            request: AppendEntriesRequest::new(Term::new(5), "leader1".into()),
            response_tx: tx,
        };
        assert!(!event.is_timeout());
        assert!(event.requires_response());
    }

    #[test]
    fn test_shutdown_event() {
        let event = RaftEvent::Shutdown;
        assert!(!event.is_timeout());
        assert!(!event.requires_response());
    }
}

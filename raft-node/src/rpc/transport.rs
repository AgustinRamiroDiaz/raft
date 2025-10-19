//! Transport layer abstraction for Raft RPC communication
//!
//! Defines the Transport trait to enable dependency injection and testing.
//! The HTTP implementation will use reqwest in blocking mode.

use crate::raft::node_id::NodeId;
use crate::rpc::messages::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use anyhow::Result;

/// Transport layer abstraction for sending RPCs to other nodes
///
/// This trait enables dependency injection, making it easy to:
/// - Use different transport implementations (HTTP, in-memory for testing, etc.)
/// - Mock network behavior in tests
/// - Handle network failures gracefully
pub trait Transport: Send + Sync {
    /// Sends a RequestVote RPC to the specified node
    ///
    /// # Errors
    /// Returns an error if the network request fails or times out
    fn send_request_vote(
        &self,
        target: &NodeId,
        request: &RequestVoteRequest,
    ) -> Result<RequestVoteResponse>;

    /// Sends an AppendEntries RPC (heartbeat) to the specified node
    ///
    /// # Errors
    /// Returns an error if the network request fails or times out
    fn send_append_entries(
        &self,
        target: &NodeId,
        request: &AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::term::Term;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Mock transport for testing that records sent messages
    #[derive(Debug, Clone)]
    pub struct MockTransport {
        request_votes: Arc<Mutex<Vec<(NodeId, RequestVoteRequest)>>>,
        append_entries: Arc<Mutex<Vec<(NodeId, AppendEntriesRequest)>>>,
        vote_response: RequestVoteResponse,
        append_response: AppendEntriesResponse,
    }

    impl MockTransport {
        #[must_use]
        pub fn new(
            vote_response: RequestVoteResponse,
            append_response: AppendEntriesResponse,
        ) -> Self {
            Self {
                request_votes: Arc::new(Mutex::new(Vec::new())),
                append_entries: Arc::new(Mutex::new(Vec::new())),
                vote_response,
                append_response,
            }
        }

        #[must_use]
        pub fn request_votes(&self) -> Vec<(NodeId, RequestVoteRequest)> {
            self.request_votes.lock().unwrap().clone()
        }

        #[must_use]
        pub fn append_entries(&self) -> Vec<(NodeId, AppendEntriesRequest)> {
            self.append_entries.lock().unwrap().clone()
        }
    }

    impl Transport for MockTransport {
        fn send_request_vote(
            &self,
            target: &NodeId,
            request: &RequestVoteRequest,
        ) -> Result<RequestVoteResponse> {
            self.request_votes
                .lock()
                .unwrap()
                .push((target.clone(), request.clone()));
            Ok(self.vote_response.clone())
        }

        fn send_append_entries(
            &self,
            target: &NodeId,
            request: &AppendEntriesRequest,
        ) -> Result<AppendEntriesResponse> {
            self.append_entries
                .lock()
                .unwrap()
                .push((target.clone(), request.clone()));
            Ok(self.append_response.clone())
        }
    }

    #[test]
    fn test_mock_transport_request_vote() {
        let transport = MockTransport::new(
            RequestVoteResponse::granted(Term::new(5)),
            AppendEntriesResponse::success(Term::new(5)),
        );

        let request = RequestVoteRequest::new(Term::new(5), "candidate1".into());
        let response = transport
            .send_request_vote(&"node1".into(), &request)
            .expect("Send failed");

        assert!(response.vote_granted);
        assert_eq!(response.term, Term::new(5));

        let sent = transport.request_votes();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0.as_str(), "node1");
    }

    #[test]
    fn test_mock_transport_append_entries() {
        let transport = MockTransport::new(
            RequestVoteResponse::granted(Term::new(5)),
            AppendEntriesResponse::success(Term::new(10)),
        );

        let request = AppendEntriesRequest::new(Term::new(10), "leader1".into());
        let response = transport
            .send_append_entries(&"node1".into(), &request)
            .expect("Send failed");

        assert!(response.success);
        assert_eq!(response.term, Term::new(10));

        let sent = transport.append_entries();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0.as_str(), "node1");
    }
}

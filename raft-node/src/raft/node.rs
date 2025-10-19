//! Raft node state machine
//!
//! This module contains the core RaftState struct that manages the consensus algorithm.

use crate::raft::{election::has_majority, node_id::NodeId, state::NodeState, term::Term};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
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

    /// Starts a new election
    ///
    /// This method:
    /// 1. Increments the current term
    /// 2. Transitions to Candidate state
    /// 3. Votes for self
    /// 4. Resets votes_received and adds self vote
    /// 5. Clears the leader_id
    ///
    /// The caller is responsible for:
    /// - Resetting the election timer
    /// - Sending RequestVote RPCs to all peers
    pub fn start_election(&mut self) {
        // Increment term
        self.current_term = self.current_term.next();

        // Transition to Candidate
        self.state = NodeState::Candidate;

        // Vote for self
        self.voted_for = Some(self.node_id.clone());

        // Reset votes and add self vote
        self.votes_received.clear();
        self.votes_received.insert(self.node_id.clone());

        // Clear leader ID (we don't know who the leader is anymore)
        self.leader_id = None;

        tracing::info!(
            node_id = %self.node_id,
            term = %self.current_term,
            "Started election"
        );
    }

    /// Handles a RequestVote RPC
    ///
    /// Raft voting rules (from the paper):
    /// 1. Reply false if request.term < current_term
    /// 2. If voted_for is None or equals candidate_id, and candidate's log is
    ///    at least as up-to-date as receiver's log, grant vote
    ///
    /// Note: For this leader-election-only implementation, we don't have logs,
    /// so we only check term and voted_for.
    ///
    /// Additionally:
    /// - If request.term > current_term, update current_term and step down to Follower
    pub fn handle_request_vote(&mut self, request: &RequestVoteRequest) -> RequestVoteResponse {
        // Rule 1: Reject if request term is older than our term
        if request.term < self.current_term {
            tracing::debug!(
                node_id = %self.node_id,
                current_term = %self.current_term,
                request_term = %request.term,
                candidate = %request.candidate_id,
                "Rejecting vote - request term too old"
            );
            return RequestVoteResponse::denied(self.current_term);
        }

        // If request term is newer, update our term and step down
        if request.term > self.current_term {
            tracing::info!(
                node_id = %self.node_id,
                old_term = %self.current_term,
                new_term = %request.term,
                "Discovered higher term, stepping down"
            );
            self.step_down(request.term);
        }

        // Rule 2: Grant vote if we haven't voted or already voted for this candidate
        let vote_granted = match &self.voted_for {
            None => {
                // Haven't voted yet in this term, grant vote
                self.voted_for = Some(request.candidate_id.clone());
                tracing::info!(
                    node_id = %self.node_id,
                    term = %self.current_term,
                    candidate = %request.candidate_id,
                    "Granting vote"
                );
                true
            }
            Some(voted_for) if voted_for == &request.candidate_id => {
                // Already voted for this candidate (duplicate request)
                tracing::debug!(
                    node_id = %self.node_id,
                    term = %self.current_term,
                    candidate = %request.candidate_id,
                    "Granting vote (duplicate request)"
                );
                true
            }
            Some(voted_for) => {
                // Already voted for someone else
                tracing::debug!(
                    node_id = %self.node_id,
                    term = %self.current_term,
                    candidate = %request.candidate_id,
                    voted_for = %voted_for,
                    "Denying vote - already voted for another candidate"
                );
                false
            }
        };

        RequestVoteResponse::new(self.current_term, vote_granted)
    }

    /// Steps down to Follower state with a new term
    ///
    /// This is called when:
    /// - We discover a node with a higher term
    /// - We receive an AppendEntries RPC from a valid leader
    ///
    /// Actions:
    /// 1. Update current_term to the new term
    /// 2. Transition to Follower state
    /// 3. Clear voted_for (new term = new vote)
    /// 4. Clear votes_received
    /// 5. Clear leader_id (will be set when we receive AppendEntries)
    pub fn step_down(&mut self, new_term: Term) {
        let old_state = self.state;

        self.current_term = new_term;
        self.state = NodeState::Follower;
        self.voted_for = None;
        self.votes_received.clear();
        self.leader_id = None;

        if old_state != NodeState::Follower {
            tracing::info!(
                node_id = %self.node_id,
                term = %self.current_term,
                old_state = %old_state,
                "Stepped down to Follower"
            );
        }
    }

    /// Handles a RequestVote response from a peer
    ///
    /// If we're a Candidate and receive a vote:
    /// 1. If response.term > current_term, step down
    /// 2. If vote granted, add to votes_received
    /// 3. If we now have majority, become Leader
    ///
    /// Returns true if we became leader, false otherwise
    pub fn handle_vote_response(
        &mut self,
        from: &NodeId,
        response: &RequestVoteResponse,
    ) -> bool {
        // If response has higher term, step down
        if response.term > self.current_term {
            tracing::info!(
                node_id = %self.node_id,
                current_term = %self.current_term,
                response_term = %response.term,
                "Discovered higher term in vote response, stepping down"
            );
            self.step_down(response.term);
            return false;
        }

        // Only process votes if we're a Candidate
        if !self.state.is_candidate() {
            tracing::debug!(
                node_id = %self.node_id,
                state = %self.state,
                "Ignoring vote response - not a candidate"
            );
            return false;
        }

        // Only count votes from the current term
        if response.term != self.current_term {
            tracing::debug!(
                node_id = %self.node_id,
                current_term = %self.current_term,
                response_term = %response.term,
                "Ignoring vote response - term mismatch"
            );
            return false;
        }

        // Record the vote if granted
        if response.vote_granted {
            self.votes_received.insert(from.clone());
            tracing::debug!(
                node_id = %self.node_id,
                term = %self.current_term,
                from = %from,
                votes = self.votes_received.len(),
                total = self.cluster_peers.len() + 1,
                "Received vote"
            );
        }

        // Check if we have majority
        let total_nodes = self.cluster_peers.len() + 1; // +1 for self
        if has_majority(self.votes_received.len(), total_nodes) {
            self.become_leader();
            return true;
        }

        false
    }

    /// Transitions to Leader state
    ///
    /// Called when a Candidate receives a majority of votes.
    fn become_leader(&mut self) {
        tracing::info!(
            node_id = %self.node_id,
            term = %self.current_term,
            votes = self.votes_received.len(),
            "Became leader"
        );

        self.state = NodeState::Leader;
        self.leader_id = Some(self.node_id.clone());

        // Clear votes (no longer needed as leader)
        self.votes_received.clear();

        // Note: The caller is responsible for:
        // - Stopping the election timer
        // - Starting the heartbeat timer
        // - Sending initial heartbeats to all peers
    }

    /// Handles an AppendEntries RPC (heartbeat)
    ///
    /// In this leader-election-only implementation, AppendEntries serves
    /// as a heartbeat to maintain leadership.
    ///
    /// Rules:
    /// 1. Reply false if request.term < current_term
    /// 2. If request.term > current_term, update term and step down
    /// 3. If request.term == current_term, recognize as valid leader
    /// 4. Reset election timeout (caller's responsibility)
    ///
    /// Returns the response to send back
    pub fn handle_append_entries(&mut self, request: &AppendEntriesRequest) -> AppendEntriesResponse {
        // Rule 1: Reject if request term is older
        if request.term < self.current_term {
            tracing::debug!(
                node_id = %self.node_id,
                current_term = %self.current_term,
                request_term = %request.term,
                leader = %request.leader_id,
                "Rejecting heartbeat - term too old"
            );
            return AppendEntriesResponse::failure(self.current_term);
        }

        // If request term is newer or equal, this is a valid leader
        if request.term > self.current_term {
            tracing::info!(
                node_id = %self.node_id,
                old_term = %self.current_term,
                new_term = %request.term,
                "Discovered higher term in heartbeat, stepping down"
            );
            self.step_down(request.term);
        }

        // If we're a candidate or leader with same term, step down to follower
        // (there's already a leader for this term)
        if self.state != NodeState::Follower {
            tracing::info!(
                node_id = %self.node_id,
                term = %self.current_term,
                old_state = %self.state,
                leader = %request.leader_id,
                "Stepping down - valid leader exists"
            );
            self.state = NodeState::Follower;
            self.voted_for = None;
            self.votes_received.clear();
        }

        // Update leader ID
        self.leader_id = Some(request.leader_id.clone());

        tracing::trace!(
            node_id = %self.node_id,
            term = %self.current_term,
            leader = %request.leader_id,
            "Received heartbeat from leader"
        );

        // Note: Caller is responsible for resetting election timeout

        AppendEntriesResponse::success(self.current_term)
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
    #[test]
    fn test_start_election_increments_term() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        let initial_term = state.current_term;
        state.start_election();

        assert_eq!(state.current_term, initial_term.next());
        assert_eq!(state.state, NodeState::Candidate);
        assert_eq!(state.voted_for, Some("node1".into()));
        assert!(state.votes_received.contains(&"node1".into()));
        assert_eq!(state.votes_received.len(), 1);
        assert_eq!(state.leader_id, None);
    }

    // T017: Unit test for vote granting logic
    #[test]
    fn test_vote_granting_logic() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        // Test 1: Grant vote if haven't voted and term is valid
        let request = RequestVoteRequest::new(Term::new(1), "node2".into());
        let response = state.handle_request_vote(&request);
        assert!(response.vote_granted);
        assert_eq!(state.voted_for, Some("node2".into()));

        // Test 2: Deny vote if already voted for someone else
        let request2 = RequestVoteRequest::new(Term::new(1), "node3".into());
        let response2 = state.handle_request_vote(&request2);
        assert!(!response2.vote_granted);
        assert_eq!(state.voted_for, Some("node2".into())); // Still voted for node2

        // Test 3: Grant duplicate vote to same candidate
        let request3 = RequestVoteRequest::new(Term::new(1), "node2".into());
        let response3 = state.handle_request_vote(&request3);
        assert!(response3.vote_granted);

        // Test 4: Reject vote if request term is older
        state.current_term = Term::new(5);
        let old_request = RequestVoteRequest::new(Term::new(3), "node3".into());
        let response4 = state.handle_request_vote(&old_request);
        assert!(!response4.vote_granted);
        assert_eq!(response4.term, Term::new(5));

        // Test 5: Grant vote and update term if request term is higher
        let mut state2 = RaftState::new("node1".into(), vec!["node2".into()]);
        state2.current_term = Term::new(3);
        state2.voted_for = Some("someone".into());

        let higher_term_request = RequestVoteRequest::new(Term::new(5), "node2".into());
        let response5 = state2.handle_request_vote(&higher_term_request);

        assert!(response5.vote_granted);
        assert_eq!(state2.current_term, Term::new(5));
        assert_eq!(state2.voted_for, Some("node2".into()));
        assert_eq!(state2.state, NodeState::Follower);
    }

    #[test]
    fn test_handle_vote_response_becomes_leader_on_majority() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        // Start election (becomes candidate, votes for self)
        state.start_election();
        assert_eq!(state.state, NodeState::Candidate);
        assert_eq!(state.votes_received.len(), 1); // Self vote

        // Receive vote from node2 (2 votes total = majority in 3-node cluster)
        let response = RequestVoteResponse::granted(state.current_term);
        let became_leader = state.handle_vote_response(&"node2".into(), &response);

        assert!(became_leader);
        assert_eq!(state.state, NodeState::Leader);
        assert_eq!(state.leader_id, Some("node1".into()));
        assert_eq!(state.votes_received.len(), 0); // Cleared after becoming leader
    }

    #[test]
    fn test_handle_vote_response_not_yet_majority() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into(), "node4".into(), "node5".into()];
        let mut state = RaftState::new(node_id, peers);

        // Start election (5-node cluster, need 3 votes for majority)
        state.start_election();
        assert_eq!(state.votes_received.len(), 1); // Self vote

        // Receive 1 vote (2 total, not majority yet)
        let response = RequestVoteResponse::granted(state.current_term);
        let became_leader = state.handle_vote_response(&"node2".into(), &response);

        assert!(!became_leader);
        assert_eq!(state.state, NodeState::Candidate); // Still candidate
        assert_eq!(state.votes_received.len(), 2);

        // Receive another vote (3 total = majority)
        let response2 = RequestVoteResponse::granted(state.current_term);
        let became_leader2 = state.handle_vote_response(&"node3".into(), &response2);

        assert!(became_leader2);
        assert_eq!(state.state, NodeState::Leader);
    }

    #[test]
    fn test_handle_vote_response_denied() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        state.start_election();

        // Receive denied vote
        let response = RequestVoteResponse::denied(state.current_term);
        let became_leader = state.handle_vote_response(&"node2".into(), &response);

        assert!(!became_leader);
        assert_eq!(state.votes_received.len(), 1); // Only self vote
    }

    #[test]
    fn test_handle_vote_response_higher_term_steps_down() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        state.start_election();
        assert_eq!(state.current_term, Term::new(1));
        assert_eq!(state.state, NodeState::Candidate);

        // Receive vote response with higher term
        let response = RequestVoteResponse::denied(Term::new(5));
        let became_leader = state.handle_vote_response(&"node2".into(), &response);

        assert!(!became_leader);
        assert_eq!(state.current_term, Term::new(5));
        assert_eq!(state.state, NodeState::Follower); // Stepped down
    }

    #[test]
    fn test_handle_append_entries_follower_accepts() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);
        state.current_term = Term::new(5);

        let request = AppendEntriesRequest::new(Term::new(5), "leader1".into());
        let response = state.handle_append_entries(&request);

        assert!(response.success);
        assert_eq!(response.term, Term::new(5));
        assert_eq!(state.leader_id, Some("leader1".into()));
        assert_eq!(state.state, NodeState::Follower);
    }

    #[test]
    fn test_handle_append_entries_rejects_old_term() {
        let node_id = "node1".into();
        let peers = vec!["node2".into()];
        let mut state = RaftState::new(node_id, peers);
        state.current_term = Term::new(10);

        let request = AppendEntriesRequest::new(Term::new(5), "leader1".into());
        let response = state.handle_append_entries(&request);

        assert!(!response.success);
        assert_eq!(response.term, Term::new(10));
        assert_eq!(state.leader_id, None); // Didn't update leader
    }

    #[test]
    fn test_handle_append_entries_candidate_steps_down() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        // Become candidate
        state.start_election();
        assert_eq!(state.state, NodeState::Candidate);
        assert_eq!(state.current_term, Term::new(1));

        // Receive heartbeat from leader with same term
        let request = AppendEntriesRequest::new(Term::new(1), "node2".into());
        let response = state.handle_append_entries(&request);

        assert!(response.success);
        assert_eq!(state.state, NodeState::Follower); // Stepped down
        assert_eq!(state.leader_id, Some("node2".into()));
        assert_eq!(state.votes_received.len(), 0); // Cleared votes
    }

    #[test]
    fn test_handle_append_entries_leader_steps_down_higher_term() {
        let node_id = "node1".into();
        let peers = vec!["node2".into(), "node3".into()];
        let mut state = RaftState::new(node_id, peers);

        // Become leader
        state.start_election();
        let response = RequestVoteResponse::granted(state.current_term);
        state.handle_vote_response(&"node2".into(), &response);
        assert_eq!(state.state, NodeState::Leader);
        assert_eq!(state.current_term, Term::new(1));

        // Receive heartbeat with higher term
        let request = AppendEntriesRequest::new(Term::new(5), "node3".into());
        let response = state.handle_append_entries(&request);

        assert!(response.success);
        assert_eq!(state.current_term, Term::new(5));
        assert_eq!(state.state, NodeState::Follower); // Stepped down
        assert_eq!(state.leader_id, Some("node3".into()));
    }

    #[test]
    fn test_step_down() {
        let node_id = "node1".into();
        let peers = vec!["node2".into()];
        let mut state = RaftState::new(node_id, peers);

        // Set up as candidate
        state.start_election();
        state.votes_received.insert("node2".into());
        assert_eq!(state.state, NodeState::Candidate);
        assert!(state.voted_for.is_some());

        // Step down
        state.step_down(Term::new(10));

        assert_eq!(state.current_term, Term::new(10));
        assert_eq!(state.state, NodeState::Follower);
        assert_eq!(state.voted_for, None);
        assert_eq!(state.votes_received.len(), 0);
        assert_eq!(state.leader_id, None);
    }
}

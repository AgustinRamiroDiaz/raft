//! Node state types for Raft consensus algorithm

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the three possible states a Raft node can be in.
///
/// In the Raft algorithm, each node is always in one of three states:
/// - **Follower**: The passive state where nodes receive and respond to RPCs
/// - **Candidate**: The transitional state during leader election
/// - **Leader**: The active state where one node manages log replication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeState {
    /// The node is a follower, receiving updates from the leader
    Follower,
    /// The node is a candidate, actively seeking votes to become leader
    Candidate,
    /// The node is the leader, managing the cluster
    Leader,
}

impl NodeState {
    /// Returns true if the node is a follower
    #[must_use]
    pub const fn is_follower(self) -> bool {
        matches!(self, Self::Follower)
    }

    /// Returns true if the node is a candidate
    #[must_use]
    pub const fn is_candidate(self) -> bool {
        matches!(self, Self::Candidate)
    }

    /// Returns true if the node is a leader
    #[must_use]
    pub const fn is_leader(self) -> bool {
        matches!(self, Self::Leader)
    }
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Follower => write!(f, "Follower"),
            Self::Candidate => write!(f, "Candidate"),
            Self::Leader => write!(f, "Leader"),
        }
    }
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Follower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_state_default() {
        let state = NodeState::default();
        assert_eq!(state, NodeState::Follower);
    }

    #[test]
    fn test_is_follower() {
        assert!(NodeState::Follower.is_follower());
        assert!(!NodeState::Candidate.is_follower());
        assert!(!NodeState::Leader.is_follower());
    }

    #[test]
    fn test_is_candidate() {
        assert!(!NodeState::Follower.is_candidate());
        assert!(NodeState::Candidate.is_candidate());
        assert!(!NodeState::Leader.is_candidate());
    }

    #[test]
    fn test_is_leader() {
        assert!(!NodeState::Follower.is_leader());
        assert!(!NodeState::Candidate.is_leader());
        assert!(NodeState::Leader.is_leader());
    }

    #[test]
    fn test_node_state_display() {
        assert_eq!(format!("{}", NodeState::Follower), "Follower");
        assert_eq!(format!("{}", NodeState::Candidate), "Candidate");
        assert_eq!(format!("{}", NodeState::Leader), "Leader");
    }

    #[test]
    fn test_node_state_equality() {
        assert_eq!(NodeState::Follower, NodeState::Follower);
        assert_ne!(NodeState::Follower, NodeState::Candidate);
        assert_ne!(NodeState::Candidate, NodeState::Leader);
    }
}

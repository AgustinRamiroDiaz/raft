//! Leader election logic for Raft

/// Calculates if the number of votes constitutes a majority
///
/// A majority requires more than half of the total nodes (including self).
/// For example, in a 3-node cluster, 2 votes = majority.
/// In a 5-node cluster, 3 votes = majority.
#[must_use]
pub fn has_majority(votes: usize, total_nodes: usize) -> bool {
    votes > total_nodes / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    // T018: Unit test for majority calculation
    #[test]
    fn test_majority_calculation() {
        // 3-node cluster: need 2 votes for majority
        assert!(!has_majority(1, 3)); // 1/3 is not majority
        assert!(has_majority(2, 3)); // 2/3 is majority
        assert!(has_majority(3, 3)); // 3/3 is majority

        // 5-node cluster: need 3 votes for majority
        assert!(!has_majority(1, 5));
        assert!(!has_majority(2, 5));
        assert!(has_majority(3, 5));
        assert!(has_majority(4, 5));
        assert!(has_majority(5, 5));

        // 1-node cluster: need 1 vote (self)
        assert!(has_majority(1, 1));

        // 2-node cluster: need 2 votes (both nodes)
        assert!(!has_majority(1, 2));
        assert!(has_majority(2, 2));

        // 7-node cluster: need 4 votes
        assert!(!has_majority(3, 7));
        assert!(has_majority(4, 7));
    }

    // Test for handle_vote_response - not yet implemented
    #[test]
    #[ignore = "Not yet implemented - TDD placeholder"]
    fn test_handle_vote_response_becomes_leader_on_majority() {
        // When a candidate receives a majority of votes, it should:
        // 1. Transition to Leader state
        // 2. Clear votes_received
        // 3. Initialize leader-specific state (if any)

        // Implementation pending
    }
}

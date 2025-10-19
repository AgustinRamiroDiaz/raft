//! Node identifier type for Raft cluster members

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a unique identifier for a node in a Raft cluster.
///
/// NodeIds are typically human-readable strings like "node1", "node2", etc.
/// They must be unique within the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Creates a new NodeId from a string.
    ///
    /// # Panics
    /// Panics if the string is empty.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        assert!(!id.is_empty(), "NodeId cannot be empty");
        Self(id)
    }

    /// Returns the inner string as a reference
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for NodeId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

impl From<&str> for NodeId {
    fn from(id: &str) -> Self {
        Self::new(id.to_string())
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_new() {
        let node_id = NodeId::new("node1");
        assert_eq!(node_id.as_str(), "node1");
    }

    #[test]
    fn test_node_id_from_string() {
        let node_id: NodeId = "node2".into();
        assert_eq!(node_id.as_str(), "node2");
    }

    #[test]
    fn test_node_id_display() {
        let node_id = NodeId::new("node3");
        assert_eq!(format!("{node_id}"), "node3");
    }

    #[test]
    fn test_node_id_equality() {
        let id1 = NodeId::new("nodeA");
        let id2 = NodeId::new("nodeA");
        let id3 = NodeId::new("nodeB");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    #[should_panic(expected = "NodeId cannot be empty")]
    fn test_node_id_empty_panics() {
        let _node_id = NodeId::new("");
    }

    #[test]
    fn test_node_id_as_ref() {
        let node_id = NodeId::new("node4");
        let s: &str = node_id.as_ref();
        assert_eq!(s, "node4");
    }
}

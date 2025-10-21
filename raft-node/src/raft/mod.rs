//! Core Raft consensus algorithm implementation

pub mod election;
pub mod event_loop;
pub mod events;
pub mod node;
pub mod node_id;
pub mod state;
pub mod term;

// Re-export commonly used types
pub use election::has_majority;
pub use events::RaftEvent;
pub use node::RaftState;
pub use node_id::NodeId;
pub use state::NodeState;
pub use term::Term;

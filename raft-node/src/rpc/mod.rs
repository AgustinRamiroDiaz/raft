//! RPC layer for Raft node communication

pub mod messages;
pub mod transport;

// Re-export commonly used types
pub use messages::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
pub use transport::Transport;

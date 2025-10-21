//! Raft consensus library
//!
//! This library implements the Raft consensus algorithm, focusing on
//! leader election and term management (without log replication).

pub mod config;
pub mod raft;
pub mod rpc;

// Re-export commonly used types for convenience
pub use config::RaftConfig;
pub use raft::{has_majority, NodeId, NodeState, RaftEvent, RaftState, Term};
pub use rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse, Transport,
};

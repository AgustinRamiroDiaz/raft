# Research: Raft Leader Election Implementation (Phase 1)

**Date**: 2025-10-18
**Feature**: 001-raft-consensus-viz
**Scope**: Resolve technical unknowns for Phase 1 implementation

## Overview

This document consolidates research findings for implementing the Raft leader election algorithm using Rust with native threads/channels, Rocket HTTP framework, and Clap CLI. All NEEDS CLARIFICATION items from the Technical Context have been resolved.

---

## Decision 1: Rust Version

**Decision**: Rust 1.90 (stable)

**Rationale**:
- Rust 1.90 (released 2024) is the current stable version with excellent tooling and diagnostics
- All required dependencies (Rocket 0.5.x, Clap 4.5) are fully compatible
- Use stable channel (not nightly) for production reliability
- MSRV (Minimum Supported Rust Version) set to 1.90 in Cargo.toml

**Alternatives Considered**:
- Rust 1.75: Older stable version, missing recent improvements
- Rust nightly: Unnecessary instability; all features needed are in stable

**Implementation**: Set `rust-version = "1.90"` in Cargo.toml

---

## Decision 2: State Storage

**Decision**: In-memory only (no persistence)

**Rationale**:
- Simplifies initial implementation - focus on core consensus algorithm
- Raft state (`current_term`, `voted_for`) kept in memory only
- On restart, nodes start fresh as Followers with term 0
- Sufficient for development and testing of leader election
- Can add persistence later if needed without changing core algorithm

**Alternatives Considered**:
- **File-based persistence (serde_json)**: Adds complexity with atomic writes, error handling
- **Database (SQLite/sled)**: Significant overkill for simple state

**Implementation**:
```rust
// RaftState stored in Arc<Mutex<RaftState>>
// No persistence layer needed
// On restart: RaftState::new() with term = 0, voted_for = None
```

**Note**: This means nodes don't remember votes/terms across restarts, which is acceptable for this implementation.

---

## Decision 3: Rocket Version and Configuration (HTTP Server)

**Decision**: Rocket 0.5.1 (latest stable), running in dedicated thread

**Rationale**:
- Rocket 0.5 is the stable async version BUT we'll use `#[rocket::main]` in a dedicated thread to isolate async runtime
- This satisfies "no async runtime in main application" - Rocket's async is encapsulated in HTTP thread
- User requirement: decouple HTTP from Raft algorithm → achieved via channels between threads
- Rocket's sync features (state management, guards) work well for this pattern

**Architecture**:
```
Main Thread (Raft Core)
  ↓ channels (receive RPC requests)
  ↓ channels (send RPC responses)
HTTP Thread (Rocket Server) ← handles incoming HTTP requests
  ↓ blocking reqwest
HTTP Client Thread Pool ← sends outgoing HTTP requests

Timer Thread (Election Timeout)
  ↓ channel (send ElectionTimeout event to Raft)

Timer Thread (Heartbeat - Leader only)
  ↓ channel (send HeartbeatTimeout event to Raft)
```

**Alternatives Considered**:
- **Actix-web**: Requires Tokio runtime, more async-first than Rocket
- **Warp**: Pure async, doesn't fit sync threading model
- **Tiny-http / Rouille**: Fully synchronous, but less ergonomic than Rocket and lacks ecosystem

**Implementation**: Spawn Rocket server in dedicated thread, communicate with Raft core via `crossbeam::channel` or `std::sync::mpsc`

---

## Decision 4: HTTP Client Library

**Decision**: reqwest 0.12 (blocking mode)

**Rationale**:
- reqwest has a `blocking` feature that provides synchronous HTTP client
- Fits perfectly with Rust threads model (no async/await in client code)
- Widely used, well-maintained, good error handling
- Supports JSON serialization/deserialization via serde integration

**Alternatives Considered**:
- **ureq**: Pure synchronous client. Simpler than reqwest but smaller ecosystem, less battle-tested
- **hyper (blocking)**: Low-level, would need to build request/response handling ourselves
- **curl bindings**: C dependencies, non-idiomatic Rust

**Implementation**:
```rust
use reqwest::blocking::Client;

fn send_request_vote(client: &Client, target: &str, req: RequestVote) -> Result<RequestVoteResponse> {
    client.post(format!("{}/raft/request_vote", target))
        .json(&req)
        .send()?
        .json()
}
```

---

## Decision 5: Thread Communication Pattern

**Decision**: Use `std::sync::mpsc` for Raft → HTTP, `crossbeam::channel` for HTTP → Raft

**Rationale**:
- **Raft → HTTP (commands)**: Single producer (Raft thread), multiple consumers (HTTP might spawn workers). Use `std::sync::mpsc::channel` (MPMC via cloning sender).
- **HTTP → Raft (RPC requests)**: Multiple producers (HTTP threads), single consumer (Raft thread). Use `crossbeam::channel::unbounded()` for MPSC with backpressure.
- crossbeam channels have better performance and ergonomics than std::mpsc for MPSC pattern
- Keep channel types explicit in interfaces for clarity

**Alternatives Considered**:
- **Only std::mpsc**: Works, but MPSC pattern less ergonomic (would need Mutex<Receiver>)
- **Only crossbeam**: Would require adding crossbeam for SPMC, but SPMC not needed
- **Flume**: Another channel library, less widely used than crossbeam

**Implementation**:
```rust
// Raft thread sends to HTTP thread
let (raft_tx, http_rx) = std::sync::mpsc::channel();

// HTTP threads send to Raft thread
let (http_tx, raft_rx) = crossbeam::channel::unbounded();
```

---

## Decision 6: Configuration Management (12-Factor)

**Decision**: Use `dotenvy` for .env file loading + `std::env` for variables

**Rationale**:
- 12-factor apps require environment variable configuration
- `dotenvy` (maintained fork of `dotenv`) loads .env files for local development
- Use `std::env::var()` to read variables at startup
- Fail fast on missing required config (don't use defaults for critical settings like node_id, cluster_peers)

**Configuration Variables**:
```bash
# Required
NODE_ID=node1
NODE_ADDRESS=127.0.0.1:8001
CLUSTER_PEERS=node2@127.0.0.1:8002,node3@127.0.0.1:8003

# Optional with defaults
ELECTION_TIMEOUT_MS=150
HEARTBEAT_INTERVAL_MS=50
STATE_FILE_PATH=./data/raft_state.json
```

**Alternatives Considered**:
- **config-rs**: Supports multiple formats (TOML, YAML, JSON, env). Overkill for env-only config.
- **clap env feature**: Clap can read env vars, but mixing CLI args and env vars is confusing
- **Pure std::env**: No .env file support for local dev, developers would need to set vars manually

**Implementation**: Load .env at startup, parse into `RaftConfig` struct, validate completeness

---

## Decision 7: Testing Strategy

**Decision**: Three-tier testing approach

### Unit Tests (in-module #[cfg(test)])
- Raft state transitions: Follower → Candidate → Leader
- Term incrementing and comparison logic
- Vote counting (majority calculation)
- Mock Transport trait for testing RPC without HTTP

### Integration Tests (tests/integration/)
- Multi-node election scenarios (3-node cluster, 5-node cluster)
- Network partition simulation (drop messages)
- Leader crash and re-election
- Use actual HTTP but localhost ports

### Contract Tests (tests/contract/)
- Validate RequestVote and RequestVoteResponse schema against JSON schema
- Ensure backward compatibility if RPC format changes

**Rationale**:
- Unit tests give fast feedback on core logic
- Integration tests validate distributed behavior (the hard part of Raft)
- Contract tests prevent accidental breaking changes to RPC protocol

**Alternatives Considered**:
- **Property-based testing (proptest)**: Useful for Raft, but deferred to Phase 2 (log replication has more invariants to test)
- **Chaos testing (Jepsen-style)**: Valuable but complex; deferred to later phases

---

## Decision 8: Error Handling Pattern

**Decision**: Use `anyhow` for application errors, `thiserror` for library errors

**Rationale**:
- **anyhow**: Ergonomic error handling in `main.rs` and integration code (dynamic errors with context)
- **thiserror**: Derive Error trait for Raft-specific errors (e.g., `RaftError::InvalidTerm`)
- Clear distinction: library code (raft, rpc) uses thiserror, application code (main, CLI) uses anyhow

**Library Errors (thiserror)**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum RaftError {
    #[error("Invalid term: expected >= {expected}, got {actual}")]
    InvalidTerm { expected: u64, actual: u64 },

    #[error("Already voted in term {term}")]
    AlreadyVoted { term: u64 },
}
```

**Application Errors (anyhow)**:
```rust
fn main() -> anyhow::Result<()> {
    let config = load_config().context("Failed to load configuration")?;
    // ...
}
```

**Alternatives Considered**:
- **Only std::error::Error**: Too much boilerplate, no context chaining
- **Only anyhow**: Not suitable for library code (loses error type information)

---

## Decision 9: Logging

**Decision**: Use `tracing` with `tracing-subscriber`

**Rationale**:
- `tracing` is the modern Rust logging standard (replaces `log` crate)
- Structured logging with spans perfect for distributed systems debugging
- Can correlate logs across RPC calls via span context
- Configurable via environment variable (`RUST_LOG=debug`)

**Implementation**:
```rust
use tracing::{info, debug, warn, error, instrument};

#[instrument(skip(self), fields(node_id = %self.node_id, term = %self.current_term))]
fn start_election(&mut self) {
    info!("Starting election");
    // ...
}
```

**Alternatives Considered**:
- **env_logger + log**: Older, less structured, no span support
- **slog**: Structured but more verbose API than tracing

---

## Best Practices: Raft Leader Election

### Raft-Specific Implementation Notes

1. **Randomized Election Timeouts**:
   - Use `rand::thread_rng()` to randomize timeout in range [150ms, 300ms] (if base is 150ms)
   - Reset timer on: receiving AppendEntries from leader, granting vote, starting election
   - Implementation: `std::time::Instant` + `thread::sleep` in election timer thread

2. **Vote Granting Rules** (from Raft paper):
   ```
   Grant vote if:
   - Candidate's term >= my current_term AND
   - I haven't voted in this term (voted_for is None or equals candidate) AND
   - Candidate's log is at least as up-to-date as mine

   Phase 1 simplification: No log comparison (no logs yet)
   ```

3. **Thread Safety**:
   - RaftNode state protected by `Arc<Mutex<RaftState>>`
   - Minimize lock hold time: compute, then acquire lock to update
   - Avoid nested locks to prevent deadlocks

4. **Heartbeat Timing**:
   - Leader sends heartbeat (empty AppendEntries) every 50ms (< election timeout)
   - Phase 1: AppendEntries without log entries (just term + leader_id)

5. **Term Monotonicity**:
   - Terms only increase, never decrease
   - On receiving higher term: immediately step down to Follower, update term
   - Persist term BEFORE sending RPC responses (crash safety)

---

## Summary of Resolved Unknowns

| Unknown | Resolution |
|---------|-----------|
| Rust version | 1.75.0 stable |
| Persistent storage | serde_json + std::fs (atomic writes) |
| Rocket integration | v0.5.1 in dedicated thread, channels to Raft core |
| HTTP client | reqwest 0.12 blocking mode |
| Channel library | std::mpsc + crossbeam::channel |
| Config management | dotenvy + std::env |
| Error handling | thiserror (lib) + anyhow (app) |
| Logging | tracing + tracing-subscriber |

**Phase 0 Complete**: All technical decisions made. Ready for Phase 1 (design artifacts).

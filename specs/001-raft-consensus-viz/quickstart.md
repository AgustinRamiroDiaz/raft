# Quickstart Guide: Raft Leader Election (Phase 1)

**Date**: 2025-10-18
**Feature**: 001-raft-consensus-viz
**Scope**: Getting started with Phase 1 implementation

## Overview

This guide walks you through setting up, building, and running a 3-node Raft cluster for leader election testing. Phase 1 implements leader election and term handling only (no log replication).

---

## Prerequisites

**Required**:
- Rust 1.90 (stable)
- cargo (comes with Rust)

**Installation**:
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version  # Should be 1.90 or later
cargo --version
```

---

## Project Setup

### 1. Initialize the Rust Project

```bash
# Navigate to repository root
cd /home/az/dev/raft

# Create Rust project directory
mkdir -p raft-node
cd raft-node

# Initialize Cargo project
cargo init --name raft-node

# Verify structure
tree -L 2
# raft-node/
# ├── Cargo.toml
# └── src/
#     └── main.rs
```

### 2. Configure Dependencies

Edit `raft-node/Cargo.toml`:

```toml
[package]
name = "raft-node"
version = "0.1.0"
edition = "2021"
rust-version = "1.90"

[dependencies]
# CLI
clap = { version = "4.5", features = ["derive", "env"] }

# HTTP Server
rocket = { version = "0.5.1", features = ["json"] }

# HTTP Client (blocking mode)
reqwest = { version = "0.12", features = ["blocking", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Environment variables
dotenvy = "0.15"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Concurrency
crossbeam-channel = "0.5"

# Random (for election timeout)
rand = "0.8"

[dev-dependencies]
# Testing utilities
# (none currently)
```

### 3. Create Source Structure

```bash
# From raft-node/ directory
mkdir -p src/{raft,rpc,http,config,threading}
mkdir -p tests/{unit,integration,contract}

# Create module files
touch src/lib.rs
touch src/raft/mod.rs src/raft/{node,state,types,election}.rs
touch src/rpc/mod.rs src/rpc/{messages,transport}.rs
touch src/http/mod.rs src/http/{server,client}.rs
touch src/config/mod.rs
touch src/threading/mod.rs src/threading/timers.rs

# Create test files
touch tests/integration/election_test.rs
touch tests/integration/http_test.rs
touch tests/contract/rpc_schema_test.rs
```

### 4. Create Environment Configuration

Create `.env.node1` (for node 1):
```bash
NODE_ID=node1
NODE_ADDRESS=127.0.0.1:8001
CLUSTER_PEERS=node2@127.0.0.1:8002,node3@127.0.0.1:8003
ELECTION_TIMEOUT_MS=150
HEARTBEAT_INTERVAL_MS=50
RUST_LOG=info,raft_node=debug
```

Create `.env.node2` (for node 2):
```bash
NODE_ID=node2
NODE_ADDRESS=127.0.0.1:8002
CLUSTER_PEERS=node1@127.0.0.1:8001,node3@127.0.0.1:8003
ELECTION_TIMEOUT_MS=150
HEARTBEAT_INTERVAL_MS=50
RUST_LOG=info,raft_node=debug
```

Create `.env.node3` (for node 3):
```bash
NODE_ID=node3
NODE_ADDRESS=127.0.0.1:8003
CLUSTER_PEERS=node1@127.0.0.1:8001,node2@127.0.0.1:8002
ELECTION_TIMEOUT_MS=150
HEARTBEAT_INTERVAL_MS=50
RUST_LOG=info,raft_node=debug
```

Create `.env.example` (for documentation):
```bash
# Node configuration
NODE_ID=nodeX
NODE_ADDRESS=127.0.0.1:800X
CLUSTER_PEERS=node1@127.0.0.1:8001,node2@127.0.0.1:8002

# Timing configuration (milliseconds)
ELECTION_TIMEOUT_MS=150
HEARTBEAT_INTERVAL_MS=50

# Logging (see tracing-subscriber docs)
RUST_LOG=info,raft_node=debug
```

---

## Development Workflow

### Test-Driven Development (TDD) Example

Following Constitution Principle V, write tests first:

**Step 1: Write Failing Test**

Create `src/raft/state.rs` with test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_follower_to_candidate_transition() {
        let mut state = RaftState::new("node1".to_string(), vec!["node2".to_string()]);
        assert_eq!(state.state, NodeState::Follower);

        state.start_election();

        assert_eq!(state.state, NodeState::Candidate);
        assert_eq!(state.current_term, 1);  // Term incremented
        assert_eq!(state.voted_for, Some("node1".to_string()));  // Voted for self
    }
}
```

**Step 2: Run Test (Verify Failure)**
```bash
cargo test test_follower_to_candidate_transition
# Should fail: start_election() doesn't exist yet
```

**Step 3: Implement Minimum Code**
```rust
impl RaftState {
    pub fn start_election(&mut self) {
        self.current_term += 1;
        self.state = NodeState::Candidate;
        self.voted_for = Some(self.node_id.clone());
    }
}
```

**Step 4: Run Test (Verify Pass)**
```bash
cargo test test_follower_to_candidate_transition
# Should pass
```

**Step 5: Refactor (If Needed)**

Repeat this cycle for each feature.

---

## Running the Cluster

### Build the Project

```bash
# From raft-node/ directory
cargo build --release

# Verify build
./target/release/raft-node --help
# Should show CLI usage
```

### Start 3-Node Cluster

**Terminal 1 (Node 1)**:
```bash
cd raft-node
source .env.node1  # Load environment variables
cargo run --release
# Or: ./target/release/raft-node
```

**Terminal 2 (Node 2)**:
```bash
cd raft-node
source .env.node2
cargo run --release
```

**Terminal 3 (Node 3)**:
```bash
cd raft-node
source .env.node3
cargo run --release
```

### Expected Behavior

1. **Startup (0-10 seconds)**:
   - All nodes start as Followers
   - Random election timeouts (150-300ms)
   - One node (say node2) times out first, becomes Candidate
   - node2 sends RequestVote to node1 and node3

2. **Election (~150-500ms)**:
   - node1 and node3 grant votes to node2 (first candidate they see)
   - node2 receives 2 votes (majority of 3) → becomes Leader
   - node2 starts sending heartbeats every 50ms

3. **Steady State**:
   - node2 remains Leader, sends heartbeats
   - node1 and node3 remain Followers, reset timeout on each heartbeat
   - Logs show: `[node2] Became leader in term 1`

### Verify Cluster Status

```bash
# Query node 1 status
curl http://127.0.0.1:8001/ops/status | jq
# {
#   "node_id": "node1",
#   "state": "Follower",
#   "current_term": 1,
#   "leader_id": "node2",
#   "cluster_peers": ["node2", "node3"]
# }

# Query node 2 status (leader)
curl http://127.0.0.1:8002/ops/status | jq
# {
#   "node_id": "node2",
#   "state": "Leader",
#   "current_term": 1,
#   "leader_id": "node2",
#   "cluster_peers": ["node1", "node3"]
# }
```

### Test Leader Failure and Re-Election

1. **Stop the leader** (Ctrl+C in Terminal 2)
2. **Observe re-election**:
   - node1 or node3 times out after ~150-300ms
   - New election in term 2
   - Remaining 2 nodes elect new leader
3. **Verify**:
   ```bash
   curl http://127.0.0.1:8001/ops/status | jq
   # Should show term 2 and new leader (node1 or node3)
   ```

---

## Testing

### Run All Tests

```bash
# Unit tests (in-module #[cfg(test)])
cargo test --lib

# Integration tests
cargo test --test '*'

# All tests
cargo test

# With verbose output
cargo test -- --nocapture
```

### Run Specific Test Suite

```bash
# Only election tests
cargo test election

# Only HTTP tests
cargo test http

# Contract tests
cargo test --test rpc_schema_test
```

### Test with Coverage (Optional)

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run tests with coverage
cargo tarpaulin --out Html --output-dir coverage

# Open coverage/index.html in browser
```

---

## Debugging

### Enable Detailed Logging

```bash
# Set RUST_LOG for trace-level debugging
RUST_LOG=trace cargo run

# Or edit .env.node1:
RUST_LOG=trace,hyper=info,rocket=info
# (trace for raft_node, info for HTTP libraries to reduce noise)
```

### Common Issues

**Issue**: "Address already in use"
```
Error: Os { code: 48, kind: AddrInUse, message: "Address already in use" }
```
**Solution**: Kill existing processes on ports 8001-8003
```bash
# Find process
lsof -i :8001

# Kill process
kill -9 <PID>

# Or use different ports in .env files
```

**Issue**: Nodes don't elect leader (all timeout repeatedly)
```
[node1] Election timeout, starting election in term 5
[node2] Election timeout, starting election in term 5
[node3] Election timeout, starting election in term 5
```
**Solution**: Split votes due to simultaneous elections. Randomized timeouts should prevent this - verify `rand` is working correctly.

---

## CLI Usage

### Basic Usage

```bash
# Start node with env file
raft-node --env .env.node1

# Start node with inline env vars
NODE_ID=node1 NODE_ADDRESS=127.0.0.1:8001 \
CLUSTER_PEERS=node2@127.0.0.1:8002,node3@127.0.0.1:8003 \
raft-node

# Show help
raft-node --help

# Show version
raft-node --version
```

### Expected CLI Interface (Clap)

```
USAGE:
    raft-node [OPTIONS]

OPTIONS:
    -e, --env <FILE>           Load environment variables from file [default: .env]
    -c, --config <FILE>        (Future) Load TOML config file
    -h, --help                 Print help information
    -V, --version              Print version information

ENVIRONMENT VARIABLES:
    NODE_ID                    Node unique identifier (required)
    NODE_ADDRESS               IP:port to listen on (required)
    CLUSTER_PEERS              Comma-separated peers: id@host:port,... (required)
    ELECTION_TIMEOUT_MS        Election timeout in milliseconds [default: 150]
    HEARTBEAT_INTERVAL_MS      Heartbeat interval in milliseconds [default: 50]
    RUST_LOG                   Log level [default: info]
```

---

## API Reference

### Consensus Endpoints (Node-to-Node)

**POST /raft/request_vote**
- Request: `{ "term": 5, "candidate_id": "node2" }`
- Response: `{ "term": 5, "vote_granted": true }`

**POST /raft/append_entries**
- Request: `{ "term": 5, "leader_id": "node1" }`
- Response: `{ "term": 5, "success": true }`

### Operations Endpoints (Monitoring)

**GET /ops/status**
- Response: `{ "node_id": "node1", "state": "Leader", "current_term": 5, ... }`

**GET /ops/health**
- Response: `{ "status": "healthy" }`

Full API specification: See [contracts/raft-rpc.yaml](contracts/raft-rpc.yaml)

---

## Next Steps

After successfully running the leader election implementation:

1. **Add log replication** (User Story 2 - State Machine Replication)
   - Implement log entries and replication logic
   - Add full AppendEntries with log consistency checks
   - Implement a simple key-value state machine

2. **Enhance monitoring** (User Story 3 - Cluster Health Monitoring)
   - Add detailed metrics endpoints
   - Consider Prometheus-compatible metrics

3. **Build visualization** (User Story 4 - Log Replication Visualization)
   - Create frontend application (e.g., Tauri + Leptos)
   - Display real-time cluster state
   - Visualize log replication progress

---

## Troubleshooting

### Logs Show "Already voted in this term"

This is normal during elections. A node can only vote once per term.

### No Leader Elected After 30 Seconds

Check:
1. Are all 3 nodes running? (`ps aux | grep raft-node`)
2. Can nodes reach each other? (`curl http://127.0.0.1:8002/ops/health`)
3. Are election timeouts configured correctly? (should be >> heartbeat interval)

### Nodes Forget State After Restart

This is expected behavior. All state is in-memory only (no persistence). When a node restarts, it starts fresh as a Follower with term 0 and will participate in the next election.

---

## Performance Tuning

### Adjust Election Timeout

For faster elections (testing):
```bash
ELECTION_TIMEOUT_MS=50  # Elections complete in ~100-150ms
```

For production (network latency tolerance):
```bash
ELECTION_TIMEOUT_MS=1000  # Elections complete in ~2-3 seconds
```

### Cluster Size Trade-offs

- **3 nodes**: Tolerates 1 failure, fast consensus (2/3 majority)
- **5 nodes**: Tolerates 2 failures, slower consensus (3/5 majority)
- **7 nodes**: Tolerates 3 failures, slowest consensus (4/7 majority)

Recommendation: Start with 3 nodes for development.

---

## References

- **Raft Paper**: https://raft.github.io/raft.pdf
- **Raft Visualization**: https://raft.github.io/ (interactive animation)
- **Rust Book**: https://doc.rust-lang.org/book/
- **Rocket Guide**: https://rocket.rs/guide/
- **Clap Documentation**: https://docs.rs/clap/latest/clap/

---

**Happy Consensus Building!** 🚀

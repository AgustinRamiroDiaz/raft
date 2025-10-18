# Data Model: Raft Leader Election

**Date**: 2025-10-18
**Feature**: 001-raft-consensus-viz
**Scope**: Core data structures for leader election without log replication

## Overview

This document defines the data model for the Raft implementation, focusing solely on leader election and term handling. Log entries are explicitly excluded.

---

## Core Entities

### 1. NodeId

**Purpose**: Unique identifier for a Raft node in the cluster

**Rust Type**:
```rust
pub type NodeId = String;
```

**Validation Rules**:
- Non-empty string
- Must be unique within cluster
- Alphanumeric + hyphens/underscores only
- Max length: 64 characters

**Rationale**: String type allows human-readable IDs (e.g., "node1", "us-east-1-replica-3") for debugging. Could be optimized to UUID in later phases if needed.

---

### 2. Term

**Purpose**: Logical clock for Raft consensus, increments on each election

**Rust Type**:
```rust
pub type Term = u64;
```

**Validation Rules**:
- Non-negative integer
- Monotonically increasing (never decreases)
- Starts at 0 for new nodes

**State Transitions**:
```
term = 0 (initial)
  → term++ (when starting election as Candidate)
  → term = max(term, received_term) (when receiving higher term from peer)
```

**Rationale**: u64 provides 2^64 possible terms, enough for centuries of elections at 1000 elections/second.

---

### 3. NodeState

**Purpose**: Represents the current role of a Raft node

**Rust Type**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}
```

**Validation Rules**:
- Must be one of three states at all times
- State transitions follow Raft state machine rules (see below)

**State Transitions**:
```
Follower
  → Candidate (election timeout elapses)
  → Follower (receive AppendEntries from leader with term >= current_term)

Candidate
  → Leader (receive votes from majority)
  → Follower (discover leader with term >= current_term)
  → Candidate (election timeout, start new election with term++)

Leader
  → Follower (discover higher term)
```

**Rationale**: Enum ensures type safety - impossible to be in invalid state. Copy trait allows efficient passing by value.

---

### 4. RaftState

**Purpose**: Complete state of a Raft node (both volatile and persistent)

**Rust Type**:
```rust
pub struct RaftState {
    // Consensus state (in-memory only)
    pub current_term: Term,
    pub voted_for: Option<NodeId>,
    pub state: NodeState,
    pub leader_id: Option<NodeId>,
    pub votes_received: HashSet<NodeId>,

    // Node identity and configuration
    pub node_id: NodeId,
    pub cluster_peers: Vec<NodeId>,
}
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `current_term` | `Term` | Latest term this node has seen (in-memory, not persisted) |
| `voted_for` | `Option<NodeId>` | Candidate ID that received vote in current term (None if no vote cast) |
| `state` | `NodeState` | Current role (Follower/Candidate/Leader) |
| `leader_id` | `Option<NodeId>` | ID of current leader (None if no leader known) |
| `votes_received` | `HashSet<NodeId>` | Node IDs that voted for this node in current term (only used in Candidate state) |
| `node_id` | `NodeId` | This node's unique identifier |
| `cluster_peers` | `Vec<NodeId>` | List of other nodes in cluster (excluding self) |

**Validation Rules**:
- `current_term` must be monotonically increasing
- `voted_for` must be None or a valid NodeId in cluster
- `votes_received` only populated when `state == Candidate`
- `leader_id` must be None or a valid NodeId in cluster
- `cluster_peers` must not contain `node_id`

**Invariants**:
1. If `voted_for.is_some()`, then `voted_for` corresponds to current_term
2. If `state == Leader`, then `leader_id == Some(node_id)`
3. If `state == Follower`, then `votes_received.is_empty()`
4. `votes_received` never contains `node_id` (node votes for itself implicitly)

**Rationale**: All state is in-memory only (no persistence). Uses Option for voted_for to distinguish "not yet voted" from "voted for someone". On restart, nodes start fresh with term 0.

---

### 5. RaftConfig

**Purpose**: Configuration loaded from environment variables

**Rust Type**:
```rust
#[derive(Debug, Clone)]
pub struct RaftConfig {
    pub node_id: NodeId,
    pub node_address: SocketAddr,
    pub cluster_peers: HashMap<NodeId, SocketAddr>,
    pub election_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub state_file_path: PathBuf,
}
```

**Fields**:

| Field | Type | Env Var | Default | Description |
|-------|------|---------|---------|-------------|
| `node_id` | `NodeId` | `NODE_ID` | (required) | This node's unique ID |
| `node_address` | `SocketAddr` | `NODE_ADDRESS` | (required) | IP:port this node listens on |
| `cluster_peers` | `HashMap<NodeId, SocketAddr>` | `CLUSTER_PEERS` | (required) | Peer nodes (format: "id1@host1:port1,id2@host2:port2") |
| `election_timeout_ms` | `u64` | `ELECTION_TIMEOUT_MS` | 150 | Base election timeout in milliseconds |
| `heartbeat_interval_ms` | `u64` | `HEARTBEAT_INTERVAL_MS` | 50 | Leader heartbeat interval in milliseconds |

**Validation Rules**:
- `node_id` must not appear in `cluster_peers` keys
- `election_timeout_ms` must be > `heartbeat_interval_ms` (typically 3x+)
- Minimum cluster size: 3 nodes (1 + 2 peers) for meaningful majority

**Rationale**: Immutable config loaded once at startup. HashMap for O(1) peer address lookup by NodeId.

---

## RPC Messages

### 6. RequestVote

**Purpose**: RPC sent by Candidate to request votes from other nodes

**Rust Type**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVote {
    pub term: Term,
    pub candidate_id: NodeId,
    // Phase 1: No log fields (last_log_index, last_log_term)
}
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `term` | `Term` | Candidate's term |
| `candidate_id` | `NodeId` | ID of candidate requesting vote |

**Validation Rules**:
- `term` must be > 0 (term 0 is only for initial state, no elections in term 0)
- `candidate_id` must be non-empty

**Rationale**: Simplified for Phase 1. In later phases, will add `last_log_index` and `last_log_term` for log comparison.

---

### 7. RequestVoteResponse

**Purpose**: RPC response to RequestVote

**Rust Type**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    pub term: Term,
    pub vote_granted: bool,
}
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `term` | `Term` | Responder's current term (for candidate to update itself) |
| `vote_granted` | `bool` | True if candidate received vote |

**Vote Granting Logic**:
```rust
vote_granted = (
    request.term >= self.current_term &&
    (self.voted_for.is_none() || self.voted_for == Some(request.candidate_id))
)
```

**Rationale**: Simple boolean response. In later phases, might add rejection reasons for debugging.

---

### 8. AppendEntries

**Purpose**: RPC sent by Leader for heartbeats (Phase 1: no log entries)

**Rust Type**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntries {
    pub term: Term,
    pub leader_id: NodeId,
    // Phase 1: No log fields (prev_log_index, prev_log_term, entries, leader_commit)
}
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `term` | `Term` | Leader's term |
| `leader_id` | `NodeId` | Leader's node ID |

**Validation Rules**:
- `term` must be > 0
- `leader_id` must be non-empty

**Rationale**: In Phase 1, AppendEntries is purely a heartbeat to prevent election timeouts. Log replication fields added in Phase 2.

---

### 9. AppendEntriesResponse

**Purpose**: RPC response to AppendEntries

**Rust Type**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    pub term: Term,
    pub success: bool,
}
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `term` | `Term` | Responder's current term |
| `success` | `bool` | True if follower accepted heartbeat (term is valid) |

**Success Logic** (Phase 1):
```rust
success = (request.term >= self.current_term)
```

**Rationale**: In Phase 1, success is purely term-based. In Phase 2, will include log consistency checks.

---

## Relationships

```
RaftConfig (1) ----creates----> RaftState (1)
    |                               |
    | contains                      | uses
    v                               v
cluster_peers (N)            RequestVote / AppendEntries (RPC)
                                    |
                                    | returns
                                    v
                        RequestVoteResponse / AppendEntriesResponse
```

**Key Relationships**:
- Each `RaftState` is associated with one `RaftConfig` (loaded at startup)
- `RaftState.cluster_peers` mirrors `RaftConfig.cluster_peers.keys()`
- `votes_received` in `RaftState` is a subset of `cluster_peers` + `{node_id}` (if candidate)
- `leader_id` in `RaftState` is either None or one of `cluster_peers` or `node_id`

---

## Thread Communication Messages

### 11. RaftCommand

**Purpose**: Commands sent from Raft core to HTTP server thread

**Rust Type**:
```rust
#[derive(Debug, Clone)]
pub enum RaftCommand {
    SendRequestVote {
        target: NodeId,
        request: RequestVote,
    },
    SendAppendEntries {
        target: NodeId,
        request: AppendEntries,
    },
}
```

**Rationale**: Raft core doesn't directly call HTTP client. Instead, sends commands via channel to decouple transport from algorithm.

---

### 12. RaftEvent

**Purpose**: Events sent to Raft core thread from various sources

**Rust Type**:
```rust
#[derive(Debug)]
pub enum RaftEvent {
    // From HTTP server thread
    ReceivedRequestVote {
        from: NodeId,
        request: RequestVote,
        response_tx: oneshot::Sender<RequestVoteResponse>,
    },
    ReceivedAppendEntries {
        from: NodeId,
        request: AppendEntries,
        response_tx: oneshot::Sender<AppendEntriesResponse>,
    },

    // From election timer thread
    ElectionTimeout,

    // From heartbeat timer thread (only when node is Leader)
    HeartbeatTimeout,
}
```

**Event Sources**:

| Event | Publisher (who sends) | Consumer (who receives) | When |
|-------|----------------------|------------------------|------|
| `ReceivedRequestVote` | HTTP server thread | Raft core thread | When HTTP endpoint receives POST /raft/request_vote |
| `ReceivedAppendEntries` | HTTP server thread | Raft core thread | When HTTP endpoint receives POST /raft/append_entries |
| `ElectionTimeout` | Election timer thread | Raft core thread | When randomized election timeout elapses (150-300ms) |
| `HeartbeatTimeout` | Heartbeat timer thread | Raft core thread | Every 50ms (only started when node becomes Leader) |

**Threading Architecture**:
```
Timer Threads:
┌─────────────────────┐
│ Election Timer      │──┐
│ (always running)    │  │
└─────────────────────┘  │
                         │ ElectionTimeout
┌─────────────────────┐  │ HeartbeatTimeout
│ Heartbeat Timer     │  │
│ (Leader only)       │──┤
└─────────────────────┘  │
                         ↓
HTTP Server Thread:      ┌──────────────────┐
┌─────────────────────┐  │                  │
│ Rocket HTTP         │──┤   Raft Core      │
│ (incoming RPCs)     │  │   Thread         │
└─────────────────────┘  │ (event loop)     │
                         └──────────────────┘
                                │
                                │ RaftCommand
                                ↓
                         ┌──────────────────┐
                         │ HTTP Client      │
                         │ (outgoing RPCs)  │
                         └──────────────────┘
```

**Rationale**:
- HTTP endpoints don't directly mutate Raft state - they send events via channel
- Timer threads are independent, send timeout events to trigger elections/heartbeats
- Raft core thread is single-threaded event loop, processes one event at a time
- This ensures all state mutations happen sequentially (no race conditions)

---

## Validation Summary

**Testable Invariants**:

1. **Term Monotonicity**: `new_term >= old_term` (always)
2. **Single Vote per Term**: If `voted_for.is_some()` in term T, cannot vote for different candidate in same term T
3. **Leader Completeness**: If node is Leader in term T, it must have received votes from majority
4. **State Validity**: `votes_received.len() >= majority` ⇒ `state == Leader`
5. **Restart Behavior**: After crash/restart, node starts as Follower with term 0

**Edge Cases**:

1. **Split Vote**: No candidate gets majority → timeout → new election with term++
2. **Stale Term**: Candidate receives response with higher term → step down to Follower
3. **Concurrent Elections**: Multiple candidates in same term → at most one gets majority (due to single vote per term)
4. **Leader Crash During Election**: New leader elected, old leader's term now stale

---

## Summary

**Core Entities**: 11 total
- Configuration: RaftConfig
- State: RaftState, NodeState (enum), Term, NodeId
- RPCs: RequestVote, RequestVoteResponse, AppendEntries, AppendEntriesResponse
- Threading: RaftCommand, RaftEvent

**Design Principles**:
- All state is in-memory (no persistence)
- Type safety via enums (NodeState) and type aliases (Term, NodeId)
- Immutable configuration (RaftConfig)
- Event-driven communication between threads
- Single-threaded Raft core (event loop pattern)

**Threading Model**:
- Raft core: Single thread with event loop
- HTTP server: Separate thread (Rocket)
- Timers: Separate threads (election timer always running, heartbeat timer when Leader)
- Communication: All via crossbeam channels

**Validation**: All entities have defined validation rules and state transition constraints testable via unit tests.

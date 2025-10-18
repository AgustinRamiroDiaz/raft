# Implementation Plan: Raft Consensus Implementation - Phase 1 (Leader Election)

**Branch**: `001-raft-consensus-viz` | **Date**: 2025-10-18 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-raft-consensus-viz/spec.md`

**Note**: This implementation focuses on leader election and term handling only (no log replication).

## Summary

This implementation plan covers the first phase of the Raft consensus algorithm: leader election and term handling. The system will implement a Rust-based consensus node that can form clusters, elect leaders, and handle term transitions without log replication. This represents User Story 1 (P1 - Node Cluster Formation) from the feature specification.

**Technical Approach**: Native Rust threads and channels for concurrency (no async runtime), Rocket framework for HTTP communication, Clap for CLI interface. Emphasis on modularity and testability following TDD principles.

## Technical Context

**Language/Version**: Rust 1.90 (stable edition)
**Primary Dependencies**:
- Rocket 0.5.1 (HTTP server framework)
- Clap 4.5 (CLI argument parsing)
- Serde 1.0 (serialization/deserialization)
- Reqwest 0.12 (HTTP client, blocking mode)
- crossbeam-channel (MPSC channels for thread communication)
- dotenvy (environment variable loading)
- tracing + tracing-subscriber (structured logging)
- anyhow + thiserror (error handling)
**Storage**: In-memory only (no persistence)
**Testing**: cargo test (unit tests, integration tests, contract tests)
**Target Platform**: Linux/macOS (cross-platform server)
**Project Type**: Single Rust project (backend service with CLI)
**Concurrency Model**: Native Rust threads + channels (no async runtime per user requirement)
**Performance Goals**:
- Leader election within 10 seconds on cluster startup
- Re-election within 2x election timeout (~300ms with 150ms default)
- HTTP response time <100ms for status endpoints
**Constraints**:
- No async runtime (tokio/async-std) - use sync threads/channels
- Minimal dependencies to reduce complexity
- Must support environment variable configuration (12-factor)
**Scale/Scope**:
- Support 3-7 node clusters (typical Raft deployment)
- Leader election only (no log replication)
- ~1500-2000 LOC for core Raft state machine + HTTP layer

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Modularity & Testability ✅
**Status**: COMPLIANT

- Raft core state machine will be isolated from HTTP transport layer
- Clear module boundaries: `raft_core`, `http_server`, `http_client`, `cli`, `persistence`
- Each module independently testable through dependency injection
- TDD approach ensures testability from the start

### Principle II: Dependency Injection ✅
**Status**: COMPLIANT

- HTTP server/client will be injected into Raft core via traits
- Persistence layer will be abstracted behind trait (e.g., `StateStorage`)
- Thread communication via channels passed as dependencies
- No global state - all configuration through constructor/builder

### Principle III: Replicable Development Environment ⚠️
**Status**: DEFERRED

**Justification**: This implementation has minimal external dependencies (no database, no message queue). The Raft nodes themselves are the distributed system being developed. Docker setup will be added when we have a complete system to containerize.

**Simpler Alternative**: Run nodes directly via `cargo run` during development. Nodes are configured via environment variables, making local setup trivial.

### Principle IV: Strict TypeScript & Linting ⏭️
**Status**: NOT APPLICABLE (Rust project only)

### Principle V: Test-First Development ✅
**Status**: COMPLIANT

- TDD strictly enforced: Write failing test → Implement → Pass test
- Unit tests for: Raft state transitions, term management, vote counting
- Integration tests for: HTTP communication, multi-node scenarios
- Contract tests for: RequestVote and AppendEntries RPC schemas

**Gate Result**: ✅ PASSED (1 deferred with justification, 1 N/A)

---

**Post-Design Re-evaluation** (after Phase 1 design):

### Principle I: Modularity & Testability ✅
**Status**: COMPLIANT (Verified)

Design artifacts confirm modular structure:
- **data-model.md**: Shows clear entity boundaries (RaftState, NodeState, RPC messages)
- **Source structure**: 6 independent modules (raft, rpc, http, persistence, config, threading)
- **Transport abstraction**: `Transport` trait allows mocking HTTP for unit tests
- **StateStorage trait**: File persistence can be swapped with in-memory for tests

### Principle II: Dependency Injection ✅
**Status**: COMPLIANT (Verified)

Design confirms DI pattern:
- RaftNode will receive `Box<dyn Transport>` for RPC communication
- Persistence via `Box<dyn StateStorage>` injected into RaftState
- Channels passed as constructor parameters (no global state)
- research.md specifies builder pattern for complex RaftNode construction

### Principle V: Test-First Development ✅
**Status**: COMPLIANT (Plan Verified)

- quickstart.md includes TDD workflow example (Red-Green-Refactor)
- 3-tier testing strategy defined: unit, integration, contract
- contracts/raft-rpc.yaml provides schema for contract tests
- All test directories created in source structure

**Final Gate Result**: ✅ PASSED - All design artifacts align with constitutional principles

## Project Structure

### Documentation (this feature)

```
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```
raft-node/                    # Rust workspace for Raft implementation
├── Cargo.toml               # Workspace manifest
├── src/
│   ├── main.rs              # CLI entrypoint (clap)
│   ├── lib.rs               # Library root
│   ├── raft/
│   │   ├── mod.rs           # Raft module root
│   │   ├── node.rs          # RaftNode struct (core state machine)
│   │   ├── state.rs         # NodeState enum (Follower/Candidate/Leader)
│   │   ├── types.rs         # Term, NodeId, RaftConfig
│   │   └── election.rs      # Election logic (candidate state, vote counting)
│   ├── rpc/
│   │   ├── mod.rs           # RPC module root
│   │   ├── messages.rs      # RequestVote, RequestVoteResponse structs
│   │   └── transport.rs     # Transport trait (abstracts HTTP)
│   ├── http/
│   │   ├── mod.rs           # HTTP module root
│   │   ├── server.rs        # Rocket HTTP server (consensus + ops endpoints)
│   │   └── client.rs        # HTTP client (reqwest blocking, implements Transport)
│   ├── config/
│   │   └── mod.rs           # Environment variable loading (12-factor)
│   └── threading/
│       ├── mod.rs           # Thread management
│       └── timers.rs        # Election and heartbeat timers
│
├── tests/
│   ├── unit/                # Unit tests (co-located with modules via #[cfg(test)])
│   ├── integration/         # Integration tests
│   │   ├── election_test.rs # Multi-node election scenarios
│   │   └── http_test.rs     # HTTP RPC communication tests
│   └── contract/            # Contract tests
│       └── rpc_schema_test.rs # Validate RequestVote schema compliance
│
└── .env.example             # Example environment configuration
```

**Structure Decision**: Single Rust project structure. This is a backend service with no frontend component. The structure emphasizes clear separation of concerns:
- `raft/`: Core consensus algorithm (transport-agnostic)
- `rpc/`: RPC message definitions and transport abstraction
- `http/`: HTTP-specific transport implementation (Rocket + reqwest)
- `config/`: Environment-based configuration
- `threading/`: Timer threads and channel management for decoupling HTTP from Raft core

## Complexity Tracking

*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| No Docker setup (Principle III) | Focuses on core algorithm development | Adding Docker now would add setup complexity without benefit - nodes run directly with `cargo run` and env vars. Docker will be added when we have visualization + need multi-host testing. |

---

## Planning Artifacts Summary

### Phase 0: Research (Complete ✅)

**Artifact**: [research.md](research.md)

Resolved all technical unknowns from Technical Context:
- Rust version: 1.90 stable
- State storage: In-memory only (no persistence)
- HTTP server: Rocket 0.5.1 in dedicated thread
- HTTP client: reqwest 0.12 blocking mode
- Thread communication: std::sync::mpsc + crossbeam-channel
- Timer architecture: Separate threads for election and heartbeat timeouts
- Configuration: dotenvy for 12-factor env vars
- Error handling: thiserror (library) + anyhow (application)
- Logging: tracing + tracing-subscriber
- Testing strategy: 3-tier (unit, integration, contract)
- Raft implementation best practices documented

### Phase 1: Design (Complete ✅)

**Artifacts**:
1. **[data-model.md](data-model.md)**: Defines 11 core entities
   - Configuration: RaftConfig
   - State: RaftState, NodeState, Term, NodeId (all in-memory)
   - RPCs: RequestVote, RequestVoteResponse, AppendEntries, AppendEntriesResponse
   - Threading: RaftCommand, RaftEvent
   - All entities include validation rules and state transitions
   - Detailed threading architecture with timer event flows

2. **[contracts/raft-rpc.yaml](contracts/raft-rpc.yaml)**: OpenAPI 3.0 specification
   - 4 endpoints: /raft/request_vote, /raft/append_entries, /ops/status, /ops/health
   - Complete JSON schemas for all RPC messages
   - Example requests/responses
   - Contract testing ready

3. **[quickstart.md](quickstart.md)**: Developer onboarding guide
   - Prerequisites and setup instructions
   - TDD workflow example (Red-Green-Refactor)
   - Running a 3-node cluster
   - Testing, debugging, and troubleshooting
   - CLI usage and API reference

### Phase 2: Task Generation (Ready)

**Next Command**: `/speckit.tasks`

This will generate [tasks.md](tasks.md) with dependency-ordered implementation tasks based on the design artifacts above.

---

## Implementation Readiness

### ✅ Ready to Begin Implementation

**Prerequisites Met**:
- [x] All NEEDS CLARIFICATION items resolved
- [x] Constitutional compliance verified
- [x] Complete data model with validation rules
- [x] API contracts defined (OpenAPI spec)
- [x] Source code structure documented
- [x] Development workflow established (TDD)
- [x] Quickstart guide for developers

**Success Criteria** (from spec.md):
- SC-001: 3-node cluster elects leader within 10 seconds ✓ (design supports)
- SC-002: Re-election within 2x timeout (~300ms) ✓ (design supports)
- SC-007: Ops server responds <100ms ✓ (design supports)

**Scope Verification**:
- ✅ Leader election implementation (User Story 1)
- ✅ Term handling (in-memory only)
- ✅ HTTP-based node-to-node communication
- ✅ Operations server for status monitoring
- ✅ Timer-based architecture (election and heartbeat timers)
- ❌ Log replication (not in scope)
- ❌ Visualization frontend (not in scope)
- ❌ State persistence (not in scope)

---

## Next Steps

1. **Generate Tasks**: Run `/speckit.tasks` to create actionable task list
2. **Initialize Project**: Create `raft-node/` directory and Cargo.toml
3. **Begin TDD Cycle**: Start with first test (e.g., NodeState transitions)
4. **Iterate**: Red-Green-Refactor for each module

**Estimated Implementation Time**: 12-15 hours (leader election only, no persistence)

---

**Planning Complete** 🎯
**Branch**: `001-raft-consensus-viz`
**Plan File**: [specs/001-raft-consensus-viz/plan.md](plan.md)
**Ready for**: `/speckit.tasks` command


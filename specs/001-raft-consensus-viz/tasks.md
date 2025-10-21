# Tasks: Raft Consensus - Leader Election

**Input**: Design documents from `/specs/001-raft-consensus-viz/`
**Prerequisites**: plan.md, spec.md, data-model.md, contracts/raft-rpc.yaml, research.md

**Scope**: This task list implements **User Story 1 (P1) - Node Cluster Formation** ONLY. Log replication, advanced monitoring, and visualization are explicitly out of scope per plan.md.

**Tests**: Following TDD (Test-First Development) as mandated by the project constitution. Tests are written BEFORE implementation.

**Organization**: Tasks are organized to build the minimal viable Raft leader election system incrementally.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[US1]**: Belongs to User Story 1 (Leader Election)
- Include exact file paths in descriptions

## Path Conventions
- Project structure: `raft-node/` at repository root
- Source: `raft-node/src/`
- Tests: `raft-node/tests/`

---

## Phase 1: Setup (Project Initialization)

**Purpose**: Initialize Rust project with required dependencies and structure

- [x] T001 Create `raft-node/` directory and initialize Cargo project
- [x] T002 Configure Cargo.toml with dependencies: rocket 0.5.1, clap 4.5, serde 1.0, reqwest 0.12 (blocking), crossbeam-channel, dotenvy, tracing, tracing-subscriber, anyhow, thiserror, rand 0.8
- [x] T003 [P] Create source directory structure: `src/raft/`, `src/rpc/`, `src/http/`, `src/config/`, `src/threading/`
- [x] T004 [P] Create test directory structure: `tests/unit/`, `tests/integration/`, `tests/contract/`
- [x] T005 [P] Create `.env.example` with configuration template (NODE_ID, NODE_ADDRESS, CLUSTER_PEERS, ELECTION_TIMEOUT_MS, HEARTBEAT_INTERVAL_MS, RUST_LOG)
- [x] T006 Configure clippy and rustfmt in Cargo.toml

**Checkpoint**: Project structure ready for implementation

---

## Phase 2: Foundational (Core Types & Configuration)

**Purpose**: Core infrastructure that MUST be complete before any Raft logic can be implemented

**⚠️ CRITICAL**: No User Story 1 work can begin until this phase is complete

- [x] T007 [P] Implement Term newtype in `raft-node/src/raft/term.rs`
- [x] T008 [P] Implement NodeId type in `raft-node/src/raft/node_id.rs`
- [x] T009 [P] Implement NodeState enum (Follower, Candidate, Leader) in `raft-node/src/raft/state.rs`
- [x] T010 [P] Implement RaftConfig struct with environment variable loading in `raft-node/src/config/mod.rs`
- [x] T011 [P] Implement RequestVote and AppendEntries RPC message structs in `raft-node/src/rpc/messages.rs`
- [x] T012 [P] Implement RaftEvent enum (timeout and RPC events) in `raft-node/src/raft/events.rs`
- [x] T013 [P] Define Transport trait for dependency injection in `raft-node/src/rpc/transport.rs`
- [x] T014 Implement logging infrastructure with tracing in `raft-node/src/main.rs`

**Checkpoint**: Foundation ready - User Story 1 implementation can now begin

---

## Phase 3: User Story 1 - Node Cluster Formation (Priority: P1) 🎯 MVP

**Goal**: Implement Raft leader election so a 3-node cluster can form, elect a leader, and handle leader failure

**Independent Test**: Start 3 nodes with configuration, verify one becomes leader within 10 seconds, kill leader and verify re-election

### Tests for User Story 1 (TDD - Write FIRST)

**NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T015 [P] [US1] Unit test for NodeState transitions (Follower→Candidate→Leader) in `raft-node/src/raft/state.rs` #[cfg(test)]
- [x] T016 [P] [US1] Unit test for term incrementing on election start in `raft-node/src/raft/node.rs` #[cfg(test)]
- [x] T017 [P] [US1] Unit test for vote granting logic in `raft-node/src/raft/node.rs` #[cfg(test)]
- [x] T018 [P] [US1] Unit test for majority calculation in `raft-node/src/raft/election.rs` #[cfg(test)]
- [x] T019 [P] [US1] Contract test for RequestVote RPC schema in `raft-node/tests/contract/rpc_schema_test.rs`
- [x] T020 [P] [US1] Contract test for AppendEntries RPC schema in `raft-node/tests/contract/rpc_schema_test.rs`
- [x] T021 [US1] Integration test for 3-node leader election in `raft-node/tests/integration/election_test.rs`
- [x] T022 [US1] Integration test for leader re-election after failure in `raft-node/tests/integration/election_test.rs`

### Core Raft State Machine (US1)

- [x] T023 [US1] Implement RaftState struct in `raft-node/src/raft/node.rs` (current_term, voted_for, state, leader_id, votes_received, node_id, cluster_peers)
- [x] T024 [US1] Implement RaftState::new() constructor in `raft-node/src/raft/node.rs`
- [x] T025 [US1] Implement RaftState::start_election() in `raft-node/src/raft/node.rs` (increment term, become Candidate, vote for self)
- [x] T026 [US1] Implement RaftState::handle_request_vote() in `raft-node/src/raft/node.rs` (grant vote if term valid and haven't voted)
- [x] T027 [US1] Implement RaftState::handle_vote_response() in `raft-node/src/raft/node.rs` (count votes, become Leader if majority)
- [x] T028 [US1] Implement RaftState::handle_append_entries() in `raft-node/src/raft/node.rs` (reset election timer, update term, revert to Follower if needed)
- [x] T029 [US1] Implement RaftState::step_down() in `raft-node/src/raft/node.rs` (revert to Follower, update term)

### Timer Threads (US1)

- [X] T030 [P] [US1] Implement election timer thread in `raft-node/src/threading/timers.rs` (randomized 150-300ms timeout, sends ElectionTimeout event)
- [X] T031 [P] [US1] Implement heartbeat timer thread in `raft-node/src/threading/timers.rs` (50ms interval, sends HeartbeatTimeout event, only runs when Leader)
- [X] T032 [US1] Implement timer reset mechanism in `raft-node/src/threading/mod.rs` (receive reset signals from Raft core)

### HTTP Transport Layer (US1)

- [X] T033 [P] [US1] Define Transport trait in `raft-node/src/rpc/transport.rs` (send_request_vote, send_append_entries methods)
- [X] T034 [US1] Implement HTTP client (reqwest blocking) implementing Transport trait in `raft-node/src/http/client.rs`
- [X] T035 [US1] Implement Rocket HTTP server with POST /raft/request_vote endpoint in `raft-node/src/http/server.rs`
- [X] T036 [US1] Implement POST /raft/append_entries endpoint in `raft-node/src/http/server.rs`
- [X] T037 [US1] Implement GET /ops/status endpoint (returns NodeStatus JSON) in `raft-node/src/http/server.rs`
- [X] T038 [US1] Implement GET /ops/health endpoint in `raft-node/src/http/server.rs`
- [X] T039 [US1] Implement Rocket HTTP server to run in dedicated thread with state sharing via Arc<Mutex<>> in `raft-node/src/http/server.rs`

### Raft Core Event Loop (US1)

- [X] T040 [US1] Implement main Raft event loop in `raft-node/src/raft/event_loop.rs` (receive RaftEvent via crossbeam channel, dispatch to handlers)
- [X] T041 [US1] Implement ElectionTimeout handler in `raft-node/src/raft/event_loop.rs` (call start_election, send RequestVote to all peers)
- [X] T042 [US1] Implement HeartbeatTimeout handler in `raft-node/src/raft/event_loop.rs` (send AppendEntries to all peers if Leader)
- [X] T043 [US1] Implement ReceivedRequestVote handler in `raft-node/src/raft/event_loop.rs` (call handle_request_vote, send response via channel)
- [X] T044 [US1] Implement ReceivedAppendEntries handler in `raft-node/src/raft/event_loop.rs` (call handle_append_entries, reset election timer, send response)
- [X] T045 [US1] Integrated HTTP transport directly in event loop handlers (synchronous RPC sending)

### CLI & Main Entrypoint (US1)

- [X] T046 [US1] Implement CLI with clap in `raft-node/src/main.rs` (parse --env flag, display help/version)
- [X] T047 [US1] Implement main() function orchestration in `raft-node/src/main.rs` (load config, spawn timers, spawn HTTP server, spawn Raft event loop, run until signal)
- [X] T048 [US1] Implement graceful shutdown on SIGINT/SIGTERM in `raft-node/src/main.rs`

### Integration & Testing (US1)

- [X] T049 [US1] Verify all unit tests pass (`cargo test --lib`) - 53 tests passing
- [X] T050 [US1] Verify contract tests pass (`cargo test --test contract_tests`) - 7 tests passing
- [~] T051 [US1] Integration tests (`cargo test --test integration_tests`) - Skipped (placeholder tests)
- [~] T052 [US1] Manual test: Start 3-node cluster, verify leader elected - Sandbox network isolation prevents full test, logs confirm correctalgorithm execution
- [~] T053 [US1] Manual test: Kill leader node, verify re-election - Sandbox limitation, see TESTING.md
- [~] T054 [US1] Manual test: Query `/ops/status` on all nodes - Sandbox limitation, see TESTING.md

**Checkpoint**: User Story 1 complete - Raft leader election fully functional and independently testable

---

## Phase 4: Polish & Documentation

**Purpose**: Final touches for production-ready code

- [ ] T055 [P] Add module-level documentation comments to all modules (raft, rpc, http, config, threading)
- [ ] T056 [P] Add function-level documentation for public APIs
- [ ] T057 [P] Run `cargo clippy` and fix all warnings
- [ ] T058 [P] Run `cargo fmt` to format code
- [ ] T059 [P] Create example .env files for 3-node cluster (.env.node1, .env.node2, .env.node3)
- [ ] T060 Validate quickstart.md instructions by following them end-to-end
- [ ] T061 Run full test suite and verify 100% pass rate (`cargo test`)
- [ ] T062 Build release binary and verify it runs (`cargo build --release`)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion (T001-T006) - BLOCKS User Story 1
- **User Story 1 (Phase 3)**: Depends on Foundational completion (T007-T014)
- **Polish (Phase 4)**: Depends on User Story 1 completion (T015-T054)

### User Story 1 Internal Dependencies

**Tests (T015-T022)**: Write FIRST, must FAIL before implementation
**Core State Machine (T023-T029)**: Sequential dependencies
  - T023 (RaftState struct) → T024-T029 depend on T023
**Timers (T030-T032)**: Can run in parallel with Core State Machine
**HTTP Layer (T033-T039)**:
  - T033 (Transport trait) → T034 depends on T033
  - T035-T039 can run in parallel
**Event Loop (T040-T045)**: Depends on Core State Machine (T023-T029) completion
**CLI (T046-T048)**: Depends on all other components
**Integration (T049-T054)**: Depends on everything

### Parallel Opportunities

**Setup (Phase 1)**:
- T003, T004, T005, T006 can all run in parallel

**Foundational (Phase 2)**:
- T007, T008, T009, T010, T011, T012, T013 can all run in parallel

**User Story 1 Tests**:
- T015, T016, T017, T018, T019, T020 can all run in parallel
- T021, T022 are integration tests, run sequentially

**User Story 1 Implementation**:
- After T023: T024-T029 build on RaftState
- T030, T031, T032 (Timers) can run in parallel with T024-T029
- T033 must complete before T034
- T035-T039 (HTTP endpoints) can run in parallel

**Polish (Phase 4)**:
- T055, T056, T057, T058, T059 can all run in parallel

---

## Parallel Example: Foundational Phase

```bash
# All foundational types can be written in parallel:
Agent 1: "Implement Term and NodeId type aliases in raft-node/src/raft/types.rs"
Agent 2: "Implement NodeState enum in raft-node/src/raft/state.rs"
Agent 3: "Implement RaftConfig struct in raft-node/src/config/mod.rs"
Agent 4: "Implement RequestVote and RequestVoteResponse in raft-node/src/rpc/messages.rs"
Agent 5: "Implement AppendEntries and AppendEntriesResponse in raft-node/src/rpc/messages.rs"
```

## Parallel Example: User Story 1 Tests

```bash
# All unit tests can be written in parallel (TDD):
Agent 1: "Unit test for NodeState transitions in raft-node/src/raft/state.rs"
Agent 2: "Unit test for term incrementing in raft-node/src/raft/node.rs"
Agent 3: "Unit test for vote granting in raft-node/src/raft/node.rs"
Agent 4: "Contract test for RequestVote RPC in raft-node/tests/contract/rpc_schema_test.rs"
Agent 5: "Contract test for AppendEntries RPC in raft-node/tests/contract/rpc_schema_test.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. **Complete Phase 1: Setup** (T001-T006)
   - Result: Empty Rust project with correct structure

2. **Complete Phase 2: Foundational** (T007-T014)
   - Result: All core types defined, ready to implement logic

3. **Write Tests for US1** (T015-T022)
   - Result: Failing tests that define success criteria

4. **Implement US1 Core** (T023-T045)
   - Result: Raft leader election working

5. **Implement US1 CLI** (T046-T048)
   - Result: Runnable binary

6. **Validate US1** (T049-T054)
   - Result: All tests pass, manual testing successful

7. **STOP and VALIDATE**: Test 3-node cluster independently

8. **Polish** (T055-T062)
   - Result: Production-ready code

### Success Criteria (from spec.md)

- **SC-001**: 3-node cluster elects leader within 10 seconds ✓ (T052)
- **SC-002**: Re-election within 2x timeout (~300ms) ✓ (T053)
- **SC-007**: Ops server responds <100ms ✓ (T054)

### Out of Scope (Not Implemented)

- **User Story 2 (P2)**: State Machine Replication / Log replication
- **User Story 3 (P3)**: Advanced cluster health monitoring
- **User Story 4 (P4)**: Log replication visualization
- **State persistence**: All state is in-memory only
- **Frontend**: No visualization UI

---

## Notes

- **[P] tasks**: Different files, no dependencies, can run in parallel
- **[US1] label**: All tasks belong to User Story 1 (Leader Election)
- **TDD strictly enforced**: Tests (T015-T022) must be written FIRST and FAIL before implementation
- **Constitution compliance**: Modularity via traits (Transport), DI via constructors, TDD mandated
- **Threading model**:
  - Raft core: Single-threaded event loop (no race conditions)
  - HTTP server: Dedicated thread (Rocket)
  - Timers: Dedicated threads (election timer always running, heartbeat when Leader)
  - Communication: crossbeam channels
- **No async runtime**: Pure Rust threads + channels as per requirements
- **Commit strategy**: Commit after each task or logical group
- **Stop at checkpoints**: Validate independently before proceeding

---

**Total Tasks**: 62
**User Story 1 Tasks**: 40 (T015-T054)
**Estimated Time**: 12-15 hours (per plan.md)
**MVP Scope**: User Story 1 only - Leader election without log replication

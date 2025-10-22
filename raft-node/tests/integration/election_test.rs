//! Integration tests for leader election
//!
//! These tests verify that a multi-node Raft cluster can successfully
//! elect a leader and handle leader failures.

use raft_node::http;
use raft_node::raft::event_loop::run_event_loop;
use raft_node::raft::{NodeState, RaftState};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Helper to create a test Raft node with all components
struct TestNode {
    node_id: String,
    raft_state: Arc<Mutex<RaftState>>,
    _server_handle: thread::JoinHandle<()>,
    _event_loop_handle: thread::JoinHandle<()>,
    shutdown: Arc<AtomicBool>,
}

impl TestNode {
    fn new(node_id: &str, port: u16, peer_configs: Vec<(String, u16)>) -> Self {
        // Create peer addresses for HTTP client
        let mut peer_addresses = HashMap::new();
        let mut peer_ids = Vec::new();
        for (peer_id, peer_port) in peer_configs {
            peer_addresses.insert(
                peer_id.clone().into(),
                format!("http://127.0.0.1:{}", peer_port),
            );
            peer_ids.push(peer_id.into());
        }

        let transport = Arc::new(http::client::HttpClient::new(peer_addresses));

        // Create Raft state (SHARED between event loop and HTTP server)
        let raft_state_shared = Arc::new(Mutex::new(RaftState::new(node_id.into(), peer_ids)));

        // Create event channel
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        // Create control flags
        let election_reset = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        // Spawn HTTP server
        let server_handle = http::server::spawn_server(
            format!("127.0.0.1:{}", port),
            raft_state_shared.clone(),
            event_tx.clone(),
        );

        // Spawn election timer
        let _election_timer = raft_node::threading::timers::spawn_election_timer(
            event_tx.clone(),
            150, // 150ms base timeout
            election_reset.clone(),
            shutdown.clone(),
        );

        // Spawn heartbeat timer
        let _heartbeat_timer = raft_node::threading::timers::spawn_heartbeat_timer(
            event_tx.clone(),
            50, // 50ms interval
            shutdown.clone(),
        );

        // Spawn event loop in background
        let event_loop_election_reset = election_reset.clone();
        let event_loop_raft_state = raft_state_shared.clone();
        let event_loop_handle = thread::spawn(move || {
            run_event_loop(event_loop_raft_state, event_rx, transport, event_loop_election_reset);
        });

        Self {
            node_id: node_id.to_string(),
            raft_state: raft_state_shared,
            _server_handle: server_handle,
            _event_loop_handle: event_loop_handle,
            shutdown,
        }
    }

    fn get_state(&self) -> NodeState {
        self.raft_state.lock().unwrap().state
    }

    fn get_term(&self) -> u64 {
        self.raft_state.lock().unwrap().current_term.value()
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

// T021: Integration test for 3-node leader election
#[test]
fn test_three_node_cluster_elects_leader() {
    // Start 3 Raft nodes
    let node1 = TestNode::new(
        "node1",
        9001,
        vec![("node2".to_string(), 9002), ("node3".to_string(), 9003)],
    );

    let node2 = TestNode::new(
        "node2",
        9002,
        vec![("node1".to_string(), 9001), ("node3".to_string(), 9003)],
    );

    let node3 = TestNode::new(
        "node3",
        9003,
        vec![("node1".to_string(), 9001), ("node2".to_string(), 9002)],
    );

    // Wait for leader election (election timeout is 150-300ms, allow multiple rounds)
    thread::sleep(Duration::from_secs(1));

    // Check cluster state
    let state1 = node1.get_state();
    let state2 = node2.get_state();
    let state3 = node3.get_state();

    eprintln!("Node1 state: {:?}, term: {}", state1, node1.get_term());
    eprintln!("Node2 state: {:?}, term: {}", state2, node2.get_term());
    eprintln!("Node3 state: {:?}, term: {}", state3, node3.get_term());

    let states = vec![
        (node1.node_id.clone(), state1),
        (node2.node_id.clone(), state2),
        (node3.node_id.clone(), state3),
    ];

    // Count leaders
    let leader_count = states.iter().filter(|(_, state)| *state == NodeState::Leader).count();

    // Verify exactly one leader
    assert_eq!(
        leader_count, 1,
        "Expected exactly 1 leader, found {}",
        leader_count
    );

    // Verify other nodes are followers
    let follower_count = states.iter().filter(|(_, state)| *state == NodeState::Follower).count();
    assert!(follower_count >= 2, "Expected at least 2 followers");

    // Verify all nodes agree on the same term (within reason)
    let terms = vec![node1.get_term(), node2.get_term(), node3.get_term()];
    let max_term = *terms.iter().max().unwrap();
    let min_term = *terms.iter().min().unwrap();
    assert!(
        max_term - min_term <= 2,
        "Terms diverged too much: {:?}",
        terms
    );

    // Cleanup
    node1.shutdown();
    node2.shutdown();
    node3.shutdown();
}

// T022: Integration test for leader re-election after failure
#[test]
fn test_leader_reelection_after_failure() {
    // Start 3-node cluster
    let node1 = TestNode::new(
        "node1",
        9011,
        vec![("node2".to_string(), 9012), ("node3".to_string(), 9013)],
    );

    let node2 = TestNode::new(
        "node2",
        9012,
        vec![("node1".to_string(), 9011), ("node3".to_string(), 9013)],
    );

    let node3 = TestNode::new(
        "node3",
        9013,
        vec![("node1".to_string(), 9011), ("node2".to_string(), 9012)],
    );

    // Wait for initial leader election
    thread::sleep(Duration::from_millis(500));

    // Find the leader
    let initial_leader = if node1.get_state() == NodeState::Leader {
        Some(("node1", &node1))
    } else if node2.get_state() == NodeState::Leader {
        Some(("node2", &node2))
    } else if node3.get_state() == NodeState::Leader {
        Some(("node3", &node3))
    } else {
        None
    };

    assert!(initial_leader.is_some(), "No initial leader elected");

    let (leader_name, leader_node) = initial_leader.unwrap();
    let initial_term = leader_node.get_term();

    // Kill the leader
    leader_node.shutdown();

    // Wait for re-election (should happen within 500ms)
    thread::sleep(Duration::from_millis(1000));

    // Check remaining nodes
    let survivors: Vec<&TestNode> = match leader_name {
        "node1" => vec![&node2, &node3],
        "node2" => vec![&node1, &node3],
        _ => vec![&node1, &node2],
    };

    // Count new leaders
    let new_leader_count = survivors
        .iter()
        .filter(|n| n.get_state() == NodeState::Leader)
        .count();

    assert_eq!(
        new_leader_count, 1,
        "Expected exactly 1 new leader after failure"
    );

    // Verify term has increased
    let new_term = survivors.iter().map(|n| n.get_term()).max().unwrap();
    assert!(
        new_term > initial_term,
        "Term should increase after re-election"
    );

    // Cleanup
    for node in survivors {
        node.shutdown();
    }
}


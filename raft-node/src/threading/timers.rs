//! Timer threads for Raft timeouts
//!
//! Provides election and heartbeat timer threads that send timeout events
//! to the Raft core event loop.

use crate::raft::events::RaftEvent;
use crossbeam_channel::Sender;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Spawns an election timer thread
///
/// The election timer sends `ElectionTimeout` events at randomized intervals
/// between base_timeout_ms and 2*base_timeout_ms.
///
/// The timer can be reset by setting the `reset_flag` to true.
///
/// # Arguments
/// * `event_sender` - Channel to send timeout events to Raft core
/// * `base_timeout_ms` - Base election timeout in milliseconds (will be randomized)
/// * `reset_flag` - Atomic flag to signal timer reset (set to true to reset)
/// * `shutdown` - Atomic flag to signal thread shutdown
///
/// # Returns
/// A join handle for the spawned thread
pub fn spawn_election_timer(
    event_sender: Sender<RaftEvent>,
    base_timeout_ms: u64,
    reset_flag: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        tracing::debug!("Election timer thread started");

        let mut rng = rand::thread_rng();

        while !shutdown.load(Ordering::Relaxed) {
            // Randomize timeout between base_timeout_ms and 2*base_timeout_ms
            let timeout_ms = rng.gen_range(base_timeout_ms..=(2 * base_timeout_ms));
            let timeout = Duration::from_millis(timeout_ms);

            tracing::trace!(timeout_ms, "Election timer set");

            // Wait for timeout, checking for reset or shutdown periodically
            let start = std::time::Instant::now();
            while start.elapsed() < timeout {
                // Check if we should reset the timer
                if reset_flag.swap(false, Ordering::Relaxed) {
                    tracing::trace!("Election timer reset");
                    break; // Break to outer loop to generate new random timeout
                }

                // Check shutdown flag
                if shutdown.load(Ordering::Relaxed) {
                    tracing::debug!("Election timer shutting down");
                    return;
                }

                // Sleep briefly to avoid busy-waiting
                thread::sleep(Duration::from_millis(10));
            }

            // If we completed the timeout without reset, send event
            if start.elapsed() >= timeout {
                tracing::debug!(timeout_ms, "Election timeout elapsed");
                if event_sender.send(RaftEvent::ElectionTimeout).is_err() {
                    tracing::error!("Failed to send election timeout event (receiver dropped)");
                    return;
                }
            }
        }

        tracing::debug!("Election timer thread stopped");
    })
}

/// Spawns a heartbeat timer thread
///
/// The heartbeat timer sends `HeartbeatTimeout` events at regular intervals.
/// This timer should only be started when the node becomes a Leader.
///
/// # Arguments
/// * `event_sender` - Channel to send timeout events to Raft core
/// * `interval_ms` - Heartbeat interval in milliseconds
/// * `shutdown` - Atomic flag to signal thread shutdown
///
/// # Returns
/// A join handle for the spawned thread
pub fn spawn_heartbeat_timer(
    event_sender: Sender<RaftEvent>,
    interval_ms: u64,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        tracing::debug!("Heartbeat timer thread started");

        let interval = Duration::from_millis(interval_ms);

        while !shutdown.load(Ordering::Relaxed) {
            thread::sleep(interval);

            // Check shutdown again after sleep
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            tracing::trace!("Heartbeat timeout elapsed");
            if event_sender.send(RaftEvent::HeartbeatTimeout).is_err() {
                tracing::error!("Failed to send heartbeat timeout event (receiver dropped)");
                return;
            }
        }

        tracing::debug!("Heartbeat timer thread stopped");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;
    use std::time::Instant;

    #[test]
    fn test_election_timer_fires() {
        let (tx, rx) = unbounded();
        let reset_flag = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let _handle = spawn_election_timer(tx, 50, reset_flag.clone(), shutdown.clone());

        // Wait for timeout event (should arrive between 50-100ms)
        let start = Instant::now();
        let event = rx.recv_timeout(Duration::from_millis(150)).unwrap();
        let elapsed = start.elapsed();

        assert!(matches!(event, RaftEvent::ElectionTimeout));
        assert!(elapsed >= Duration::from_millis(50));
        assert!(elapsed <= Duration::from_millis(150));

        // Shutdown
        shutdown.store(true, Ordering::Relaxed);
    }

    #[test]
    fn test_election_timer_reset() {
        let (tx, rx) = unbounded();
        let reset_flag = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let _handle = spawn_election_timer(tx, 100, reset_flag.clone(), shutdown.clone());

        // Wait 50ms, then reset
        thread::sleep(Duration::from_millis(50));
        reset_flag.store(true, Ordering::Relaxed);

        // Should not receive timeout yet (timer was reset)
        thread::sleep(Duration::from_millis(60));
        assert!(rx.try_recv().is_err());

        // Should receive timeout after another 100-200ms
        let event = rx.recv_timeout(Duration::from_millis(250)).unwrap();
        assert!(matches!(event, RaftEvent::ElectionTimeout));

        // Shutdown
        shutdown.store(true, Ordering::Relaxed);
    }

    #[test]
    fn test_heartbeat_timer_fires_regularly() {
        let (tx, rx) = unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));

        let _handle = spawn_heartbeat_timer(tx, 30, shutdown.clone());

        // Receive 3 heartbeat events
        for _ in 0..3 {
            let start = Instant::now();
            let event = rx.recv_timeout(Duration::from_millis(50)).unwrap();
            let elapsed = start.elapsed();

            assert!(matches!(event, RaftEvent::HeartbeatTimeout));
            assert!(elapsed >= Duration::from_millis(25));
            assert!(elapsed <= Duration::from_millis(45));
        }

        // Shutdown
        shutdown.store(true, Ordering::Relaxed);
    }

    #[test]
    fn test_shutdown_stops_timers() {
        let (tx, rx) = unbounded();
        let reset_flag = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let handle = spawn_election_timer(tx, 50, reset_flag.clone(), shutdown.clone());

        // Signal shutdown
        shutdown.store(true, Ordering::Relaxed);

        // Thread should terminate quickly
        handle.join().unwrap();

        // Should not receive any events
        thread::sleep(Duration::from_millis(100));
        assert!(rx.try_recv().is_err());
    }
}

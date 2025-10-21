#!/bin/bash
# Start 3-node Raft cluster

set -e

cd "$(dirname "$0")"

# Kill any existing nodes
echo "Stopping any existing nodes..."
pkill -f "raft-node" || true
sleep 1

# Create log directory
mkdir -p /tmp/claude

# Start node1
echo "Starting node1 on port 8001..."
./target/release/raft-node --env .env.node1 > /tmp/claude/raft-node1.log 2>&1 &
echo "  PID: $!"

# Start node2
echo "Starting node2 on port 8002..."
./target/release/raft-node --env .env.node2 > /tmp/claude/raft-node2.log 2>&1 &
echo "  PID: $!"

# Start node3
echo "Starting node3 on port 8003..."
./target/release/raft-node --env .env.node3 > /tmp/claude/raft-node3.log 2>&1 &
echo "  PID: $!"

echo ""
echo "All nodes started!"
echo "Logs: /tmp/claude/raft-node{1,2,3}.log"
echo ""
echo "To test cluster: ./test-cluster.sh"
echo "To stop cluster: pkill -f raft-node"

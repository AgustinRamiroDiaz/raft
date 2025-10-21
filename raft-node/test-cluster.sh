#!/bin/bash
# Test script for 3-node Raft cluster

set -e

cd "$(dirname "$0")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Raft 3-Node Cluster Test ===${NC}\n"

# Function to check if node is running
check_node() {
    local port=$1
    local node_name=$2

    if curl -s "http://127.0.0.1:${port}/ops/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $node_name is running (port $port)"
        return 0
    else
        echo -e "${RED}✗${NC} $node_name is not responding (port $port)"
        return 1
    fi
}

# Function to get node status
get_status() {
    local port=$1
    local node_name=$2

    echo -e "\n${YELLOW}Status of $node_name:${NC}"
    curl -s "http://127.0.0.1:${port}/ops/status" | jq '.' || echo "Failed to get status"
}

# Wait for nodes to start
echo "Waiting for nodes to start (5 seconds)..."
sleep 5

# Check all nodes
echo -e "\n${YELLOW}Checking node health:${NC}"
check_node 8001 "node1"
check_node 8002 "node2"
check_node 8003 "node3"

# Get status from all nodes
get_status 8001 "node1"
get_status 8002 "node2"
get_status 8003 "node3"

# Wait for election
echo -e "\n${YELLOW}Waiting for leader election (10 seconds)...${NC}"
sleep 10

# Check leader election
echo -e "\n${YELLOW}Checking leader election:${NC}"
for port in 8001 8002 8003; do
    status=$(curl -s "http://127.0.0.1:${port}/ops/status")
    state=$(echo "$status" | jq -r '.state')
    term=$(echo "$status" | jq -r '.current_term')
    leader=$(echo "$status" | jq -r '.leader_id')
    node=$(echo "$status" | jq -r '.node_id')

    echo "  $node: state=$state, term=$term, leader=$leader"

    if [ "$state" == "Leader" ]; then
        echo -e "${GREEN}  └─ Found leader: $node${NC}"
    fi
done

echo -e "\n${GREEN}=== Test Complete ===${NC}"
echo "To kill all nodes: pkill -f raft-node"

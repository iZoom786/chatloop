#!/bin/bash
#
# ChatLoop Coordinator Launch Script for YARN
#
# This script is executed by YARN to start the coordinator container.
# It sets up the environment and starts the coordinator.

set -e

# Configuration
COORDINATOR_PORT="${CHATLOOP_PORT:-50050}"
CONFIG_PATH="/home/chatloop/configs/coordinator-config.yaml"

echo "Starting ChatLoop Coordinator"
echo "Port: ${COORDINATOR_PORT}"
echo "Config: ${CONFIG_PATH}"

# Wait for workers to be ready (optional)
# This is useful if workers need to start before the coordinator
if [ "${WAIT_FOR_WORKERS:-false}" = "true" ]; then
    echo "Waiting for workers to be ready..."
    # Could implement health check logic here
fi

# Start the coordinator
echo "Starting coordinator process..."
exec chatloop-coordinator

# If we get here, the coordinator has exited
echo "Coordinator process exited with code $?"

#!/bin/bash
#
# ChatLoop Worker Launch Script for YARN
#
# This script is executed by YARN to start a worker container.
# It sets up the environment, binds CPU cores (optional), and starts the worker.

set -e

# Configuration from YARN environment
WORKER_ID="${CHATLOOP_WORKER_ID:-0}"
WORKER_PORT="${CHATLOOP_PORT:-50051}"
LAYER_GROUP_START="${LAYER_GROUP_START:-0}"
LAYER_GROUP_END="${LAYER_GROUP_END:-16}"
MODEL_PATH="/home/chatloop/models"
CONFIG_PATH="/home/chatloop/configs/worker-config.yaml"

echo "Starting ChatLoop Worker ${WORKER_ID}"
echo "Port: ${WORKER_PORT}"
echo "Layer Group: ${LAYER_GROUP_START}-${LAYER_GROUP_END}"
echo "Model Path: ${MODEL_PATH}"

# Optional: NUMA binding (if configured)
if [ -n "${NUMA_NODE}" ]; then
    echo "Binding to NUMA node ${NUMA_NODE}"
    command="numactl --cpunodebind=${NUMA_NODE} --membind=${NUMA_NODE}"
else
    command=""
fi

# Optional: CPU pinning (if configured)
if [ -n "${CPU_CORES}" ]; then
    echo "Pinning to CPU cores: ${CPU_CORES}"
    command="${command} taskset -c ${CPU_CORES}"
fi

# Start the worker
echo "Starting worker process..."
exec ${command} chatloop-worker

# If we get here, the worker has exited
echo "Worker process exited with code $?"

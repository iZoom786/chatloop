#!/bin/bash
# ChatLoop Docker Build and Run Script
# No Rust installation required - everything runs in Docker

set -e

IMAGE_NAME="chatloop-all-in-one"

function show_help() {
    cat << EOF
ChatLoop Docker Management Script

Usage: ./docker-run.sh [command] [options]

Commands:
    build                   Build the Docker image
    run-coordinator         Run a coordinator container
    run-worker [id]         Run a worker container (default id: 0)
    stop-all                Stop all ChatLoop containers
    cleanup                 Remove all ChatLoop containers and images
    logs [service]          Show logs for a service
    status                  Show status of all containers
    help                    Show this help message

Examples:
    # Build the image
    ./docker-run.sh build

    # Run coordinator
    ./docker-run.sh run-coordinator

    # Run worker 0
    ./docker-run.sh run-worker 0

    # Run worker 1
    ./docker-run.sh run-worker 1

    # Stop all containers
    ./docker-run.sh stop-all

    # View logs
    ./docker-run.sh logs coordinator
    ./docker-run.sh logs worker-0

Environment Variables:
    CHATLOOP_MODEL_PATH       Path to model partition (for workers)
    CHATLOOP_CONFIG_PATH      Path to config directory
    CHATLOOP_PORT             Port to expose (default: 50051 for workers, 50050 for coordinator)

EOF
}

function build_image() {
    echo "Building ChatLoop Docker image..."
    docker build -f docker/all-in-one.Dockerfile -t $IMAGE_NAME:latest .
    echo "Build complete!"
}

function run_coordinator() {
    local port=${CHATLOOP_PORT:-50050}
    echo "Starting ChatLoop Coordinator on port $port..."

    docker run -d \
        --name chatloop-coordinator \
        --restart unless-stopped \
        -e CHATLOOP_ROLE=coordinator \
        -e CHATLOOP_PORT=$port \
        -p $port:50050 \
        -p 9090:9091 \
        -v "${PWD}/configs/coordinator-config.yaml:/home/chatloop/configs/coordinator-config.yaml:ro" \
        -v "${PWD}/logs:/home/chatloop/logs" \
        $IMAGE_NAME:latest

    echo "Coordinator started! Access it at localhost:$port"
    echo "View logs: docker logs -f chatloop-coordinator"
}

function run_worker() {
    local worker_id=${1:-0}
    local port=${CHATLOOP_PORT:-$((50051 + worker_id))}
    local config_file="${PWD}/configs/worker-config-${worker_id}.yaml"

    # Check if config exists
    if [ ! -f "$config_file" ]; then
        echo "Error: Config file not found: $config_file"
        echo "Please create it first or use the default config."
        exit 1
    fi

    echo "Starting ChatLoop Worker ${worker_id} on port $port..."

    docker run -d \
        --name chatloop-worker-${worker_id} \
        --restart unless-stopped \
        -e CHATLOOP_ROLE=worker \
        -e CHATLOOP_WORKER_ID=worker-${worker_id} \
        -e CHATLOOP_PORT=$port \
        -p $port:50051 \
        -p $((9091 + worker_id)):9091 \
        -v "${PWD}/models:/home/chatloop/models:ro" \
        -v "${config_file}:/home/chatloop/configs/worker-config.yaml:ro" \
        -v "${PWD}/logs:/home/chatloop/logs" \
        $IMAGE_NAME:latest

    echo "Worker ${worker_id} started!"
    echo "View logs: docker logs -f chatloop-worker-${worker_id}"
}

function stop_all() {
    echo "Stopping all ChatLoop containers..."
    docker stop chatloop-coordinator chatloop-worker-0 chatloop-worker-1 chatloop-worker-2 chatloop-worker-3 2>/dev/null || true
    docker rm chatloop-coordinator chatloop-worker-0 chatloop-worker-1 chatloop-worker-2 chatloop-worker-3 2>/dev/null || true
    echo "All containers stopped and removed."
}

function cleanup() {
    echo "Cleaning up ChatLoop containers and images..."
    stop_all
    docker rmi $IMAGE_NAME:latest 2>/dev/null || true
    echo "Cleanup complete."
}

function show_logs() {
    local service=${1:-coordinator}
    local container_name="chatloop-${service}"

    if docker ps --format '{{.Names}}' | grep -q "^${container_name}$"; then
        docker logs -f $container_name
    else
        echo "Container $container_name is not running."
        echo "Run './docker-run.sh status' to see all containers."
    fi
}

function show_status() {
    echo "ChatLoop Container Status:"
    echo "=========================="
    docker ps --filter "name=chatloop-" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
}

# Main script logic
case "${1:-help}" in
    build)
        build_image
        ;;
    run-coordinator)
        run_coordinator
        ;;
    run-worker)
        run_worker ${2:-0}
        ;;
    stop-all)
        stop_all
        ;;
    cleanup)
        cleanup
        ;;
    logs)
        show_logs ${2:-coordinator}
        ;;
    status)
        show_status
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "Unknown command: $1"
        echo ""
        show_help
        exit 1
        ;;
esac

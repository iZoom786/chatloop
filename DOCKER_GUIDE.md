# ChatLoop Docker Quick Start

This guide shows you how to run ChatLoop using Docker - **no Rust installation required**!

## Prerequisites

- Docker Desktop installed (https://www.docker.com/products/docker-desktop)
- At least 8GB RAM available for Docker
- 20GB free disk space

## Quick Start (Windows PowerShell)

### 1. Build the Docker Image

```powershell
# Navigate to chatloop directory
cd E:\chatloop

# Build the all-in-one image
.\docker-run.ps1 build
```

This will take 5-10 minutes as it:
1. Downloads the Rust base image
2. Installs build dependencies
3. Compiles all Rust code
4. Creates the runtime image

### 2. Start the Coordinator

```powershell
.\docker-run.ps1 run-coordinator
```

### 3. Start Workers

```powershell
# Start worker 0
.\docker-run.ps1 run-worker 0

# Start worker 1
.\docker-run.ps1 run-worker 1

# Start worker 2
.\docker-run.ps1 run-worker 2

# Start worker 3
.\docker-run.ps1 run-worker 3
```

### 4. Check Status

```powershell
.\docker-run.ps1 status
```

### 5. View Logs

```powershell
# Coordinator logs
.\docker-run.ps1 logs coordinator

# Worker 0 logs
.\docker-run.ps1 logs worker-0
```

### 6. Stop All Services

```powershell
.\docker-run.ps1 stop-all
```

## Quick Start (Linux/macOS)

### 1. Build the Docker Image

```bash
cd chatloop
chmod +x docker-run.sh
./docker-run.sh build
```

### 2. Start Services

```bash
# Start coordinator
./docker-run.sh run-coordinator

# Start workers
./docker-run.sh run-worker 0
./docker-run.sh run-worker 1
./docker-run.sh run-worker 2
./docker-run.sh run-worker 3
```

### 3. Check Status

```bash
./docker-run.sh status
```

### 4. View Logs

```bash
./docker-run.sh logs coordinator
./docker-run.sh logs worker-0
```

### 5. Stop All

```bash
./docker-run.sh stop-all
```

## Using Docker Compose (Alternative)

If you prefer docker-compose:

```powershell
# Build and start all services
docker-compose -f docker-compose-all-in-one.yml up -d

# View logs
docker-compose -f docker-compose-all-in-one.yml logs -f

# Stop services
docker-compose -f docker-compose-all-in-one.yml down
```

## Configuration

Before starting workers, you need to create configuration files:

### Coordinator Config

Create `configs/coordinator-config.yaml`:

```yaml
mode: "coordinator"
bind_address: "0.0.0.0"
port: 50050

coordinator:
  worker_endpoints:
    - "http://worker-0:50051"
    - "http://worker-1:50052"
    - "http://worker-2:50053"
    - "http://worker-3:50054"
  health_check_interval_secs: 5
  failure_threshold: 3
  request_timeout_secs: 30
  max_concurrent_requests: 1000

observability:
  log_level: "info"
  enable_metrics: true
  metrics_port: 9091
```

### Worker 0 Config

Create `configs/worker-config-0.yaml`:

```yaml
mode: "worker"
bind_address: "0.0.0.0"
port: 50051

worker:
  worker_id: "worker-0"
  layer_group:
    start_layer: 0
    end_layer: 8
    total_layers: 32
    num_heads: 32
    head_dim: 128
    hidden_dim: 4096
    intermediate_dim: 11008
  next_worker_endpoint: "http://worker-1:50052"
  batching:
    max_batch_size: 16
    batching_window_ms: 5
    max_queue_size: 256
  weights_path: "/home/chatloop/models/partition_0.safetensors"
  worker_threads: 8

observability:
  log_level: "info"
  enable_metrics: true
```

Create similar configs for workers 1, 2, and 3, updating:
- `worker_id`: worker-1, worker-2, worker-3
- `port`: 50052, 50053, 50054
- `start_layer` / `end_layer`: 8-16, 16-24, 24-32
- `next_worker_endpoint`: Point to next worker

## Model Setup

Before running workers, you need model partitions:

### Option 1: Split an Existing Model

```bash
# Install Python dependencies
pip install torch transformers safetensors

# Split a model
python python/model_splitter/split_model.py \
    --model gpt2 \
    --output ./models/partitions \
    --num-partitions 4
```

### Option 2: Use Pre-partitioned Models

Place your model partition files in the `models/` directory:
```
models/
├── partition_0.safetensors
├── partition_1.safetensors
├── partition_2.safetensors
└── partition_3.safetensors
```

## Manual Docker Commands

If you prefer direct docker commands:

### Build Image

```bash
docker build -f docker/all-in-one.Dockerfile -t chatloop-all-in-one:latest .
```

### Run Coordinator

```bash
docker run -d \
  --name chatloop-coordinator \
  -e CHATLOOP_ROLE=coordinator \
  -p 50050:50050 \
  -p 9090:9091 \
  -v $(PWD)/configs/coordinator-config.yaml:/home/chatloop/configs/coordinator-config.yaml:ro \
  chatloop-all-in-one:latest
```

### Run Worker

```bash
docker run -d \
  --name chatloop-worker-0 \
  -e CHATLOOP_ROLE=worker \
  -e CHATLOOP_WORKER_ID=worker-0 \
  -p 50051:50051 \
  -p 9091:9091 \
  -v $(PWD)/models:/home/chatloop/models:ro \
  -v $(PWD)/configs/worker-config-0.yaml:/home/chatloop/configs/worker-config.yaml:ro \
  chatloop-all-in-one:latest
```

## Troubleshooting

### Build Takes Too Long

The first build takes 5-10 minutes. Subsequent builds are faster due to Docker layer caching.

### Container Exits Immediately

Check logs:
```powershell
docker logs chatloop-coordinator
docker logs chatloop-worker-0
```

Common issues:
- Missing config files
- Invalid YAML syntax
- Model files not found

### Port Already in Use

Change the port:
```powershell
$env:CHATLOOP_PORT=50060
.\docker-run.ps1 run-coordinator
```

### Out of Memory

Increase Docker memory limit in Docker Desktop:
1. Open Docker Desktop
2. Go to Settings → Resources
3. Increase memory to at least 8GB

### Can't Access Service from Host

Make sure ports are exposed:
```bash
docker ps
# Check the PORTS column
```

## Advanced Usage

### Custom Config Location

```bash
docker run -d \
  -e CHATLOOP_ROLE=worker \
  -v /path/to/config:/home/chatloop/configs \
  chatloop-all-in-one:latest
```

### Environment Variables

Set environment variables for the container:

```bash
docker run -d \
  -e CHATLOOP_ROLE=worker \
  -e CHATLOOP_LOG_LEVEL=debug \
  -e CHATLOOP_WORKER_THREADS=16 \
  chatloop-all-in-one:latest
```

### Resource Limits

Limit CPU and memory:

```bash
docker run -d \
  --name chatloop-worker-0 \
  --cpus="8" \
  --memory="16g" \
  -e CHATLOOP_ROLE=worker \
  chatloop-all-in-one:latest
```

## Cleanup

### Remove All Containers

```powershell
.\docker-run.ps1 stop-all
```

### Remove Image

```powershell
.\docker-run.ps1 cleanup
```

### Manual Cleanup

```bash
# Stop all containers
docker stop $(docker ps -q --filter "name=chatloop-")

# Remove all containers
docker rm $(docker ps -aq --filter "name=chatloop-")

# Remove image
docker rmi chatloop-all-in-one:latest
```

## Next Steps

1. Verify all services are running: `.\docker-run.ps1 status`
2. Check logs for any errors
3. Test inference (once gRPC is implemented)
4. Scale by adding more workers
5. Monitor metrics at http://localhost:9091/metrics

## Getting Help

- **Logs**: Check container logs first
- **Documentation**: See [README.md](README.md)
- **Issues**: [GitHub Issues](https://github.com/iZoom786/chatloop/issues)

Happy containerizing! 🐳

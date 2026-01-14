# ChatLoop Quick Start Guide

This guide will get you up and running with ChatLoop in under 30 minutes.

## Prerequisites

- Docker and Docker Compose installed
- Python 3.9+ with pip
- At least 16GB RAM (32GB recommended)
- 50GB free disk space

## Step 1: Build the Project

```bash
# Clone the repository
git clone https://github.com/your-org/chatloop.git
cd chatloop

# Install Python dependencies
make install-python-deps

# Build Rust components
make build
```

## Step 2: Split a Model

For this example, we'll use a smaller model. Replace with your preferred model:

```bash
# Split Llama-2-7B into 4 partitions
make split-model MODEL_NAME=meta-llama/Llama-2-7b-hf NUM_PARTITIONS=4
```

This creates:
- `models/partitions/partition_0.safetensors` through `partition_3.safetensors`
- `models/partitions/partition_metadata.json`
- `models/partitions/tokenizer/`

## Step 3: Configure Workers

Create worker configurations for each partition:

```bash
# Worker 0 (layers 0-7)
cat > configs/worker-config-0.yaml << EOF
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
  prev_worker_endpoint: null
  batching:
    max_batch_size: 16
    batching_window_ms: 5
    max_queue_size: 256
    queue_timeout_ms: 1000
  weights_path: "/home/chatloop/models/partition_0.safetensors"
  worker_threads: 8
  enable_cpu_pinning: true
observability:
  log_level: "info"
  enable_metrics: true
  metrics_port: 9091
EOF

# Worker 1 (layers 8-15)
cat > configs/worker-config-1.yaml << EOF
mode: "worker"
bind_address: "0.0.0.0"
port: 50052
worker:
  worker_id: "worker-1"
  layer_group:
    start_layer: 8
    end_layer: 16
    total_layers: 32
    num_heads: 32
    head_dim: 128
    hidden_dim: 4096
    intermediate_dim: 11008
  next_worker_endpoint: "http://worker-2:50053"
  prev_worker_endpoint: "http://worker-0:50051"
  batching:
    max_batch_size: 16
    batching_window_ms: 5
    max_queue_size: 256
    queue_timeout_ms: 1000
  weights_path: "/home/chatloop/models/partition_1.safetensors"
  worker_threads: 8
  enable_cpu_pinning: true
observability:
  log_level: "info"
  enable_metrics: true
  metrics_port: 9091
EOF

# Create similar configs for workers 2 and 3...
```

## Step 4: Build Docker Images

```bash
make docker-build
```

## Step 5: Update docker-compose.yml

Ensure your `docker-compose.yml` has the correct number of workers:

```yaml
services:
  coordinator:
    # ... (as provided)

  worker-0:
    # ... (as provided)

  worker-1:
    # ... (as provided)

  worker-2:
    build:
      context: .
      dockerfile: docker/worker.Dockerfile
    ports:
      - "50053:50053"
      - "9094:9091"
    volumes:
      - ./models:/home/chatloop/models:ro
      - ./configs/worker-config-2.yaml:/home/chatloop/configs/worker-config.yaml
    environment:
      - CHATLOOP_CONFIG=/home/chatloop/configs/worker-config.yaml
      - CHATLOOP_WORKER_ID=worker-2
    networks:
      - chatloop-network
    restart: unless-stopped

  worker-3:
    build:
      context: .
      dockerfile: docker/worker.Dockerfile
    ports:
      - "50054:50054"
      - "9095:9091"
    volumes:
      - ./models:/home/chatloop/models:ro
      - ./configs/worker-config-3.yaml:/home/chatloop/configs/worker-config.yaml
    environment:
      - CHATLOOP_CONFIG=/home/chatloop/configs/worker-config.yaml
      - CHATLOOP_WORKER_ID=worker-3
    networks:
      - chatloop-network
    restart: unless-stopped
```

## Step 6: Start Services

```bash
make docker-compose-up
```

Check logs to ensure services started correctly:

```bash
docker-compose logs -f coordinator
docker-compose logs -f worker-0
```

## Step 7: Send a Test Request

Create a test client:

```python
# test_client.py
import grpc
from inference_pb2_grpc import InferenceServiceStub
from inference_pb2 import InferenceRequest

def test_inference():
    # Connect to coordinator
    channel = grpc.insecure_channel("localhost:50050")
    stub = InferenceServiceStub(channel)

    # Send request
    request = InferenceRequest(
        model_id="llama-2-7b",
        prompt="The future of AI is",
        max_tokens=50,
        temperature=0.8,
        top_p=0.95,
    )

    print("Sending request...")
    response = stub.Inference(request, timeout=60)

    print(f"Generated text: {response.text}")
    print(f"Prompt tokens: {response.prompt_tokens}")
    print(f"Completion tokens: {response.completion_tokens}")
    print(f"Total time: {response.total_duration_ms}ms")

if __name__ == "__main__":
    test_inference()
```

Run the client:

```bash
pip install grpcio grpcio-tools
python test_client.py
```

## Step 8: Check Metrics

Access Prometheus metrics:

```bash
# Coordinator metrics
curl http://localhost:9091/metrics

# Worker 0 metrics
curl http://localhost:9092/metrics
```

## Troubleshooting

### Workers not connecting

Check network connectivity:
```bash
docker-compose exec coordinator ping worker-0
```

### Out of memory errors

Reduce batch size in worker configs:
```yaml
batching:
  max_batch_size: 8
```

### Slow inference

1. Check CPU utilization:
```bash
docker exec chatloop-worker-0 top -b -n 1
```

2. Enable CPU pinning in worker configs

3. Reduce batching window for lower latency

## Next Steps

1. **Scale Up**: Add more workers for higher throughput
2. **Quantization**: Use INT8 quantization to reduce memory
   ```bash
   make split-model-int8
   ```
3. **Monitoring**: Set up Prometheus and Grafana
4. **Production**: Deploy to Kubernetes or YARN

## Production Tips

- Use 10 Gbps networking between workers
- Pin workers to dedicated CPU cores
- Use NVMe storage for model weights
- Enable NUMA binding for multi-socket systems
- Monitor queue depths and adjust batch sizes

## Getting Help

- Full documentation: [README.md](README.md)
- Issues: [GitHub Issues](https://github.com/your-org/chatloop/issues)
- Discussions: [GitHub Discussions](https://github.com/your-org/chatloop/discussions)

Happy inference! 🚀

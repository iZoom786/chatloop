# ChatLoop: Distributed CPU-Only LLM Inference Platform

ChatLoop is a production-grade, horizontally scalable LLM inference platform designed for CPU-only environments. It uses pipeline parallelism across worker nodes, with model weights partitioned and memory-mapped for optimal performance.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Key Features](#key-features)
- [System Requirements](#system-requirements)
- [Quick Start](#quick-start)
- [Architecture Deep Dive](#architecture-deep-dive)
- [Deployment](#deployment)
- [Performance Tuning](#performance-tuning)
- [Monitoring and Observability](#monitoring-and-observability)
- [Troubleshooting](#troubleshooting)

## Architecture Overview

ChatLoop consists of three main components:

1. **Inference Workers**: Rust-based services that load model partitions and execute forward passes
2. **Coordinator**: Stateless router that distributes requests to workers
3. **Model Splitter**: Python tooling that partitions models into layer groups

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│  Coordinator    │ ◄─── Health Checks
│  (Router)       │
└────────┬────────┘
         │
    ┌────┴────┬────────┬────────┐
    ▼         ▼        ▼        ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│Worker0│ │Worker1│ │Worker2│ │Worker3│
│Layers │ │Layers │ │Layers │ │Layers │
│ 0-7   │→│ 8-15  │→│ 16-23 │→│ 24-31 │
└───────┘ └───────┘ └───────┘ └───────┘
   mmap       mmap       mmap       mmap
```

## Key Features

### Core Capabilities

- **CPU-Only Design**: No GPUs required, optimized for CPU inference
- **Horizontal Scalability**: Add workers to increase throughput
- **Pipeline Parallelism**: Model split across workers for efficient execution
- **Memory-Mapped Weights**: Zero-copy access, OS-managed caching
- **Token-Level Batching**: 2-5ms batching window for low latency
- **Fault Tolerance**: Workers can fail without crashing the system
- **Production-Ready**: Metrics, logging, health checks, and graceful shutdown

### Performance Features

- **SIMD Optimization**: AVX2/AVX-512 support for faster computation
- **Lock-Free Queues**: Minimized contention in the hot path
- **CPU Pinning**: Bind workers to specific cores for cache locality
- **NUMA Awareness**: Optimize memory allocation for multi-socket systems
- **Quantization Support**: INT8/INT4 quantization for reduced memory usage

### Operational Features

- **Docker Containers**: Easy deployment and isolation
- **YARN Integration**: Resource management and lifecycle supervision
- **Prometheus Metrics**: Built-in observability
- **Structured Logging**: JSON logs with tracing support
- **Health Checks**: Automatic failure detection and recovery

## System Requirements

### Minimum Requirements

- **CPU**: x86_64 with AVX2 support
- **Memory**: 4 GB per worker (depends on model partition size)
- **Storage**: 10 GB for model weights (depends on model)
- **Network**: 1 Gbps (10 Gbps recommended)
- **OS**: Linux (kernel 5.4+)

### Recommended for Production

- **CPU**: 16+ cores per worker, modern Xeon Scalable or AMD EPYC
- **Memory**: 32+ GB per worker
- **Network**: 10 Gbps or faster
- **Storage**: SSD/NVMe for model weights

### Software Dependencies

- **Rust**: 1.75+
- **Docker**: 20.10+
- **Hadoop YARN**: 3.3+ (for orchestration)
- **Python**: 3.9+ (for model splitting tool)

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/chatloop.git
cd chatloop
```

### 2. Build the Project

```bash
cargo build --release
```

### 3. Split a Model

```bash
# Install Python dependencies
pip install -r python/model_splitter/requirements.txt

# Split a model into 4 partitions
python python/model_splitter/split_model.py \
    --model meta-llama/Llama-2-7b-hf \
    --output ./models/llama-2-7b-partitions \
    --num-partitions 4
```

This creates:
- `partition_0.safetensors`, `partition_1.safetensors`, etc.
- `partition_metadata.json` with layer group information
- `tokenizer/` directory with tokenizer files

### 4. Configure Workers

Edit `configs/worker-config.yaml`:

```yaml
worker:
  worker_id: "worker-0"
  layer_group:
    start_layer: 0
    end_layer: 8
    # ... other layer config ...
  weights_path: "/models/partition_0.safetensors"
```

Create separate configs for each worker (0 to N-1).

### 5. Build Docker Images

```bash
# Build base image
docker build -f docker/base.Dockerfile -t chatloop-base:latest .

# Build worker image
docker build -f docker/worker.Dockerfile -t chatloop-worker:latest .

# Build coordinator image
docker build -f docker/coordinator.Dockerfile -t chatloop-coordinator:latest .
```

### 6. Deploy with YARN

```bash
# Deploy workers
yarn app -install chatloop-worker
yarn app -start chatloop-worker -Dworker.id=0 -Dworker.port=50051
yarn app -start chatloop-worker -Dworker.id=1 -Dworker.port=50052
# ... etc for all workers

# Deploy coordinator
yarn app -install chatloop-coordinator
yarn app -start chatloop-coordinator
```

### 7. Send Inference Requests

```python
import grpc
from inference_pb2_grpc import InferenceServiceStub
from inference_pb2 import InferenceRequest

# Connect to coordinator
channel = grpc.insecure_channel("coordinator:50050")
stub = InferenceServiceStub(channel)

# Send request
request = InferenceRequest(
    model_id="llama-2-7b",
    prompt="Explain quantum computing in simple terms.",
    max_tokens=100,
    temperature=0.7,
)

response = stub.Inference(request)
print(response.text)
```

## Architecture Deep Dive

### Pipeline Parallelism

ChatLoop splits the model into layer groups, each handled by a different worker:

```
Input Tokens
    │
    ▼
┌─────────┐
│ Embed   │
└────┬────┘
     │
     ▼
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│ Layer 0 │ ──→ │ Layer 1 │ ──→ │ Layer 2 │ ──→ │ Layer 3 │
│ Layer 1 │     │ Layer 2 │     │ Layer 3 │     │ Layer 4 │
│ ...     │     │ ...     │     │ ...     │     │ ...     │
│ Layer 7 │     │ Layer 15│     │ Layer 23│     │ Layer 31│
└────┬────┘     └────┬────┘     └────┬────┘     └────┬────┘
     │               │               │               │
     └───────────────┴───────────────┴───────────────┘
                              │
                              ▼
                         ┌─────────┐
                         │  LMHead │
                         └────┬────┘
                              │
                              ▼
                         Output Tokens
```

Each worker:
1. Receives hidden states from previous worker
2. Processes its layer group
3. Passes results to next worker
4. For final worker, generates output tokens

### Token-Level Batching

The batching window balances throughput and latency:

```
Time ────────────────────────────────────────►

Request 1:  ████                (processed alone)
Request 2:      ████             (batched with 3)
Request 3:         ████          (batched with 4)
Request 4:            ████       (batched with 5)
Request 5:               ████    (batched with 6)
Request 6:                  ████

┌────────────────┐
│ Batching Window│ (2-5 ms)
└────────────────┘
```

Benefits:
- Low latency: Small window ensures fast processing
- High throughput: Batches multiple requests when they arrive close together
- Backpressure: Rejects requests when queue is full

### Memory-Mapped Weights

Model weights are memory-mapped for efficient access:

```
File: partition_0.safetensors (10 GB)
┌──────────────────────────────────┐
│ Header (JSON)                    │
├──────────────────────────────────┤
│ Tensor 0: model.layers.0.weight  │ ◄────┐
├──────────────────────────────────┤      │
│ Tensor 1: model.layers.1.weight  │ ◄──┐ │
├──────────────────────────────────┤    │ │
│ Tensor 2: model.layers.2.weight  │ ◄─┐ │ │
├──────────────────────────────────┤  │ │ │
│ ...                              │  │ │ │
└──────────────────────────────────┘  │ │ │
                                       │ │ │
            Memory Map                │ │ │
            (Virtual Memory)          │ │ │
                                       │ │ │
┌──────────────────────────────────┐  │ │ │
│ RAM (2 GB loaded)                │  │ │ │
│ ├─ Tensor 0 (page 1-10)          │ ◄┘ │ │
│ ├─ Tensor 2 (page 50-100)        │ ◄──┘ │
│ └─ Tensor 5 (page 200-250)       │ ◄────┘
└──────────────────────────────────┘

Key Benefits:
- Zero-copy access
- OS-managed caching
- Fast startup (no loading time)
- Shared memory across processes
```

### Fault Tolerance

ChatLoop handles failures gracefully:

```
┌─────────────┐
│ Coordinator │
└──────┬──────┘
       │
       ├──────────────┐
       │              │
       ▼              ▼
  ┌─────────┐    ┌─────────┐
  │Worker 0 │    │Worker 1 │◄─── FAILURE!
  │Healthy  │    │Unhealthy│
  └─────────┘    └─────────┘
       │              │
       │              └─────┐
       │                    │
       ▼                    ▼
  Request 1          Request 2
  (to worker 0)      (rerouted to worker 0)
```

Failure Handling:
1. Health check detects failure
2. Router marks worker unhealthy
3. Requests rerouted to healthy workers
4. YARN restarts failed container
5. Worker rejoins after restart

## Deployment

### Docker Compose (Development)

```yaml
version: '3.8'
services:
  coordinator:
    image: chatloop-coordinator:latest
    ports:
      - "50050:50050"
      - "9091:9091"
    volumes:
      - ./configs/coordinator-config.yaml:/home/chatloop/configs/coordinator-config.yaml

  worker-0:
    image: chatloop-worker:latest
    ports:
      - "50051:50051"
      - "9092:9091"
    environment:
      - CHATLOOP_WORKER_ID=worker-0
    volumes:
      - ./models/partition_0.safetensors:/home/chatloop/models/partition_0.safetensors
      - ./configs/worker-config-0.yaml:/home/chatloop/configs/worker-config.yaml

  # Add more workers...
```

### Kubernetes (Production)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: chatloop-worker-0
spec:
  replicas: 1
  selector:
    matchLabels:
      app: chatloop-worker-0
  template:
    metadata:
      labels:
        app: chatloop-worker-0
    spec:
      containers:
      - name: worker
        image: chatloop-worker:latest
        ports:
        - containerPort: 50051
        resources:
          requests:
            memory: "16Gi"
            cpu: "8"
          limits:
            memory: "32Gi"
            cpu: "16"
        volumeMounts:
        - name: model
          mountPath: /home/chatloop/models
      volumes:
      - name: model
        hostPath:
          path: /mnt/models/partition_0.safetensors
```

### YARN (Production)

See [yarn/worker-service.xml](yarn/worker-service.xml) and [yarn/coordinator-service.xml](yarn/coordinator-service.xml) for full configuration.

## Performance Tuning

### CPU Optimization

1. **Enable CPU Pinning** in worker config:
   ```yaml
   worker:
     enable_cpu_pinning: true
     cpu_cores: "0-15"  # Pin to cores 0-15
   ```

2. **Use NUMA Binding** for multi-socket systems:
   ```yaml
   worker:
     numa_node: 0  # Bind to NUMA node 0
   ```

3. **Enable SIMD** (enabled by default):
   ```yaml
   performance:
     enable_simd: true
   ```

### Memory Optimization

1. **Adjust KV Cache Size**:
   ```yaml
   performance:
     kv_cache_mb: 1024  # Increase for longer sequences
   ```

2. **Use Quantization**:
   ```bash
   python split_model.py --quantization int8 ...
   ```

3. **Preallocate Activations**:
   ```yaml
   performance:
     preallocate_activations: true
   ```

### Batching Tuning

```yaml
worker:
  batching:
    max_batch_size: 32          # Increase for higher throughput
    batching_window_ms: 5       # Decrease for lower latency
    max_queue_size: 512         # Increase for higher burst capacity
```

Trade-offs:
- Larger batch size = higher throughput, higher latency
- Larger batching window = higher throughput, higher latency
- Smaller values = lower latency, lower throughput

### Network Optimization

1. **Use gRPC Compression**:
   ```yaml
   grpc:
     compression: "gzip"
   ```

2. **Increase Buffer Sizes**:
   ```yaml
   grpc:
     max_receive_message_length: 134217728  # 128 MB
   ```

## Monitoring and Observability

### Prometheus Metrics

All components expose Prometheus metrics on port 9091:

**Worker Metrics:**
- `worker_forward_duration_seconds` - Forward pass latency
- `worker_queue_time_seconds` - Time spent in queue
- `worker_queue_depth` - Current queue depth
- `worker_batch_size` - Batch size distribution
- `worker_cpu_utilization_percent` - CPU usage
- `worker_memory_used_bytes` - Memory usage

**Coordinator Metrics:**
- `coordinator_requests_routed_total` - Total requests routed
- `coordinator_active_workers` - Healthy worker count
- `coordinator_unhealthy_workers` - Unhealthy worker count
- `coordinator_worker_response_time_seconds` - Worker response time

**Inference Metrics:**
- `inference_requests_total` - Total requests
- `inference_request_duration_seconds` - End-to-end latency
- `inference_tokens_generated_total` - Total tokens generated
- `inference_tokens_per_second` - Throughput

Example Grafana dashboard queries:
```promql
# Request rate
rate(inference_requests_total[5m])

# P95 latency
histogram_quantile(0.95, rate(inference_request_duration_seconds_bucket[5m]))

# Tokens per second
rate(inference_tokens_generated_total[5m])

# Worker health
count(coordinator_active_workers) by (worker_id)
```

### Logging

Logs are structured JSON with tracing:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "info",
  "target": "chatloop_worker::batching",
  "message": "Created batch: 4 requests, max_seq_len: 128, age: 4.2ms",
  "span": {
    "request_id": "req-123",
    "trace_id": "trace-456"
  }
}
```

Enable debug logging:
```yaml
observability:
  log_level: "debug"
```

### Distributed Tracing

Enable OpenTelemetry tracing:
```yaml
observability:
  otel_endpoint: "http://jaeger:4317"
```

## Troubleshooting

### Worker Not Starting

**Symptoms**: Worker exits immediately after starting

**Solutions**:
1. Check model path exists:
   ```bash
   ls -lh /home/chatloop/models/partition_0.safetensors
   ```

2. Check configuration syntax:
   ```bash
   python -c "import yaml; yaml.safe_load(open('configs/worker-config.yaml'))"
   ```

3. Check logs:
   ```bash
   docker logs chatloop-worker-0
   ```

### High Latency

**Symptoms**: Requests taking >100ms

**Solutions**:
1. Check queue depth:
   ```bash
   curl http://worker:9091/metrics | grep worker_queue_depth
   ```

2. Reduce batching window:
   ```yaml
   batching_window_ms: 2
   ```

3. Check CPU utilization:
   ```bash
   curl http://worker:9091/metrics | grep worker_cpu_utilization
   ```

4. Enable CPU pinning if not already enabled

### Out of Memory

**Symptoms**: Worker killed with OOM

**Solutions**:
1. Reduce KV cache size:
   ```yaml
   kv_cache_mb: 256
   ```

2. Use quantization (INT8/INT4)

3. Reduce batch size:
   ```yaml
   max_batch_size: 16
   ```

4. Increase container memory limit

### Worker Failures

**Symptoms**: Workers marked unhealthy frequently

**Solutions**:
1. Check health check interval:
   ```yaml
   health_check_interval_secs: 10
   ```

2. Increase failure threshold:
   ```yaml
   failure_threshold: 5
   ```

3. Check worker logs for errors:
   ```bash
   docker logs chatloop-worker-0 --tail 100
   ```

4. Verify network connectivity between coordinator and workers

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Candle for CPU tensor operations
- HuggingFace Transformers for model loading
- Safetensors for efficient weight serialization
- Tokio for async runtime
- Tonic for gRPC framework

## Support

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/your-org/chatloop/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/chatloop/discussions)

---

**ChatLoop** - Production-grade, CPU-only distributed LLM inference.

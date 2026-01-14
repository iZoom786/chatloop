# ChatLoop Quick Start Guide

This guide will get you up and running with ChatLoop in under 30 minutes.

## Prerequisites

- Rust 1.75+ (install from https://rustup.rs/)
- Python 3.9+ with pip
- Docker and Docker Compose (optional, for containerized deployment)
- At least 16GB RAM (32GB recommended)
- 50GB free disk space

## Step 1: Install Rust

### Windows
```powershell
# Download from https://rustup.rs/ or use winget:
winget install Rustlang.Rustup

# Restart terminal and verify:
rustc --version
cargo --version
```

### Linux/macOS
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify:
rustc --version
cargo --version
```

## Step 2: Install Python Dependencies

```bash
pip install torch transformers safetensors accelerate
```

## Step 3: Build ChatLoop

### Windows (PowerShell)
```powershell
cd E:\chatloop
.\build.ps1 build
```

### Linux/macOS
```bash
cd chatloop
cargo build --release
```

This will take 5-10 minutes on the first build as it downloads and compiles dependencies.

## Step 4: Split a Model

For this example, we'll use a smaller model. You can replace with your preferred model:

```bash
python python/model_splitter/split_model.py \
    --model meta-llama/Llama-2-7b-hf \
    --output ./models/partitions \
    --num-partitions 4
```

This creates:
- `models/partitions/partition_0.safetensors` through `partition_3.safetensors`
- `models/partitions/partition_metadata.json`
- `models/partitions/tokenizer/`

## Step 5: Configure Workers

Create worker configurations for each partition. Here's an example for Worker 0:

**`configs/worker-config-0.yaml`:**
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
  weights_path: "/models/partition_0.safetensors"
  worker_threads: 8

observability:
  log_level: "info"
  enable_metrics: true
```

Create similar configs for workers 1, 2, and 3, updating:
- `worker_id`: worker-1, worker-2, worker-3
- `port`: 50052, 50053, 50054
- `start_layer` / `end_layer`: 8-16, 16-24, 24-32
- `next_worker_endpoint`: Point to next worker (or null for last worker)

## Step 6: Build Docker Images (Optional)

```bash
# Build base image
docker build -f docker/base.Dockerfile -t chatloop-base:latest .

# Build worker image
docker build -f docker/worker.Dockerfile -t chatloop-worker:latest .

# Build coordinator image
docker build -f docker/coordinator.Dockerfile -t chatloop-coordinator:latest .
```

## Step 7: Start Services

### Using Docker Compose (Recommended for Development)

```bash
docker-compose up -d
```

### Running Directly (Development)

```bash
# Terminal 1: Start coordinator
cargo run --release -p chatloop-coordinator

# Terminal 2: Start worker 0
cargo run --release -p chatloop-worker

# Terminal 3, 4, 5: Start other workers
```

## Step 8: Verify Installation

Check that services are running:

```bash
# Check logs
docker-compose logs -f coordinator
docker-compose logs -f worker-0

# Check metrics
curl http://localhost:9091/metrics
```

## Common Issues

### Build Errors

**Error: `cargo` not found**
- Install Rust from https://rustup.rs/
- Restart your terminal

**Error: `make` not found (Windows)**
- Use `.\build.ps1` instead of `make`
- Or install make: `choco install make`

### Runtime Errors

**Worker fails to start**
- Check that model partitions exist
- Verify configuration YAML syntax
- Check logs for specific errors

**Out of memory**
- Reduce batch size in config
- Reduce KV cache size
- Use fewer workers

### Model Download Issues

**HuggingFace authentication**
```bash
pip install huggingface-hub
huggingface-cli login
```

**Model not found**
- Verify model name is correct
- Check you have access to the model
- Try a public model like `gpt2` for testing

## Next Steps

1. **Scale Up**: Add more workers for higher throughput
2. **Quantization**: Use INT8 to reduce memory usage
   ```bash
   python python/model_splitter/split_model.py --quantization int8 ...
   ```
3. **Monitoring**: Set up Prometheus and Grafana for metrics
4. **Production**: Deploy to Kubernetes or YARN

## Testing

Once services are running, you can test inference:

```python
# Create a simple test client
# test_client.py

import requests

# Send a test request to the coordinator
response = requests.post("http://localhost:50050/inference", json={
    "model_id": "llama-2-7b",
    "prompt": "Hello, how are you?",
    "max_tokens": 50,
    "temperature": 0.7
})

print(response.json())
```

## Getting Help

- **Full Documentation**: See [README.md](README.md)
- **Issues**: [GitHub Issues](https://github.com/iZoom786/chatloop/issues)
- **Troubleshooting**: Check the [Troubleshooting](#troubleshooting) section

Happy inferencing! 🚀

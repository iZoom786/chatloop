# ChatLoop: Distributed CPU-Only LLM Inference Platform

ChatLoop is a production-grade, horizontally scalable LLM inference platform designed for CPU-only environments. It uses pipeline parallelism across worker nodes, with model weights partitioned and memory-mapped for optimal performance.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Key Features](#key-features)
- [System Requirements](#system-requirements)
- [Quick Start](#quick-start)
- [Project Status](#project-status)
- [Architecture Deep Dive](#architecture-deep-dive)
- [Development](#development)
- [Deployment](#deployment)
- [Performance Tuning](#performance-tuning)
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
- **OS**: Linux, macOS, or Windows 10+

### Recommended for Production

- **CPU**: 16+ cores per worker, modern Xeon Scalable or AMD EPYC
- **Memory**: 32+ GB per worker
- **Network**: 10 Gbps or faster
- **Storage**: SSD/NVMe for model weights

### Software Dependencies

- **Rust**: 1.75+ (install from https://rustup.rs/)
- **Docker**: 20.10+ (optional, for containerized deployment)
- **Hadoop YARN**: 3.3+ (optional, for orchestration)
- **Python**: 3.9+ (for model splitting tool)

## Quick Start

### Option 1: Docker (Recommended - No Rust Installation Required)

The easiest way to get started is using Docker. You don't need to install Rust on your machine.

#### 1. Install Docker Desktop

- **Windows**: Download from [docker.com](https://www.docker.com/products/docker-desktop/)
- **Linux**: Install via package manager (see Docker docs)
- **macOS**: Download from [docker.com](https://www.docker.com/products/docker-desktop/)

#### 2. Build the Docker Image

**Windows (PowerShell):**
```powershell
# Build the all-in-one image (includes both coordinator and worker)
.\docker-run.ps1 build
```

**Linux/macOS:**
```bash
# Build the all-in-one image
chmod +x docker-run.sh
./docker-run.sh build
```

**Note:** First build takes 10-20 minutes as it compiles Rust code and downloads dependencies.

**Build Troubleshooting:** If the build fails, see [DOCKER_BUILD_TROUBLESHOOTING.md](DOCKER_BUILD_TROUBLESHOOTING.md) for detailed troubleshooting steps.

#### 3. Run Coordinator

```powershell
# Windows
.\docker-run.ps1 run-coordinator

# Linux/macOS
./docker-run.sh run-coordinator
```

#### 4. Run Workers

```powershell
# Windows - Start 4 workers (each handles 8 layers)
.\docker-run.ps1 run-worker 0
.\docker-run.ps1 run-worker 1
.\docker-run.ps1 run-worker 2
.\docker-run.ps1 run-worker 3

# Linux/macOS
./docker-run.sh run-worker 0
./docker-run.sh run-worker 1
./docker-run.sh run-worker 2
./docker-run.sh run-worker 3
```

#### 5. Check Status

```powershell
# Windows
.\docker-run.ps1 status

# Linux/macOS
./docker-run.sh status
```

For complete Docker documentation, see:
- [DOCKER_GUIDE.md](DOCKER_GUIDE.md) - Complete Docker setup guide
- [DOCKER_BUILD_TROUBLESHOOTING.md](DOCKER_BUILD_TROUBLESHOOTING.md) - Build troubleshooting
- [MODEL_SETUP_GUIDE.md](MODEL_SETUP_GUIDE.md) - Where to place model files
- [WORKER_CONFIGS.md](WORKER_CONFIGS.md) - Worker configuration details

### Option 2: Local Build (Requires Rust Installation)

#### 1. Install Rust

**Windows:**
```powershell
# Download and run rustup-init.exe from https://rustup.rs/
# Or use winget:
winget install Rustlang.Rustup

# Restart terminal and verify:
rustc --version
cargo --version
```

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 2. Install Python Dependencies (for model splitting)

```bash
pip install torch transformers safetensors accelerate
```

#### 3. Build the Project

**Windows (PowerShell):**
```powershell
# Use the build script
.\build.ps1 build

# Or use cargo directly
cargo build --release
```

**Linux/macOS:**
```bash
# Use make (if installed)
make build

# Or use cargo directly
cargo build --release
```

### Split a Model

```bash
python python/model_splitter/split_model.py \
    --model meta-llama/Llama-2-7b-hf \
    --output ./models/partitions \
    --num-partitions 4
```

This creates:
- `partition_0.safetensors`, `partition_1.safetensors`, etc.
- `partition_metadata.json` with layer group information
- `tokenizer/` directory with tokenizer files

### Run with Docker Compose (Development)

```bash
# Build Docker images
docker build -f docker/base.Dockerfile -t chatloop-base:latest .
docker build -f docker/worker.Dockerfile -t chatloop-worker:latest .
docker build -f docker/coordinator.Dockerfile -t chatloop-coordinator:latest .

# Start services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

## Project Status

### ✅ Implemented

- **Core Architecture**: Modular Rust workspace with worker, coordinator, and common crates
- **Configuration System**: YAML-based configuration with environment variable overrides
- **Error Handling**: Comprehensive error types with gRPC status conversion
- **Metrics**: Prometheus metrics for observability
- **Tensor Operations**: SIMD-friendly matrix operations with quantization support
- **Memory-Mapped Weights**: Zero-copy Safetensors loading
- **Token-Level Batching**: Lock-free batching with configurable window (2-5ms)
- **KV Cache Management**: Efficient caching for autoregressive generation
- **Model Splitter**: Python tooling for partitioning HuggingFace models
- **Docker Support**: Container images for workers and coordinator
- **YARN Integration**: Service definitions and launch scripts

### 🚧 In Progress

- **gRPC Communication**: Protocol buffers defined (simplified implementation in place)
- **Inference Engine**: Core logic implemented (needs testing and optimization)
- **Load Balancing**: Router logic implemented (needs integration testing)

### 📋 TODO

- Complete gRPC server/client implementation
- Add comprehensive unit tests
- Add integration tests
- Performance benchmarks
- Complete documentation
- Add example clients (Python, JavaScript, etc.)
- Kubernetes deployment manifests

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

### Token-Level Batching

```
Time ───────────────────────────────────────►

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

### Memory-Mapped Weights

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
```

## Development

### Project Structure

```
chatloop/
├── Cargo.toml                    # Rust workspace
├── build.ps1                     # Windows build script
├── Makefile                      # Unix build script
├── docker-compose.yml            # Dev deployment
├── crates/
│   ├── proto/                    # Protocol definitions (simplified)
│   ├── common/                   # Shared code
│   ├── worker/                   # Inference worker
│   └── coordinator/              # Router
├── python/model_splitter/        # Model splitting tool
├── docker/                       # Docker images
├── yarn/                         # YARN integration
├── configs/                      # Configuration files
├── docs/                         # Additional documentation
├── README.md                     # This file
└── QUICKSTART.md                 # Quick start guide
```

### Building

**Windows (PowerShell):**
```powershell
.\build.ps1 build          # Build release
.\build.ps1 test           # Run tests
.\build.ps1 clean          # Clean artifacts
```

**Linux/macOS:**
```bash
make build                 # Build release
make test                  # Run tests
make clean                 # Clean artifacts
```

**Cross-platform:**
```bash
cargo build --release      # Build all crates
cargo test --all           # Run all tests
cargo clean                # Clean build artifacts
```

### Running Tests

```bash
# Run all tests
cargo test --all

# Run tests for specific crate
cargo test -p chatloop-common
cargo test -p chatloop-worker

# Run tests with output
cargo test -- --nocapture
```

## Deployment

### Docker

Build images:
```bash
docker build -f docker/base.Dockerfile -t chatloop-base:latest .
docker build -f docker/worker.Dockerfile -t chatloop-worker:latest .
docker build -f docker/coordinator.Dockerfile -t chatloop-coordinator:latest .
```

Run with docker-compose:
```bash
docker-compose up -d
```

### Kubernetes

See `configs/` for example Kubernetes manifests (TODO)

### YARN

```bash
# Deploy workers
yarn app -install chatloop-worker
yarn app -start chatloop-worker -Dworker.id=0

# Deploy coordinator
yarn app -install chatloop-coordinator
yarn app -start chatloop-coordinator
```

## Performance Tuning

### CPU Optimization

- Enable CPU pinning in worker config
- Use NUMA binding for multi-socket systems
- Enable SIMD (enabled by default)

### Memory Optimization

- Adjust KV cache size
- Use quantization (INT8/INT4)
- Preallocate activations

### Batching Tuning

```yaml
batching:
  max_batch_size: 32          # Increase for higher throughput
  batching_window_ms: 5       # Decrease for lower latency
  max_queue_size: 512         # Increase for higher burst capacity
```

## Troubleshooting

### Build Issues

**Error: `cargo` not found**
- Install Rust from https://rustup.rs/
- Restart your terminal after installation

**Error: `make` not found (Windows)**
- Use `.\build.ps1` instead
- Or install make via Chocolatey: `choco install make`

### Runtime Issues

**Worker not starting**
- Check model path exists
- Check configuration syntax
- View logs: `docker logs chatloop-worker-0`

**High latency**
- Check queue depth metrics
- Reduce batching window
- Enable CPU pinning

**Out of memory**
- Reduce KV cache size
- Use quantization
- Reduce batch size

## Contributing

Contributions are welcome! Please see:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Candle for CPU tensor operations
- HuggingFace Transformers for model loading
- Safetensors for efficient weight serialization
- Tokio for async runtime
- Tonic for gRPC framework

## Support

- **Documentation**: See [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/iZoom786/chatloop/issues)
- **Discussions**: [GitHub Discussions](https://github.com/iZoom786/chatloop/discussions)

---

**ChatLoop** - Production-grade, CPU-only distributed LLM inference.

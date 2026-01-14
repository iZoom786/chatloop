# ChatLoop Development Guide

This guide is for developers who want to contribute to or extend ChatLoop.

## Development Setup

### 1. Clone and Build

```bash
# Clone the repository
git clone https://github.com/iZoom786/chatloop.git
cd chatloop

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build --release
```

### 2. Run Tests

```bash
# Run all tests
cargo test --all

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_matmul
```

### 3. Development Workflow

```bash
# Watch mode (requires cargo-watch)
cargo watch -x build
cargo watch -x test

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Check without building
cargo check
```

## Project Structure

```
chatloop/
├── Cargo.toml                    # Workspace configuration
├── build.ps1                     # Windows build script
├── Makefile                      # Unix build script
│
├── crates/                       # Rust workspace members
│   ├── proto/                    # Protocol buffer definitions
│   │   ├── src/
│   │   │   └── lib.rs           # Type definitions (simplified)
│   │   ├── inference.proto       # gRPC service definitions
│   │   └── worker.proto
│   │
│   ├── common/                   # Shared library
│   │   └── src/
│   │       ├── config.rs         # Configuration structures
│   │       ├── error.rs          # Error types
│   │       ├── metrics.rs        # Prometheus metrics
│   │       └── lib.rs
│   │
│   ├── worker/                   # Inference worker
│   │   └── src/
│   │       ├── main.rs           # Worker entry point
│   │       ├── model.rs          # Model partition loading
│   │       ├── inference.rs      # Forward pass execution
│   │       ├── batching.rs       # Request batching
│   │       ├── tensor/
│   │       │   ├── mod.rs        # Tensor types
│   │       │   ├── safetensors.rs # Memory-mapped weights
│   │       │   └── ops.rs        # Tensor operations
│   │       └── lib.rs
│   │
│   └── coordinator/              # Request router
│       └── src/
│           ├── main.rs           # Coordinator entry point
│           ├── router.rs         # Load balancing logic
│           ├── worker_client.rs  # Worker communication
│           └── lib.rs
│
├── python/model_splitter/        # Python tooling
│   ├── split_model.py            # Model partitioning script
│   └── requirements.txt
│
├── docker/                       # Docker configurations
│   ├── base.Dockerfile
│   ├── worker.Dockerfile
│   └── coordinator.Dockerfile
│
├── yarn/                         # Hadoop YARN integration
│   ├── worker-service.xml
│   ├── coordinator-service.xml
│   └── launch-scripts/
│
└── configs/                      # Configuration files
    ├── worker-config.yaml
    └── coordinator-config.yaml
```

## Architecture Components

### 1. Worker (`crates/worker/`)

The worker is responsible for:
- Loading a model partition (subset of layers)
- Processing forward passes for those layers
- Batching requests for efficiency
- Managing KV cache for sequences

**Key files:**
- `model.rs`: Memory-mapped model loading
- `inference.rs`: Forward pass execution
- `batching.rs`: Token-level request batching
- `tensor/ops.rs`: SIMD-friendly tensor operations

### 2. Coordinator (`crates/coordinator/`)

The coordinator handles:
- Request routing to workers
- Load balancing based on queue depth
- Health checking workers
- Failover handling

**Key files:**
- `router.rs`: Request routing logic
- `worker_client.rs`: Worker communication

### 3. Common (`crates/common/`)

Shared utilities:
- Configuration management
- Error handling
- Metrics collection

## Adding New Features

### Adding a New Tensor Operation

1. Edit `crates/worker/src/tensor/ops.rs`:

```rust
pub fn my_operation<T: TensorOps<T>>(
    tensor: &TensorView<'_, T>,
    param: T,
) -> Result<Tensor<T>> {
    // Implementation
}
```

2. Add tests:

```rust
#[test]
fn test_my_operation() {
    let data = vec![1.0f32, 2.0, 3.0, 4.0];
    let tensor = TensorView::new(&data, vec![2, 2]);

    let result = f32::my_operation(&tensor, 2.0).unwrap();

    assert_eq!(result.data, vec![2.0, 4.0, 6.0, 8.0]);
}
```

### Adding a New Configuration Option

1. Edit `crates/common/src/config.rs`:

```rust
pub struct MyConfig {
    pub my_option: String,
}
```

2. Update configuration loading:

```rust
pub fn from_file<P: Into<PathBuf>>(path: P) -> Result<Self> {
    let config: ChatLoopConfig = serde_yaml::from_str(&content)?;
    config.validate()?;
    Ok(config)
}
```

### Adding Metrics

Edit `crates/common/src/metrics.rs`:

```rust
pub struct MyMetrics {
    pub my_counter: IntCounter,
    pub my_gauge: IntGauge,
    pub my_histogram: Histogram,
}

// In MetricsRegistry::new()
let my_counter = IntCounter::new(
    "my_metric_total",
    "My metric description"
).unwrap();

registry.register(Box::new(my_counter.clone())).unwrap();
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

### Integration Tests

Create `crates/worker/tests/integration_test.rs`:

```rust
use chatloop_worker::ModelPartition;

#[tokio::test]
async fn test_end_to_end() {
    // Setup
    let model = ModelPartition::load("test.safetensors", config).unwrap();

    // Test
    let result = model.forward(&input).unwrap();

    // Assert
    assert_eq!(result.len(), expected_len);
}
```

### Running Benchmarks

```bash
# Install criterion
cargo install cargo-criterion

# Run benchmarks
cargo criterion
```

## Debugging

### Enable Debug Logging

Set environment variable:
```bash
RUST_LOG=debug cargo run --release
```

Or in code:
```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

### Using lldb/gdb

```bash
# Build with debug symbols
cargo build

# Run with debugger
lldb target/debug/chatloop-worker
```

### Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin chatloop-worker
```

## Code Style

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Linting

```bash
# Run clippy
cargo clippy -- -D warnings

# Fix issues automatically
cargo clippy --fix -- -D warnings
```

## Performance Optimization

### Profile-Guided Optimization (PGO)

```bash
# Step 1: Build with profiling instrumentation
cargo build --release --profile=profiling

# Step 2: Run typical workload
./target/profiling/chatloop-worker

# Step 3: Build with PGO data
cargo build --release
```

### Benchmarking

Use criterion for microbenchmarks:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_matmul(c: &mut Criterion) {
    c.bench_function("matmul_2x2", |b| {
        b.iter(|| {
            let a = TensorView::new(&data_a, vec![2, 2]);
            let b = TensorView::new(&data_b, vec![2, 2]);
            f32::matmul(black_box(&a), black_box(&b))
        });
    });
}

criterion_group!(benches, benchmark_matmul);
criterion_main!(benches);
```

## Releasing

### Version Bump

Edit `Cargo.toml`:
```toml
[workspace.package]
version = "0.2.0"  # Bump version
```

### Create Release

```bash
# Run full test suite
cargo test --all

# Build release binaries
cargo build --release

# Create tag
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

## Contributing

1. Fork the repository
2. Create a feature branch:
   ```bash
   git checkout -b feature/my-feature
   ```
3. Make your changes
4. Add tests
5. Run tests and linter:
   ```bash
   cargo test --all
   cargo clippy -- -D warnings
   cargo fmt
   ```
6. Commit changes:
   ```bash
   git commit -m "Add my feature"
   ```
7. Push to fork:
   ```bash
   git push origin feature/my-feature
   ```
8. Create pull request

## Useful Commands

```bash
# Check for unused dependencies
cargo +nightly udeps

# Update dependencies
cargo update

# Audit dependencies for security issues
cargo audit

# Generate documentation
cargo doc --open

# Find code smells
cargo +nightly clippy -- -W clippy::all
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Documentation](https://tokio.rs/)
- [Candle Examples](https://github.com/huggingface/candle/tree/main/candle-examples)
- [Protobuf Guide](https://protobuf.dev/)

## Getting Help

- Ask questions in [GitHub Discussions](https://github.com/iZoom786/chatloop/discussions)
- Report bugs via [GitHub Issues](https://github.com/iZoom786/chatloop/issues)
- Check existing [documentation](README.md)

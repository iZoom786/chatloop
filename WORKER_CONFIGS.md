# Worker Configuration Summary

## Overview

This document shows the 4 worker configurations for ChatLoop's distributed inference setup.

## Worker Distribution

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│  Coordinator    │
│  Port: 50050    │
└────────┬────────┘
         │
    ┌────┴────┬────────┬────────┐
    ▼         ▼        ▼        ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│Worker0│ │Worker1│ │Worker2│ │Worker3│
│Port:  │ │Port:  │ │Port:  │ │Port:  │
│50051  │ │50052  │ │50053  │ │50054  │
└───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘
    │         │        │        │
    ▼         ▼        ▼        ▼
┌─────────────────────────────────┐
│    Model Layers (0-31)         │
│  ├─ Layers 0-7   (Worker 0)   │
│  ├─ Layers 8-15  (Worker 1)   │
│  ├─ Layers 16-23 (Worker 2)   │
│  └─ Layers 24-31 (Worker 3)   │
└─────────────────────────────────┘
```

## Worker Configurations

### Worker 0 (First Worker)
- **Config File**: `configs/worker-config-0.yaml`
- **Port**: 50051
- **Layers**: 0-7 (first 8 layers)
- **Model Partition**: `partition_0.safetensors`
- **Next Worker**: Worker 1 (port 50052)
- **Previous Worker**: None (first in pipeline)

### Worker 1
- **Config File**: `configs/worker-config-1.yaml`
- **Port**: 50052
- **Layers**: 8-15 (next 8 layers)
- **Model Partition**: `partition_1.safetensors`
- **Next Worker**: Worker 2 (port 50053)
- **Previous Worker**: Worker 0 (port 50051)

### Worker 2
- **Config File**: `configs/worker-config-2.yaml`
- **Port**: 50053
- **Layers**: 16-23 (next 8 layers)
- **Model Partition**: `partition_2.safetensors`
- **Next Worker**: Worker 3 (port 50054)
- **Previous Worker**: Worker 1 (port 50052)

### Worker 3 (Last Worker)
- **Config File**: `configs/worker-config-3.yaml`
- **Port**: 50054
- **Layers**: 24-31 (last 8 layers)
- **Model Partition**: `partition_3.safetensors`
- **Next Worker**: None (last in pipeline)
- **Previous Worker**: Worker 2 (port 50053)

## Request Flow

```
Request → Coordinator → Worker 0 (Layers 0-7)
                           ↓
                         Worker 1 (Layers 8-15)
                           ↓
                         Worker 2 (Layers 16-23)
                           ↓
                         Worker 3 (Layers 24-31)
                           ↓
                        Response
```

## Model Partition Mapping

| Worker | Partition File | Layers | Size (approx) |
|--------|----------------|--------|---------------|
| 0 | `partition_0.safetensors` | 0-7 | ~3.25 GB |
| 1 | `partition_1.safetensors` | 8-15 | ~3.25 GB |
| 2 | `partition_2.safetensors` | 16-23 | ~3.25 GB |
| 3 | `partition_3.safetensors` | 24-31 | ~3.25 GB |

**Total**: ~13 GB for Llama 2 7B model

## Configuration Parameters

### Common Settings (All Workers)

| Parameter | Value | Description |
|-----------|-------|-------------|
| `worker_threads` | 8 | Number of CPU cores per worker |
| `enable_cpu_pinning` | true | Bind worker to specific CPU cores |
| `max_batch_size` | 16 | Maximum requests in a batch |
| `batching_window_ms` | 5 | Wait time for batching |
| `max_queue_size` | 256 | Maximum queued requests |
| `kv_cache_mb` | 512 | KV cache size in memory |
| `enable_simd` | true | Enable SIMD optimizations |

### Network Configuration

| Service | Endpoint | Port |
|---------|----------|------|
| Coordinator | `coordinator` | 50050 |
| Worker 0 | `worker-0` | 50051 |
| Worker 1 | `worker-1` | 50052 |
| Worker 2 | `worker-2` | 50053 |
| Worker 3 | `worker-3` | 50054 |

## Running the Workers

### Using PowerShell Script

```powershell
# Start all workers
.\docker-run.ps1 run-worker 0
.\docker-run.ps1 run-worker 1
.\docker-run.ps1 run-worker 2
.\docker-run.ps1 run-worker 3
```

### Using Docker Compose

```powershell
docker-compose -f docker-compose-all-in-one.yml up -d
```

### Manual Docker Commands

```powershell
# Worker 0
docker run -d `
  --name chatloop-worker-0 `
  -e CHATLOOP_ROLE=worker `
  -p 50051:50051 `
  -v "${PWD}/models:/home/chatloop/models:ro" `
  -v "${PWD}/configs/worker-config-0.yaml:/home/chatloop/configs/worker-config.yaml:ro" `
  chatloop-all-in-one:latest

# Worker 1
docker run -d `
  --name chatloop-worker-1 `
  -e CHATLOOP_ROLE=worker `
  -p 50052:50052 `
  -v "${PWD}/models:/home/chatloop/models:ro" `
  -v "${PWD}/configs/worker-config-1.yaml:/home/chatloop/configs/worker-config.yaml:ro" `
  chatloop-all-in-one:latest

# Worker 2
docker run -d `
  --name chatloop-worker-2 `
  -e CHATLOOP_ROLE=worker `
  -p 50053:50053 `
  -v "${PWD}/models:/home/chatloop/models:ro" `
  -v "${PWD}/configs/worker-config-2.yaml:/home/chatloop/configs/worker-config.yaml:ro" `
  chatloop-all-in-one:latest

# Worker 3
docker run -d `
  --name chatloop-worker-3 `
  -e CHATLOOP_ROLE=worker `
  -p 50054:50054 `
  -v "${PWD}/models:/home/chatloop/models:ro" `
  -v "${PWD}/configs/worker-config-3.yaml:/home/chatloop/configs/worker-config.yaml:ro" `
  chatloop-all-in-one:latest
```

## Verification

Check all workers are running:

```powershell
.\docker-run.ps1 status
```

Expected output:
```
NAMES                   STATUS          PORTS
chatloop-coordinator    Up 2 minutes    0.0.0.0:50050->50050/tcp, 0.0.0.0:9090->9091/tcp
chatloop-worker-0       Up 2 minutes    0.0.0.0:50051->50051/tcp, 0.0.0.0:9091->9091/tcp
chatloop-worker-1       Up 2 minutes    0.0.0.0:50052->50052/tcp, 0.0.0.0:9092->9091/tcp
chatloop-worker-2       Up 2 minutes    0.0.0.0:50053->50053/tcp, 0.0.0.0:9093->9091/tcp
chatloop-worker-3       Up 2 minutes    0.0.0.0:50054->50054/tcp, 0.0.0.0:9094->9091/tcp
```

## Troubleshooting

### Worker fails to start

1. **Check config file exists**:
   ```powershell
   Test-Path "configs\worker-config-0.yaml"
   ```

2. **Check model partition exists**:
   ```powershell
   Test-Path "models\partitions\partition_0.safetensors"
   ```

3. **Check logs**:
   ```powershell
   docker logs chatloop-worker-0
   ```

### Worker can't connect to next worker

1. **Verify network connectivity**:
   ```powershell
   docker exec chatloop-worker-0 ping worker-1
   ```

2. **Check next worker is running**:
   ```powershell
   docker ps | grep chatloop-worker
   ```

### Port already in use

Change the port in the config file:
```yaml
port: 50055  # Use different port
```

## Scaling

### Add More Partitions

If you want to use 8 partitions instead of 4:

1. Split model into 8 partitions:
   ```powershell
   python python\model_splitter\split_model.py `
       --model gpt2 `
       --output .\models\partitions `
       --num-partitions 8
   ```

2. Create worker configs for workers 4-7 with:
   - Worker 4: layers 28-31 (if 32 total)
   - Worker 5: layers 32-35
   - etc.

3. Update each worker's `start_layer` and `end_layer`

### Use Larger Models

For Llama 2 13B (more layers):

1. Adjust `total_layers` in configs (e.g., 40 layers)
2. Create more workers to handle all layers
3. Each worker handles ~8-10 layers

## Next Steps

1. ✅ **Configs created** - All 4 worker configs are ready
2. ⏳ **Split model** - Run the model splitter
3. ⏳ **Build Docker image** - Run `.\docker-run.ps1 build`
4. ⏳ **Start services** - Run coordinator and workers
5. ⏳ **Test inference** - Send test requests

---

For more information:
- [MODEL_SETUP_GUIDE.md](MODEL_SETUP_GUIDE.md)
- [DOCKER_GUIDE.md](DOCKER_GUIDE.md)
- [README.md](README.md)

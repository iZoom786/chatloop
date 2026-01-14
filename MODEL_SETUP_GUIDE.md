# Model Setup Guide for ChatLoop

This guide explains how to prepare and place model files for ChatLoop.

## Directory Structure

Your ChatLoop directory should look like this:

```
E:\chatloop\
├── configs/                      # Configuration files
│   ├── coordinator-config.yaml
│   ├── worker-config-0.yaml
│   ├── worker-config-1.yaml
│   ├── worker-config-2.yaml
│   └── worker-config-3.yaml
│
├── models/                       # 👈 MODEL FILES GO HERE
│   ├── partitions/               # Partitioned models
│   │   ├── partition_0.safetensors
│   │   ├── partition_1.safetensors
│   │   ├── partition_2.safetensors
│   │   ├── partition_3.safetensors
│   │   ├── partition_metadata.json
│   │   └── tokenizer/            # Tokenizer files
│   │       ├── config.json
│   │       ├── tokenizer_config.json
│   │       └── vocab.json
│   │
│   └── full-models/              # Optional: Full models before splitting
│       └── gpt2/
│
├── logs/                         # Created automatically
├── docker/                       # Docker files
├── crates/                       # Rust source code
└── ...
```

## Method 1: Split a HuggingFace Model (Recommended)

### Step 1: Create the models directory

```powershell
# In PowerShell
cd E:\chatloop
mkdir models\partitions
```

### Step 2: Install Python dependencies

```powershell
pip install torch transformers safetensors accelerate
```

### Step 3: Split the model

```powershell
# Using GPT-2 (smaller, good for testing)
python python\model_splitter\split_model.py `
    --model gpt2 `
    --output .\models\partitions `
    --num-partitions 4

# Or using Llama 2 (requires authentication)
python python\model_splitter\split_model.py `
    --model meta-llama/Llama-2-7b-hf `
    --output .\models\partitions `
    --num-partitions 4
```

This will create:
```
models\partitions\
├── partition_0.safetensors  (layers 0-7)
├── partition_1.safetensors  (layers 8-15)
├── partition_2.safetensors  (layers 16-23)
├── partition_3.safetensors  (layers 24-31)
├── partition_metadata.json
└── tokenizer\
    ├── config.json
    ├── tokenizer_config.json
    └── vocab.json
```

## Method 2: Use Pre-partitioned Models

If you already have partitioned models:

### Step 1: Create models directory

```powershell
mkdir models\partitions
```

### Step 2: Copy your partition files

Place your `.safetensors` files in:
```
E:\chatloop\models\partitions\
```

Expected files:
- `partition_0.safetensors`
- `partition_1.safetensors`
- `partition_2.safetensors`
- `partition_3.safetensors`

### Step 3: Verify file paths

Your Docker containers need to mount these files. The paths in your worker configs should match:

**`configs/worker-config-0.yaml`:**
```yaml
worker:
  weights_path: "/home/chatloop/models/partition_0.safetensors"
```

This maps to:
```
E:\chatloop\models\partitions\partition_0.safetensors
```

## Method 3: Download Pre-trained Models

### Option A: Using GPT-2 (No authentication needed)

```powershell
# GPT-2 is small and public
python python\model_splitter\split_model.py `
    --model gpt2 `
    --output .\models\partitions `
    --num-partitions 4
```

### Option B: Using Llama 2 (Requires HuggingFace account)

1. **Request access**: https://huggingface.co/meta-llama/Llama-2-7b-hf
2. **Accept the license** on HuggingFace
3. **Login to HuggingFace**:
   ```powershell
   pip install huggingface-hub
   huggingface-cli login
   ```
4. **Split the model**:
   ```powershell
   python python\model_splitter\split_model.py `
       --model meta-llama/Llama-2-7b-hf `
       --output .\models\partitions `
       --num-partitions 4
   ```

## Directory Mounting in Docker

When using Docker, the `models` directory is mounted into the container:

### Using docker-run.ps1

The script automatically mounts:
```powershell
-v "${PWD}\/models:/home/chatloop/models:ro"
```

This maps:
- Host: `E:\chatloop\models\`
- Container: `/home/chatloop/models/`

### Using docker-compose

```yaml
volumes:
  - ./models:/home/chatloop/models:ro
```

### Manual Docker Run

```powershell
docker run -d \
  --name chatloop-worker-0 \
  -v "E:\chatloop\models:/home/chatloop/models:ro" \
  chatloop-all-in-one:latest
```

## Verification

### Check Files Exist

```powershell
# List model files
dir E:\chatloop\models\partitions

# Should show:
# partition_0.safetensors
# partition_1.safetensors
# partition_2.safetensors
# partition_3.safetensors
```

### Check File Sizes

```powershell
# Show file sizes
dir E:\chatloop\models\partitions | Measure-Object
```

Typical sizes:
- GPT-2 partitions: ~50-100 MB each
- Llama 2 7B partitions: ~3-4 GB each

## Configuration File Setup

Each worker needs its own config pointing to the correct partition:

### Worker 0 (`configs/worker-config-0.yaml`)

```yaml
mode: "worker"
port: 50051

worker:
  worker_id: "worker-0"
  layer_group:
    start_layer: 0      # First 8 layers
    end_layer: 8
    total_layers: 32
  weights_path: "/home/chatloop/models/partitions/partition_0.safetensors"
  next_worker_endpoint: "http://worker-1:50052"
```

### Worker 1 (`configs/worker-config-1.yaml`)

```yaml
mode: "worker"
port: 50052

worker:
  worker_id: "worker-1"
  layer_group:
    start_layer: 8      # Next 8 layers
    end_layer: 16
    total_layers: 32
  weights_path: "/home/chatloop/models/partitions/partition_1.safetensors"
  next_worker_endpoint: "http://worker-2:50053"
```

### Worker 2 (`configs/worker-config-2.yaml`)

```yaml
mode: "worker"
port: 50053

worker:
  worker_id: "worker-2"
  layer_group:
    start_layer: 16     # Next 8 layers
    end_layer: 24
    total_layers: 32
  weights_path: "/home/chatloop/models/partitions/partition_2.safetensors"
  next_worker_endpoint: "http://worker-3:50054"
```

### Worker 3 (`configs/worker-config-3.yaml`)

```yaml
mode: "worker"
port: 50054

worker:
  worker_id: "worker-3"
  layer_group:
    start_layer: 24     # Last 8 layers
    end_layer: 32
    total_layers: 32
  weights_path: "/home/chatloop/models/partitions/partition_3.safetensors"
  next_worker_endpoint: null  # No next worker
```

## Quick Setup Script

Save this as `setup-models.ps1`:

```powershell
# Quick model setup script

Write-Host "ChatLoop Model Setup" -ForegroundColor Cyan

# Create directories
Write-Host "Creating directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path "models\partitions" | Out-Null

# Check if model exists
if (Test-Path "models\partitions\partition_0.safetensors") {
    Write-Host "Model partitions already exist!" -ForegroundColor Green
    Write-Host "Skipping download."
} else {
    Write-Host "Model partitions not found." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Choose an option:" -ForegroundColor Cyan
    Write-Host "1. Split GPT-2 model (small, fast, ~500MB)"
    Write-Host "2. Split Llama 2 7B (large, slow, ~13GB)"
    Write-Host "3. Exit"

    $choice = Read-Host "Enter choice (1-3)"

    switch ($choice) {
        "1" {
            Write-Host "Downloading and splitting GPT-2..." -ForegroundColor Cyan
            python python\model_splitter\split_model.py `
                --model gpt2 `
                --output .\models\partitions `
                --num-partitions 4
        }
        "2" {
            Write-Host "Downloading and splitting Llama 2..." -ForegroundColor Cyan
            python python\model_splitter\split_model.py `
                --model meta-llama/Llama-2-7b-hf `
                --output .\models\partitions `
                --num-partitions 4
        }
        "3" {
            Write-Host "Exiting..." -ForegroundColor Yellow
            exit
        }
    }
}

Write-Host ""
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host "Model files are in: models\partitions\"
Write-Host "You can now start the services:"
Write-Host "  .\docker-run.ps1 run-coordinator"
Write-Host "  .\docker-run.ps1 run-worker 0"
Write-Host "  .\docker-run.ps1 run-worker 1"
Write-Host "  .\docker-run.ps1 run-worker 2"
Write-Host "  .\docker-run.ps1 run-worker 3"
```

Run it:
```powershell
.\setup-models.ps1
```

## Troubleshooting

### Issue: "Model file not found"

**Solution**: Check the file path in your config matches the actual file location:

```powershell
# In container, the path should be:
/home/chatloop/models/partition_0.safetensors

# On your host, it should be:
E:\chatloop\models\partitions\partition_0.safetensors
```

### Issue: "Permission denied accessing model files"

**Solution**: Make sure the directory exists and is readable:

```powershell
# Check directory exists
Test-Path "E:\chatloop\models\partitions"

# Check files are readable
dir E:\chatloop\models\partitions
```

### Issue: "Out of disk space"

**Solution**:
- GPT-2: ~1GB total
- Llama 2 7B: ~13GB total

Make sure you have enough free space on `E:\` drive.

### Issue: "HuggingFace authentication error"

**Solution**:
```powershell
pip install huggingface-hub
huggingface-cli login
```

Enter your HuggingFace token from: https://huggingface.co/settings/tokens

## Next Steps

Once models are in place:

1. **Verify files exist**: `dir models\partitions`
2. **Create worker configs** (see examples above)
3. **Build Docker image**: `.\docker-run.ps1 build`
4. **Start services**:
   ```powershell
   .\docker-run.ps1 run-coordinator
   .\docker-run.ps1 run-worker 0
   .\docker-run.ps1 run-worker 1
   .\docker-run.ps1 run-worker 2
   .\docker-run.ps1 run-worker 3
   ```
5. **Check logs**: `.\docker-run.ps1 logs worker-0`

## Storage Requirements

| Model | Total Size | Partition Size | Partitions |
|-------|-----------|----------------|------------|
| GPT-2 (small) | ~500 MB | ~125 MB | 4 |
| GPT-2 (medium) | ~1.5 GB | ~375 MB | 4 |
| GPT-2 (large) | ~5 GB | ~1.25 GB | 4 |
| Llama 2 7B | ~13 GB | ~3.25 GB | 4 |
| Llama 2 13B | ~26 GB | ~6.5 GB | 4 |

Make sure `E:\` drive has enough space!

---

For more information, see:
- [Quick Start Guide](QUICKSTART.md)
- [Docker Guide](DOCKER_GUIDE.md)
- [README](README.md)

# ChatLoop Directory Structure

Complete directory layout showing where to place model files.

```
E:\chatloop\                          # Root directory
│
├── 📁 configs/                      # Configuration files
│   ├── coordinator-config.yaml      # Coordinator settings
│   ├── worker-config-0.yaml        # Worker 0 settings
│   ├── worker-config-1.yaml        # Worker 1 settings
│   ├── worker-config-2.yaml        # Worker 2 settings
│   └── worker-config-3.yaml        # Worker 3 settings
│
├── 📁 models/                       # ⭐ MODEL FILES GO HERE ⭐
│   └── 📁 partitions/              # Partitioned model files
│       ├── 📄 partition_0.safetensors          # Layers 0-7
│       ├── 📄 partition_1.safetensors          # Layers 8-15
│       ├── 📄 partition_2.safetensors          # Layers 16-23
│       ├── 📄 partition_3.safetensors          # Layers 24-31
│       ├── 📄 partition_metadata.json
│       │
│       └── 📁 tokenizer/             # Tokenizer files
│           ├── config.json
│           ├── tokenizer_config.json
│           ├── vocab.json
│           └── merges.txt            # (for some models)
│
├── 📁 logs/                         # Auto-created log files
│   ├── coordinator.log
│   ├── worker-0.log
│   ├── worker-1.log
│   ├── worker-2.log
│   └── worker-3.log
│
├── 📁 docker/                       # Docker files
│   ├── all-in-one.Dockerfile       # Single Dockerfile for both services
│   ├── base.Dockerfile
│   ├── worker.Dockerfile
│   └── coordinator.Dockerfile
│
├── 📁 crates/                       # Rust source code
│   ├── proto/                       # Protocol definitions
│   ├── common/                      # Shared code
│   ├── worker/                      # Worker service
│   └── coordinator/                 # Coordinator service
│
├── 📁 python/                       # Python tooling
│   └── model_splitter/
│       ├── split_model.py          # Model splitting script
│       └── requirements.txt
│
├── 📁 yarn/                         # Hadoop YARN integration
│   ├── worker-service.xml
│   ├── coordinator-service.xml
│   └── launch-scripts/
│
├── 📄 docker-compose-all-in-one.yml # Docker Compose config
├── 📄 docker-run.ps1                # Windows management script
├── 📄 docker-run.sh                 # Linux/macOS management script
├── 📄 Cargo.toml                    # Rust workspace config
├── 📄 README.md                     # Main documentation
├── 📄 QUICKSTART.md                 # Quick start guide
├── 📄 DOCKER_GUIDE.md               # Docker setup guide
├── 📄 MODEL_SETUP_GUIDE.md          # Model setup guide
└── 📄 DEVELOPMENT.md                # Development guide
```

## How Model Files Are Used

### Worker Containers Mount Model Files

Each worker container mounts the `models` directory:

```powershell
docker run -d \
  --name chatloop-worker-0 \
  -v "E:\chatloop\models:/home/chatloop/models:ro" \
  chatloop-all-in-one:latest
```

**Mapping:**
- Host: `E:\chatloop\models\partitions\partition_0.safetensors`
- Container: `/home/chatloop/models/partition_0.safetensors`

### Configuration Points to Model

In `configs/worker-config-0.yaml`:

```yaml
worker:
  weights_path: "/home/chatloop/models/partitions/partition_0.safetensors"
```

### Worker Loads the Partition

When worker starts:

1. Reads config file → gets model path
2. Opens `/home/chatloop/models/partitions/partition_0.safetensors`
3. Memory-maps the file (no loading into RAM)
4. Ready to process requests!

## Step-by-Step Setup

### 1️⃣ Create Directory Structure

```powershell
cd E:\chatloop
mkdir models\partitions
```

### 2️⃣ Download/Split Model

```powershell
# Using GPT-2 (fastest, good for testing)
python python\model_splitter\split_model.py `
    --model gpt2 `
    --output .\models\partitions `
    --num-partitions 4
```

### 3️⃣ Verify Files Created

```powershell
dir models\partitions
```

Expected output:
```
partition_0.safetensors
partition_1.safetensors
partition_2.safetensors
partition_3.safetensors
partition_metadata.json
tokenizer/
```

### 4️⃣ Update Worker Configs

Edit each `configs/worker-config-*.yaml`:

```yaml
worker:
  weights_path: "/home/chatloop/models/partition_0.safetensors"  # Change number
```

### 5️⃣ Start Services

```powershell
.\docker-run.ps1 run-coordinator
.\docker-run.ps1 run-worker 0
.\docker-run.ps1 run-worker 1
.\docker-run.ps1 run-worker 2
.\docker-run.ps1 run-worker 3
```

## File Sizes Reference

### GPT-2 (Recommended for Testing)
```
partition_0.safetensors  ~120 MB
partition_1.safetensors  ~120 MB
partition_2.safetensors  ~120 MB
partition_3.safetensors  ~120 MB
-------------------------
Total: ~500 MB
```

### Llama 2 7B
```
partition_0.safetensors  ~3.2 GB
partition_1.safetensors  ~3.2 GB
partition_2.safetensors  ~3.2 GB
partition_3.safetensors  ~3.2 GB
-------------------------
Total: ~13 GB
```

## Common Mistakes to Avoid

### ❌ Wrong Path in Config

**Don't use:**
```yaml
weights_path: "E:\chatloop\models\partition_0.safetensors"  # ❌ Wrong! Host path
weights_path: "./models/partition_0.safetensors"           # ❌ Wrong! Relative path
```

**Do use:**
```yaml
weights_path: "/home/chatloop/models/partitions/partition_0.safetensors"  # ✅ Correct!
```

### ❌ Missing Partition Number

**Don't name files:**
```
model.safetensors     # ❌ Wrong!
worker0.safetensors   # ❌ Wrong!
```

**Do name files:**
```
partition_0.safetensors  # ✅ Correct!
partition_1.safetensors  # ✅ Correct!
```

### ❌ Wrong Directory

**Don't place files in:**
```
E:\chatloop\models\partition_0.safetensors  # ❌ Wrong level
E:\chatloop\partition_0.safetensors         # ❌ Wrong directory
```

**Do place files in:**
```
E:\chatloop\models\partitions\partition_0.safetensors  # ✅ Correct!
```

## Quick Verification

Run these commands to verify setup:

```powershell
# 1. Check directory exists
Test-Path "E:\chatloop\models\partitions"
# Should return: True

# 2. List partition files
Get-ChildItem "E:\chatloop\models\partitions\partition_*.safetensors"
# Should show: 4 files

# 3. Check total size
(Get-ChildItem "E:\chatloop\models\partitions" -Recurse |
    Measure-Object -Property Length -Sum).Sum / 1GB
# Should show size in GB

# 4. Verify metadata
Test-Path "E:\chatloop\models\partitions\partition_metadata.json"
# Should return: True
```

## Summary

✅ **Place model files in:** `E:\chatloop\models\partitions\`

✅ **Required files:**
- `partition_0.safetensors`
- `partition_1.safetensors`
- `partition_2.safetensors`
- `partition_3.safetensors`
- `partition_metadata.json`
- `tokenizer/` directory

✅ **Config path:** `/home/chatloop/models/partitions/partition_X.safetensors`

✅ **Docker mount:** `-v E:\chatloop\models:/home/chatloop/models:ro`

---

For detailed instructions, see [MODEL_SETUP_GUIDE.md](MODEL_SETUP_GUIDE.md)

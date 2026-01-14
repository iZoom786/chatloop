# Docker Build Troubleshooting Guide

This guide helps you troubleshoot issues when building the ChatLoop Docker image.

## Prerequisites

### Windows
- Docker Desktop for Windows installed and running
- PowerShell (included with Windows)
- At least 4GB of RAM available for Docker
- At least 10GB of free disk space

### Verifying Docker is Running
```powershell
docker version
docker ps
```

Both commands should succeed without errors.

## Common Issues and Solutions

### Issue 1: "not found" Error When Copying Binaries

**Error Message:**
```
ERROR: failed to build: failed to solve: failed to compute cache key: failed to calculate checksum of ref: "/build/target/release/chatloop-coordinator": not found
```

**Cause:** The Rust compilation failed, so the binaries were never created.

**Solution:**
1. Check the build log in the Docker output
2. Look for compilation errors above the "not found" message
3. Common compilation errors:
   - Missing dependencies
   - Syntax errors in Rust code
   - Type mismatches
   - Missing imports

**Example Fix:**
```
# The build log will show something like:
# error[E0433]: failed to resolve: use of undeclared crate
# This means a dependency is missing from Cargo.toml
```

### Issue 2: Build Takes Too Long

**Normal Behavior:** First build can take 10-20 minutes due to:
- Downloading Rust toolchain
- Compiling dependencies (tonic, tokio, candle-core, etc.)
- Optimizing code in release mode

**Subsequent builds:** Should be faster (2-5 minutes) due to Docker layer caching.

### Issue 3: Out of Memory During Build

**Error Message:**
```
error: could not compile <crate> (signal: 9, SIGKILL)
```

**Cause:** Docker ran out of memory during compilation.

**Solution:**
1. Increase Docker memory limit in Docker Desktop:
   - Open Docker Desktop
   - Go to Settings → Resources
   - Increase "Memory" to at least 4GB
   - Click "Apply & Restart"

### Issue 4: Network Timeout During Build

**Error Message:**
```
error: failed to get `<crate>` from registry
```

**Cause:** Network connectivity issues or slow download.

**Solution:**
1. Check internet connection
2. Try again (may be a temporary network issue)
3. Use a different network if available

### Issue 5: Permission Denied Errors

**Error Message:**
```
permission denied while opening...
```

**Cause:** File permissions issue on Linux/Mac.

**Solution:**
```bash
# Fix file permissions
chmod -R 755 .
```

## Build Process Overview

### Stage 1: Builder (Rust Compilation)
1. Install Rust toolchain
2. Install build dependencies (pkg-config, libssl-dev, etc.)
3. Copy source code
4. Compile with `cargo build --release`
5. Output: `target/release/chatloop-worker` and `target/release/chatloop-coordinator`

### Stage 2: Runtime (Minimal Image)
1. Start with slim Debian image
2. Install runtime dependencies (libssl3, ca-certificates)
3. Copy compiled binaries from builder
4. Copy configuration files
5. Setup non-root user
6. Set environment variables

## Verifying the Build

### Check if Build Succeeded
```powershell
# Build the image
.\docker-run.ps1 build

# Check if image exists
docker images | Select-String "chatloop-all-in-one"
```

### Test the Image
```powershell
# Test coordinator (should fail without config, but proves binary exists)
docker run --rm chatloop-all-in-one:latest echo "Coordinator binary:" && which chatloop-coordinator

# Test worker
docker run --rm chatloop-all-in-one:latest echo "Worker binary:" && which chatloop-worker
```

## Viewing Build Logs

### Save Build Log to File
```powershell
docker build -f docker/all-in-one.Dockerfile -t chatloop-all-in-one:latest . 2>&1 | Out-File -FilePath build.log
```

### View Build Log
```powershell
Get-Content build.log | Select-String -Pattern "error" -Context 2
```

## Getting Help

### Information to Collect
1. Full error message
2. Build log (last 50 lines)
3. Docker version: `docker version`
4. System information:
   - Windows version
   - Docker Desktop version
   - Available RAM and disk space

### Useful Commands for Debugging
```powershell
# Check Docker status
docker info

# Check build cache
docker system df

# Clean build cache (if needed)
docker builder prune -f

# Rebuild without cache
docker build --no-cache -f docker/all-in-one.Dockerfile -t chatloop-all-in-one:latest .
```

## Manual Build (For Development)

If Docker builds keep failing, you can try building locally:

### Prerequisites
- Install Rust: https://rustup.rs/
- Install Visual Studio Build Tools (Windows)
- Install OpenSSL: `vcpkg install openssl:x64-windows`

### Build Commands
```powershell
# Build all crates
cargo build --release

# Check if binaries exist
ls target\release\chatloop-*.exe

# Run worker
.\target\release\chatloop-worker.exe

# Run coordinator
.\target\release\chatloop-coordinator.exe
```

## Next Steps After Successful Build

1. **Prepare Model Files:**
   - Download model (e.g., Llama-2-7B)
   - Split into partitions using Python tool
   - Place in `models/partitions/` directory

2. **Configure Workers:**
   - Edit `configs/worker-config-0.yaml` through `configs/worker-config-3.yaml`
   - Adjust layer assignments if needed
   - Set batching parameters

3. **Deploy:**
   - Run coordinator: `.\docker-run.ps1 run-coordinator`
   - Run workers: `.\docker-run.ps1 run-worker 0`, `.\docker-run.ps1 run-worker 1`, etc.

4. **Test:**
   - Send test inference request
   - Monitor logs: `.\docker-run.ps1 logs`
   - Check metrics at http://localhost:9091/metrics

## Quick Reference

### Build Commands
```powershell
# Build
.\docker-run.ps1 build

# Build with debug output
docker build -f docker/all-in-one.Dockerfile -t chatloop-all-in-one:latest --progress=plain .

# Rebuild without cache
docker build --no-cache -f docker/all-in-one.Dockerfile -t chatloop-all-in-one:latest .
```

### Run Commands
```powershell
# Run coordinator
.\docker-run.ps1 run-coordinator

# Run worker (specify partition number)
.\docker-run.ps1 run-worker 0

# View logs
.\docker-run.ps1 logs

# Stop all
.\docker-run.ps1 stop-all

# Check status
.\docker-run.ps1 status
```

### Troubleshooting Commands
```powershell
# Check Docker is running
docker version

# View running containers
docker ps -a

# View container logs
docker logs <container-id>

# Execute shell in container
docker exec -it <container-id> /bin/bash

# Check for binaries in container
docker exec -it <container-id> ls -la /usr/local/bin/chatloop-*
```

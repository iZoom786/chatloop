# ChatLoop Build Script for Windows
# Provides similar functionality to Makefile

param(
    [Parameter(Position=0)]
    [ValidateSet("build", "test", "clean", "docker-base", "docker-worker", "docker-coordinator", "docker-build", "docker-run", "split-model", "help")]
    [string]$Task = "help"
)

$ErrorActionPreference = "Stop"

function Show-Help {
    Write-Host "ChatLoop Build Script - Windows" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\build.ps1 [task]" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Available tasks:" -ForegroundColor Green
    Write-Host "  build            - Build all Rust crates"
    Write-Host "  test             - Run all tests"
    Write-Host "  clean            - Clean build artifacts"
    Write-Host "  docker-base      - Build base Docker image"
    Write-Host "  docker-worker    - Build worker Docker image"
    Write-Host "  docker-coordinator - Build coordinator Docker image"
    Write-Host "  docker-build     - Build all Docker images"
    Write-Host "  split-model      - Split a model into partitions"
    Write-Host "  help             - Show this help message"
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  .\build.ps1 build"
    Write-Host "  .\build.ps1 docker-build"
    Write-Host "  .\build.ps1 split-model"
}

function Build-Cargo {
    Write-Host "Building ChatLoop..." -ForegroundColor Cyan
    cargo build --release
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Build successful!" -ForegroundColor Green
    } else {
        Write-Host "Build failed!" -ForegroundColor Red
        exit 1
    }
}

function Test-Cargo {
    Write-Host "Running tests..." -ForegroundColor Cyan
    cargo test --all
}

function Clean-Cargo {
    Write-Host "Cleaning build artifacts..." -ForegroundColor Cyan
    cargo clean
    Remove-Item -Path "models\*.safetensors" -ErrorAction SilentlyContinue
    Remove-Item -Path "models\partitions\*" -Recurse -ErrorAction SilentlyContinue
    Write-Host "Clean complete!" -ForegroundColor Green
}

function Build-DockerBase {
    Write-Host "Building base Docker image..." -ForegroundColor Cyan
    docker build -f docker/base.Dockerfile -t chatloop-base:latest .
}

function Build-DockerWorker {
    Write-Host "Building worker Docker image..." -ForegroundColor Cyan
    docker build -f docker/worker.Dockerfile -t chatloop-worker:latest .
}

function Build-DockerCoordinator {
    Write-Host "Building coordinator Docker image..." -ForegroundColor Cyan
    docker build -f docker/coordinator.Dockerfile -t chatloop-coordinator:latest .
}

function Build-DockerAll {
    Build-DockerBase
    Build-DockerWorker
    Build-DockerCoordinator
    Write-Host "All Docker images built successfully!" -ForegroundColor Green
}

function Split-Model {
    param(
        [string]$ModelName = "meta-llama/Llama-2-7b-hf",
        [int]$NumPartitions = 4,
        [string]$Output = "./models/partitions"
    )

    Write-Host "Splitting model: $ModelName into $NumPartitions partitions..." -ForegroundColor Cyan
    python python/model_splitter/split_model.py `
        --model $ModelName `
        --output $Output `
        --num-partitions $NumPartitions
}

# Main execution
switch ($Task) {
    "build" { Build-Cargo }
    "test" { Test-Cargo }
    "clean" { Clean-Cargo }
    "docker-base" { Build-DockerBase }
    "docker-worker" { Build-DockerWorker }
    "docker-coordinator" { Build-DockerCoordinator }
    "docker-build" { Build-DockerAll }
    "split-model" { Split-Model }
    "help" { Show-Help }
    default { Show-Help }
}

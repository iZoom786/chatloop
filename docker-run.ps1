# ChatLoop Docker Build and Run Script for Windows
# No Rust installation required - everything runs in Docker

param(
    [Parameter(Position=0)]
    [ValidateSet("build", "run-coordinator", "run-worker", "stop-all", "cleanup", "logs", "status", "help")]
    [string]$Command = "help",

    [Parameter(Position=1)]
    [string]$WorkerId = "0",

    [Parameter(Position=2)]
    [string]$Service = "coordinator"
)

$ErrorActionPreference = "Stop"
$IMAGE_NAME = "chatloop-all-in-one"

function Show-Help {
    Write-Host "ChatLoop Docker Management Script for Windows" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\docker-run.ps1 [command] [options]" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Commands:" -ForegroundColor Green
    Write-Host "  build              - Build the Docker image"
    Write-Host "  run-coordinator    - Run a coordinator container"
    Write-Host "  run-worker [id]    - Run a worker container (default id: 0)"
    Write-Host "  stop-all           - Stop all ChatLoop containers"
    Write-Host "  cleanup            - Remove all ChatLoop containers and images"
    Write-Host "  logs [service]     - Show logs for a service"
    Write-Host "  status             - Show status of all containers"
    Write-Host "  help               - Show this help message"
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  .\docker-run.ps1 build"
    Write-Host "  .\docker-run.ps1 run-coordinator"
    Write-Host "  .\docker-run.ps1 run-worker 0"
    Write-Host "  .\docker-run.ps1 run-worker 1"
    Write-Host "  .\docker-run.ps1 stop-all"
    Write-Host "  .\docker-run.ps1 logs coordinator"
    Write-Host "  .\docker-run.ps1 logs worker-0"
    Write-Host ""
    Write-Host "Environment Variables (optional):" -ForegroundColor Yellow
    Write-Host "  CHATLOOP_MODEL_PATH     - Path to model partition (for workers)"
    Write-Host "  CHATLOOP_CONFIG_PATH    - Path to config directory"
    Write-Host "  CHATLOOP_PORT           - Port to expose"
}

function Build-Image {
    Write-Host "Building ChatLoop Docker image..." -ForegroundColor Cyan
    docker build -f docker/all-in-one.Dockerfile -t "${IMAGE_NAME}:latest" .
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Build complete!" -ForegroundColor Green
    } else {
        Write-Host "Build failed!" -ForegroundColor Red
        exit 1
    }
}

function Run-Coordinator {
    $port = if ($env:CHATLOOP_PORT) { $env:CHATLOOP_PORT } else { "50050" }

    Write-Host "Starting ChatLoop Coordinator on port $port..." -ForegroundColor Cyan

    docker run -d `
        --name chatloop-coordinator `
        --restart unless-stopped `
        -e CHATLOOP_ROLE=coordinator `
        -e CHATLOOP_PORT=$port `
        -p "${port}:50050" `
        -p "9090:9091" `
        -v "${PWD}/configs/coordinator-config.yaml:/home/chatloop/configs/coordinator-config.yaml:ro" `
        -v "${PWD}/logs:/home/chatloop/logs" `
        "${IMAGE_NAME}:latest"

    if ($LASTEXITCODE -eq 0) {
        Write-Host "Coordinator started! Access it at localhost:$port" -ForegroundColor Green
        Write-Host "View logs: docker logs -f chatloop-coordinator" -ForegroundColor Yellow
    }
}

function Run-Worker {
    $workerId = $WorkerId
    $port = if ($env:CHATLOOP_PORT) { [int]$env:CHATLOOP_PORT } else { 50051 + [int]$workerId }
    $configFile = "${PWD}\configs\worker-config-${workerId}.yaml"

    # Check if config exists
    if (-not (Test-Path $configFile)) {
        Write-Host "Error: Config file not found: $configFile" -ForegroundColor Red
        Write-Host "Please create it first or use the default config." -ForegroundColor Yellow
        exit 1
    }

    Write-Host "Starting ChatLoop Worker ${workerId} on port $port..." -ForegroundColor Cyan

    docker run -d `
        --name "chatloop-worker-${workerId}" `
        --restart unless-stopped `
        -e CHATLOOP_ROLE=worker `
        -e CHATLOOP_WORKER_ID="worker-${workerId}" `
        -e CHATLOOP_PORT=$port `
        -p "${port}:50051" `
        -p "$(9091 + [int]$workerId):9091" `
        -v "${PWD}\models:/home/chatloop/models:ro" `
        -v "${configFile}:/home/chatloop/configs/worker-config.yaml:ro" `
        -v "${PWD}\logs:/home/chatloop/logs" `
        "${IMAGE_NAME}:latest"

    if ($LASTEXITCODE -eq 0) {
        Write-Host "Worker ${workerId} started!" -ForegroundColor Green
        Write-Host "View logs: docker logs -f chatloop-worker-${workerId}" -ForegroundColor Yellow
    }
}

function Stop-All {
    Write-Host "Stopping all ChatLoop containers..." -ForegroundColor Cyan

    $containers = @("chatloop-coordinator", "chatloop-worker-0", "chatloop-worker-1", "chatloop-worker-2", "chatloop-worker-3")
    foreach ($container in $containers) {
        docker stop $container 2>$null | Out-Null
        docker rm $container 2>$null | Out-Null
    }

    Write-Host "All containers stopped and removed." -ForegroundColor Green
}

function CleanUp {
    Write-Host "Cleaning up ChatLoop containers and images..." -ForegroundColor Cyan
    Stop-All
    docker rmi "${IMAGE_NAME}:latest" 2>$null | Out-Null
    Write-Host "Cleanup complete." -ForegroundColor Green
}

function Show-Logs {
    $serviceName = $Service
    $containerName = "chatloop-${serviceName}"

    $running = docker ps --format "{{.Names}}" | Select-String -Pattern "^${containerName}$"

    if ($running) {
        docker logs -f $containerName
    } else {
        Write-Host "Container $containerName is not running." -ForegroundColor Red
        Write-Host "Run '.\docker-run.ps1 status' to see all containers." -ForegroundColor Yellow
    }
}

function Show-Status {
    Write-Host "ChatLoop Container Status:" -ForegroundColor Cyan
    Write-Host "===========================" -ForegroundColor Cyan
    docker ps --filter "name=chatloop-" --format "table {{.Names}}`t{{.Status}}`t{{.Ports}}"
}

# Main script logic
switch ($Command) {
    "build" {
        Build-Image
    }
    "run-coordinator" {
        Run-Coordinator
    }
    "run-worker" {
        Run-Worker
    }
    "stop-all" {
        Stop-All
    }
    "cleanup" {
        CleanUp
    }
    "logs" {
        Show-Logs
    }
    "status" {
        Show-Status
    }
    "help" {
        Show-Help
    }
    default {
        Write-Host "Unknown command: $Command" -ForegroundColor Red
        Write-Host ""
        Show-Help
        exit 1
    }
}

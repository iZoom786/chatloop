# Build Docker image with full output capture
param(
    [switch]$NoCache = $false
)

$ErrorActionPreference = "Continue"
$IMAGE_NAME = "chatloop-all-in-one"
$LOG_FILE = "docker-build-$(Get-Date -Format 'yyyyMMdd-HHmmss').log"

Write-Host "Building ChatLoop Docker image..." -ForegroundColor Cyan
Write-Host "Log file: $LOG_FILE" -ForegroundColor Yellow
Write-Host ""

# Check if Docker is running
try {
    $null = docker version 2>&1
    Write-Host "✓ Docker is running" -ForegroundColor Green
} catch {
    Write-Host "✗ Docker is not running!" -ForegroundColor Red
    Write-Host "Please start Docker Desktop and try again." -ForegroundColor Yellow
    exit 1
}

# Build command
$buildArgs = @(
    "build"
    "-f", "docker/all-in-one.Dockerfile"
    "-t", "${IMAGE_NAME}:latest"
    "--progress=plain"
    "."
)

if ($NoCache) {
    $buildArgs = @("build", "--no-cache", "-f", "docker/all-in-one.Dockerfile", "-t", "${IMAGE_NAME}:latest", "--progress=plain", ".")
}

Write-Host "Starting build..." -ForegroundColor Cyan
Write-Host "This may take 10-20 minutes..." -ForegroundColor Yellow
Write-Host ""

# Run build and capture all output
$process = Start-Process -FilePath "docker" -ArgumentList $buildArgs -NoNewWindow -PassThru -RedirectStandardOutput "$LOG_FILE.stdout" -RedirectStandardError "$LOG_FILE.stderr"

# Wait for completion
$process.WaitForExit()

# Combine output
Get-Content "$LOG_FILE.stdout", "$LOG_FILE.stderr" | Out-File $LOG_FILE

# Display output with color coding
Write-Host ""
Write-Host "=======================================" -ForegroundColor Cyan
Write-Host "Build Output:" -ForegroundColor Cyan
Write-Host "=======================================" -ForegroundColor Cyan

$buildOutput = Get-Content $LOG_FILE
foreach ($line in $buildOutput) {
    if ($line -match "error\[E\d+") {
        Write-Host $line -ForegroundColor Red
    } elseif ($line -match "error:|Error:|ERROR:") {
        Write-Host $line -ForegroundColor Red
    } elseif ($line -match "warning:|Warning:|WARNING:") {
        Write-Host $line -ForegroundColor Yellow
    } elseif ($line -match "Compiling|Finished") {
        Write-Host $line -ForegroundColor Green
    } elseif ($line -match "===.*===") {
        Write-Host $line -ForegroundColor Cyan
    } else {
        Write-Host $line
    }
}

Write-Host ""
Write-Host "=======================================" -ForegroundColor Cyan

if ($process.ExitCode -eq 0) {
    Write-Host "Build completed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "  1. Run coordinator: .\docker-run.ps1 run-coordinator" -ForegroundColor Yellow
    Write-Host "  2. Run workers: .\docker-run.ps1 run-worker 0" -ForegroundColor Yellow
    Write-Host "  3. Check status: .\docker-run.ps1 status" -ForegroundColor Yellow
} else {
    Write-Host "Build failed with exit code $($process.ExitCode)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Full log saved to: $LOG_FILE" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "To view errors:" -ForegroundColor Cyan
    Write-Host "  Select-String -Path '$LOG_FILE' -Pattern 'error\[' -Context 2" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "To view last 50 lines:" -ForegroundColor Cyan
    Write-Host "  Get-Content '$LOG_FILE' | Select-Object -Last 50" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "For detailed troubleshooting, see: DEBUG_BUILD.md" -ForegroundColor Cyan
}

# Clean up temp files
Remove-Item "$LOG_FILE.stdout", "$LOG_FILE.stderr" -ErrorAction SilentlyContinue

exit $process.ExitCode

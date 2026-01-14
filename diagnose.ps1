# Quick diagnostic script to check Docker build issues

Write-Host "ChatLoop Docker Build Diagnostics" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

# Check 1: Docker is running
Write-Host "Checking Docker..." -ForegroundColor Yellow
try {
    $dockerVersion = docker version --format "{{.Server.Version}}" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Docker is running (version $dockerVersion)" -ForegroundColor Green
    } else {
        Write-Host "✗ Docker may not be running properly" -ForegroundColor Red
    }
} catch {
    Write-Host "✗ Docker is not running!" -ForegroundColor Red
    Write-Host "Start Docker Desktop and try again." -ForegroundColor Yellow
    exit 1
}

# Check 2: Available Docker resources
Write-Host ""
Write-Host "Checking Docker resources..." -ForegroundColor Yellow
docker info | Select-String -Pattern "CPUs|Total Memory" | ForEach-Object {
    Write-Host "  $_" -ForegroundColor White
}

# Check 3: Check for existing ChatLoop images
Write-Host ""
Write-Host "Checking existing ChatLoop images..." -ForegroundColor Yellow
$images = docker images --format "{{.Repository}}:{{.Tag}}" | Select-String "chatloop"
if ($images) {
    Write-Host "Found existing images:" -ForegroundColor Green
    $images | ForEach-Object { Write-Host "  - $_" -ForegroundColor White }
} else {
    Write-Host "No ChatLoop images found (this is normal for first build)" -ForegroundColor Gray
}

# Check 4: Check if source files exist
Write-Host ""
Write-Host "Checking project structure..." -ForegroundColor Yellow
$requiredFiles = @(
    "Cargo.toml",
    "crates/worker/Cargo.toml",
    "crates/coordinator/Cargo.toml",
    "crates/common/Cargo.toml",
    "crates/proto/Cargo.toml",
    "docker/all-in-one.Dockerfile"
)

$allExist = $true
foreach ($file in $requiredFiles) {
    if (Test-Path $file) {
        Write-Host "  ✓ $file" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $file (missing!)" -ForegroundColor Red
        $allExist = $false
    }
}

if (-not $allExist) {
    Write-Host ""
    Write-Host "ERROR: Some required files are missing!" -ForegroundColor Red
    exit 1
}

# Check 5: Check for recent build logs
Write-Host ""
Write-Host "Checking for recent build logs..." -ForegroundColor Yellow
$logs = Get-ChildItem -Path "." -Filter "docker-build-*.log" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 3
if ($logs) {
    Write-Host "Found recent build logs:" -ForegroundColor Green
    foreach ($log in $logs) {
        Write-Host "  - $($log.Name) ($($log.LastWriteTime))" -ForegroundColor White
    }
    Write-Host ""
    Write-Host "View most recent log:" -ForegroundColor Cyan
    Write-Host "  Get-Content '$($logs[0].Name)' | Select-Object -Last 50" -ForegroundColor Yellow
} else {
    Write-Host "No recent build logs found" -ForegroundColor Gray
}

# Check 6: Check git status
Write-Host ""
Write-Host "Checking git status..." -ForegroundColor Yellow
try {
    $gitStatus = git status --short 2>&1
    if ($LASTEXITCODE -eq 0) {
        $changes = git status --short | Measure-Object
        if ($changes.Count -gt 0) {
            Write-Host "  Uncommitted changes detected:" -ForegroundColor Yellow
            git status --short | ForEach-Object { Write-Host "    $_" -ForegroundColor White }
        } else {
            Write-Host "  ✓ Working directory clean" -ForegroundColor Green
        }
    }
} catch {
    Write-Host "  Git not available or not a git repository" -ForegroundColor Gray
}

# Check 7: System resources
Write-Host ""
Write-Host "System information..." -ForegroundColor Yellow
$os = Get-CimInstance -ClassName Win32_OperatingSystem
$cs = Get-CimInstance -ClassName Win32_ComputerSystem
Write-Host "  OS: $($os.Caption)" -ForegroundColor White
Write-Host "  RAM: $([math]::Round($cs.TotalPhysicalMemory / 1GB, 2)) GB" -ForegroundColor White
Write-Host "  CPUs: $($cs.NumberOfProcessors)" -ForegroundColor White

# Recommendations
Write-Host ""
Write-Host "==================================" -ForegroundColor Cyan
Write-Host "Recommendations:" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

if ($cs.TotalPhysicalMemory -lt 8GB) {
    Write-Host "⚠ Warning: Less than 8GB RAM detected" -ForegroundColor Yellow
    Write-Host "  Consider increasing Docker memory limit in Docker Desktop Settings" -ForegroundColor Yellow
    Write-Host ""
}

if ($cs.NumberOfProcessors -lt 4) {
    Write-Host "⚠ Warning: Less than 4 CPU cores detected" -ForegroundColor Yellow
    Write-Host "  Build may be slow" -ForegroundColor Yellow
    Write-Host ""
}

Write-Host "To build the Docker image, run:" -ForegroundColor Green
Write-Host "  .\build-docker.ps1" -ForegroundColor Yellow
Write-Host ""
Write-Host "For troubleshooting, see:" -ForegroundColor Green
Write-Host "  DEBUG_BUILD.md" -ForegroundColor Yellow
Write-Host ""

# ===================================================================
# williw 跨平台启动脚本 (PowerShell)
# ===================================================================
#
# 功能：
# - 一键启动 Rust 节点（Docker 容器）
# - 一键启动 GPU 推理服务（宿主机）
# - 适用于 Windows PowerShell
#
# 使用方式:
#   .\start.ps1             # 只启动 Rust 节点
#   .\start.ps1 -All        # 启动所有服务
#   .\start.ps1 -Node       # 只启动 Rust 节点
#   .\start.ps1 -Gpu        # 只启动 GPU 推理服务
#   .\start.ps1 -Clean      # 清理所有容器
#

$ErrorActionPreference = "Stop"

# 配置
$NodeName = "williw-node"
$GpuServicePort = 8000
$NodePort = 9235

# 颜色输出
function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# 检测 GPU 可用性
function Test-GpuAvailable {
    try {
        $result = python -c "import torch; print('cuda' if torch.cuda.is_available() else 'cpu')" 2>&1
        if ($result -like "*cuda*") {
            return "cuda"
        } elseif ($result -like "*cpu*") {
            return "cpu"
        }
    } catch {
        # Python 或 torch 未安装
    }
    return "cpu"
}

# 检查 Docker
function Test-DockerInstalled {
    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Write-Error "Docker 未安装，请先安装 Docker Desktop"
        Write-Info "下载地址：https://www.docker.com/products/docker-desktop"
        return $false
    }
    
    try {
        docker info | Out-Null
        Write-Success "Docker 检查通过"
        return $true
    } catch {
        Write-Error "Docker 未运行，请先启动 Docker Desktop"
        return $false
    }
}

# 检查 Python 依赖
function Test-PythonDeps {
    Write-Info "检查 Python 依赖..."
    
    try {
        python -c "import flask" 2>&1 | Out-Null
        Write-Success "Python 依赖检查通过"
        return $true
    } catch {
        Write-Warning "Flask 未安装，GPU 推理服务将无法运行"
        Write-Info "安装命令：pip install -r requirements-gpu.txt"
        return $false
    }
}

# 启动 GPU 推理服务
function Start-GpuService {
    $gpuType = Test-GpuAvailable
    Write-Info "检测到 GPU 模式：$gpuType"
    
    if (-not (Test-PythonDeps)) {
        Write-Warning "跳过 GPU 推理服务启动"
        return $false
    }
    
    # 检查端口是否被占用
    $existingProcess = Get-NetTCPConnection -LocalPort $GpuServicePort -ErrorAction SilentlyContinue
    if ($existingProcess) {
        Write-Warning "端口 $GpuServicePort 已被占用，GPU 推理服务可能已在运行"
        return $true
    }
    
    Write-Info "启动 GPU 推理服务 (端口：$GpuServicePort)..."
    
    # 启动进程
    $scriptPath = Join-Path $PSScriptRoot "gpu_inference_server_clean.py"
    $logPath = Join-Path $PSScriptRoot "gpu_service.log"
    $pidPath = Join-Path $PSScriptRoot "gpu_service.pid"
    
    $process = Start-Process -FilePath "python" `
        -ArgumentList $scriptPath, "--port", $GpuServicePort `
        -PassThru `
        -RedirectStandardOutput $logPath `
        -RedirectStandardError $logPath `
        -WindowStyle Hidden
    
    # 保存 PID
    $process.Id | Out-File -FilePath $pidPath -Encoding ASCII
    
    Start-Sleep -Seconds 2
    
    # 检查是否启动成功
    $process = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
    if ($process) {
        Write-Success "GPU 推理服务已启动 (PID: $($process.Id))"
        Write-Info "日志文件：$logPath"
        return $true
    } else {
        Write-Error "GPU 推理服务启动失败"
        return $false
    }
}

# 启动 Rust 节点（Docker）
function Start-Node {
    Write-Info "启动 Rust 节点 (Docker 容器)..."
    
    # Docker host（Windows 上用 host.docker.internal）
    $dockerHost = "host.docker.internal"
    
    Write-Info "GPU 推理服务地址：$dockerHost`:$GpuServicePort"
    
    # 停止旧容器
    docker stop $NodeName 2>$null | Out-Null
    docker rm $NodeName 2>$null | Out-Null
    
    # 构建镜像
    Write-Info "构建 Docker 镜像..."
    docker build -t williw-node .
    
    # 启动容器
    Write-Info "启动容器..."
    docker run -d `
        --name $NodeName `
        --network host `
        -e WILLIW_GPU_INFERENCE_URL=http://$dockerHost`:$GpuServicePort `
        -e RUST_LOG=info `
        -e WILLIW_DEVICE_TYPE=high `
        williw-node
    
    Start-Sleep -Seconds 3
    
    # 检查容器状态
    $container = docker ps --filter "name=$NodeName" --format "{{.Names}}"
    if ($container -like "*$NodeName*") {
        Write-Success "Rust 节点已启动"
        Write-Info "查看日志：docker logs -f $NodeName"
        return $true
    } else {
        Write-Error "Rust 节点启动失败"
        docker logs $NodeName
        return $false
    }
}

# 停止所有服务
function Stop-All {
    Write-Info "停止所有服务..."
    
    # 停止 GPU 服务
    $pidPath = Join-Path $PSScriptRoot "gpu_service.pid"
    if (Test-Path $pidPath) {
        $pid = Get-Content $pidPath
        $process = Get-Process -Id $pid -ErrorAction SilentlyContinue
        if ($process) {
            Stop-Process -Id $pid
            Write-Info "GPU 推理服务已停止 (PID: $pid)"
        }
        Remove-Item $pidPath
    }
    
    # 停止 Docker 容器
    docker stop $NodeName 2>$null | Out-Null
    Write-Info "Rust 节点已停止"
    
    Write-Success "所有服务已停止"
}

# 清理所有容器
function Clean-All {
    param([switch]$Force)
    
    Write-Info "清理 Docker 容器和镜像..."
    
    docker stop $NodeName 2>$null | Out-Null
    docker rm $NodeName 2>$null | Out-Null
    docker rmi williw-node 2>$null | Out-Null
    
    if ($Force) {
        docker volume rm williw-data williw-checkpoints 2>$null | Out-Null
        Write-Warning "数据卷已删除（数据丢失）"
    }
    
    Write-Success "清理完成"
}

# 显示状态
function Show-Status {
    $gpuType = Test-GpuAvailable
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  williw 系统状态" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "GPU 模式：$gpuType"
    Write-Host ""
    
    Write-Host "Docker 容器:" -ForegroundColor Yellow
    docker ps --filter "name=$NodeName" --format "table {{.Names}}`t{{.Status}}`t{{.Ports}}"
    Write-Host ""
    
    Write-Host "GPU 推理服务:" -ForegroundColor Yellow
    $pidPath = Join-Path $PSScriptRoot "gpu_service.pid"
    if (Test-Path $pidPath) {
        $pid = Get-Content $pidPath
        $process = Get-Process -Id $pid -ErrorAction SilentlyContinue
        if ($process) {
            Write-Host "✅ 运行中 (PID: $pid)" -ForegroundColor Green
        } else {
            Write-Host "❌ 已停止" -ForegroundColor Red
        }
    } else {
        Write-Host "❌ 未启动" -ForegroundColor Red
    }
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
}

# 显示帮助
function Show-Help {
    Write-Host @"
williw 跨平台启动脚本 (PowerShell)

使用方式:
  .\start.ps1 [选项]

选项:
  -All         启动所有服务（Rust 节点 + GPU 推理）
  -Node        只启动 Rust 节点（Docker）
  -Gpu         只启动 GPU 推理服务（宿主机）
  -Stop        停止所有服务
  -Clean       清理所有 Docker 容器和镜像
  -Status      显示系统状态
  -Help        显示此帮助信息

示例:
  .\start.ps1 -All              # 启动所有服务
  .\start.ps1 -Node             # 只启动 Rust 节点
  .\start.ps1 -Clean -Force     # 清理并删除数据

"@ -ForegroundColor White
}

# 主程序
param(
    [switch]$All,
    [switch]$Node,
    [switch]$Gpu,
    [switch]$Stop,
    [switch]$Clean,
    [switch]$Force,
    [switch]$Status,
    [switch]$Help
)

if ($Help) {
    Show-Help
    exit 0
}

if ($All) {
    if (-not (Test-DockerInstalled)) { exit 1 }
    Start-GpuService
    Start-Node
    Show-Status
    exit 0
}

if ($Node) {
    if (-not (Test-DockerInstalled)) { exit 1 }
    Start-Node
    Show-Status
    exit 0
}

if ($Gpu) {
    Start-GpuService
    exit 0
}

if ($Stop) {
    Stop-All
    exit 0
}

if ($Clean) {
    Clean-All -Force:$Force
    exit 0
}

if ($Status) {
    Show-Status
    exit 0
}

# 默认只启动节点
if (-not (Test-DockerInstalled)) { exit 1 }
Start-Node
Show-Status

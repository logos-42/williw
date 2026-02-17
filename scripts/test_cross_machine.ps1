# ===================================================================
# williw 跨机器 P2P 连接测试脚本 (PowerShell)
# ===================================================================
#
# 使用方式:
#   在电脑 A 上：.\scripts\test_cross_machine.ps1 -Host
#   在电脑 B 上：.\scripts\test_cross_machine.ps1 -Client -Target 192.168.1.100
#

$ErrorActionPreference = "Stop"

# 配置
$NodeName = "williw-node"
$TestDuration = 30
$BootstrapPort = 9235

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

# 显示帮助
function Show-Help {
    Write-Host @"
williw 跨机器 P2P 连接测试

使用方式:
  .\scripts\test_cross_machine.ps1 [选项]

选项:
  -Host               作为主机（第一个节点）
  -Client             作为客户端（连接主机）
  -Target <IP>        目标主机 IP（客户端模式）
  -Duration <秒>      测试持续时间（默认：30 秒）
  -Clean              清理测试环境
  -Help               显示帮助

示例:
  # 在电脑 A 上（作为主机）
  .\scripts\test_cross_machine.ps1 -Host

  # 在电脑 B 上（连接电脑 A）
  .\scripts\test_cross_machine.ps1 -Client -Target 192.168.1.100

  # 清理测试环境
  .\scripts\test_cross_machine.ps1 -Clean

"@ -ForegroundColor White
}

# 清理环境
function Clean-Test {
    Write-Info "清理测试环境..."
    docker stop $NodeName 2>$null | Out-Null
    docker rm $NodeName 2>$null | Out-Null
    Write-Success "清理完成"
}

# 测试 GPU 推理服务
function Test-GpuService {
    Write-Info "测试 GPU 推理服务连接..."
    
    $gpuUrl = $env:WILLIW_GPU_INFERENCE_URL
    if (-not $gpuUrl) {
        $gpuUrl = "http://localhost:8000"
    }
    
    try {
        $response = Invoke-WebRequest -Uri "$gpuUrl/" -TimeoutSec 5 -UseBasicParsing
        Write-Success "GPU 推理服务响应正常"
        Write-Info "状态：$($response.Content)"
    } catch {
        Write-Warning "GPU 推理服务未响应 ($gpuUrl)"
        Write-Info "启动 GPU 服务：.\start.ps1 -Gpu"
    }
}

# 启动主机节点
function Start-Host {
    Write-Info "启动主机节点..."
    
    # 停止旧容器
    docker stop $NodeName 2>$null | Out-Null
    docker rm $NodeName 2>$null | Out-Null
    
    # 获取本机 IP
    $hostIp = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.InterfaceAlias -notlike "*Loopback*" } | Select-Object -First 1).IPAddress
    Write-Info "本机 IP: $hostIp"
    
    # 启动容器
    docker run -d `
        --name $NodeName `
        --network host `
        -e WILLIW_NODE_ID=host-node `
        -e RUST_LOG=debug `
        -e WILLIW_DEVICE_TYPE=high `
        williw-node
    
    Start-Sleep -Seconds 5
    
    # 获取节点 ID
    $nodeId = docker logs $NodeName 2>&1 | Select-String "Node ID" | Select-Object -First 1
    if (-not $nodeId) {
        $nodeId = "未知"
    }
    
    Write-Success "主机节点已启动"
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "主机信息:"
    Write-Host "  IP 地址：$hostIp"
    Write-Host "  端口：$BootstrapPort"
    Write-Host "  节点 ID: $nodeId"
    Write-Host ""
    Write-Host "在另一台电脑上运行:"
    Write-Host "  .\scripts\test_cross_machine.ps1 -Client -Target $hostIp"
    Write-Host "========================================"
    Write-Host ""
    
    # 监听日志
    Write-Info "监听节点日志（按 Ctrl+C 停止）..."
    docker logs -f $NodeName
}

# 启动客户端节点
function Start-Client {
    param([string]$Target)
    
    if (-not $Target) {
        Write-Error "请指定目标主机 IP"
        exit 1
    }
    
    Write-Info "启动客户端节点，连接目标：$Target`:$BootstrapPort"
    
    # 停止旧容器
    docker stop $NodeName 2>$null | Out-Null
    docker rm $NodeName 2>$null | Out-Null
    
    # 启动容器
    docker run -d `
        --name $NodeName `
        --network host `
        -e WILLIW_NODE_ID=client-node `
        -e WILLIW_BOOTSTRAP=$Target`:$BootstrapPort `
        -e RUST_LOG=debug `
        -e WILLIW_DEVICE_TYPE=high `
        williw-node
    
    Start-Sleep -Seconds 5
    
    Write-Success "客户端节点已启动"
    
    # 监听连接状态
    Write-Info "监听连接状态..."
    
    $startTime = Get-Date
    $connected = $false
    
    for ($i = 0; $i -lt $TestDuration; $i += 2) {
        # 检查日志中的连接信息
        $logs = docker logs $NodeName 2>&1
        if ($logs -match "connected|peer") {
            Write-Success "检测到 P2P 连接!"
            $connected = $true
            break
        }
        
        Start-Sleep -Seconds 2
    }
    
    # 显示最终日志
    Write-Host ""
    Write-Info "最终日志:"
    docker logs $NodeName 2>&1 | Select-Object -Last 20
    
    if ($connected) {
        Write-Host ""
        Write-Success "跨机器 P2P 连接测试成功!"
    } else {
        Write-Host ""
        Write-Warning "未检测到 P2P 连接，请检查:"
        Write-Host "  1. 防火墙设置"
        Write-Host "  2. 目标主机是否在线"
        Write-Host "  3. 端口是否开放"
    }
    
    # 清理
    Write-Info "清理测试容器..."
    docker stop $NodeName 2>$null | Out-Null
    docker rm $NodeName 2>$null | Out-Null
}

# 主程序
param(
    [switch]$Host,
    [switch]$Client,
    [string]$Target,
    [int]$Duration = 30,
    [switch]$Clean,
    [switch]$Help
)

if ($Help) {
    Show-Help
    exit 0
}

if ($Clean) {
    Clean-Test
    exit 0
}

if ($Host) {
    Test-GpuService
    Start-Host
    exit 0
}

if ($Client) {
    Test-GpuService
    Start-Client -Target $Target
    exit 0
}

# 默认显示帮助
Show-Help

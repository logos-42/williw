# iroh跨网络P2P设置脚本
# 自动配置网络演示环境

param(
    [string]$Mode = "info",  # info, receiver, sender
    [string]$TargetIP = "",
    [string]$TargetNodeId = "",
    [string]$Message = "Hello from network demo!"
)

Write-Host "🌐 iroh跨网络P2P设置脚本" -ForegroundColor Green
Write-Host "============================" -ForegroundColor Green

# 检查Rust环境
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 错误: 未找到cargo命令，请先安装Rust" -ForegroundColor Red
    exit 1
}

# 构建网络演示程序
Write-Host "🔨 构建网络演示程序..." -ForegroundColor Blue
cargo build --example iroh_network_demo
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 构建失败" -ForegroundColor Red
    exit 1
}

Write-Host "✅ 构建成功" -ForegroundColor Green
Write-Host ""

switch ($Mode.ToLower()) {
    "info" {
        Write-Host "📋 显示网络信息" -ForegroundColor Yellow
        cargo run --example iroh_network_demo -- info
        
        Write-Host ""
        Write-Host "📖 使用说明:" -ForegroundColor Cyan
        Write-Host "1. 接收端模式: .\scripts\setup_network_demo.ps1 -Mode receiver" -ForegroundColor White
        Write-Host "2. 发送端模式: .\scripts\setup_network_demo.ps1 -Mode sender -TargetIP <IP> -TargetNodeId <节点ID>" -ForegroundColor White
    }
    
    "receiver" {
        Write-Host "🎯 启动接收端模式" -ForegroundColor Yellow
        Write-Host ""
        
        # 检查防火墙设置
        Write-Host "🔥 防火墙检查..." -ForegroundColor Blue
        try {
            $firewallRule = Get-NetFirewallRule -DisplayName "*iroh*" -ErrorAction SilentlyContinue
            if (-not $firewallRule) {
                Write-Host "⚠️ 未检测到iroh防火墙规则" -ForegroundColor Yellow
                Write-Host "💡 建议手动添加防火墙规则允许端口11207" -ForegroundColor Cyan
            } else {
                Write-Host "✅ 检测到防火墙规则" -ForegroundColor Green
            }
        } catch {
            Write-Host "⚠️ 无法检查防火墙状态（需要管理员权限）" -ForegroundColor Yellow
        }
        
        Write-Host ""
        Write-Host "🚀 启动接收端..." -ForegroundColor Green
        Write-Host "📋 请将显示的节点ID和IP地址发送给发送端" -ForegroundColor Cyan
        Write-Host ""
        
        cargo run --example iroh_network_demo -- receive --bind-ip 0.0.0.0 --port 11207
    }
    
    "sender" {
        if ([string]::IsNullOrEmpty($TargetIP) -or [string]::IsNullOrEmpty($TargetNodeId)) {
            Write-Host "❌ 发送端模式需要指定目标IP和节点ID" -ForegroundColor Red
            Write-Host "用法: .\scripts\setup_network_demo.ps1 -Mode sender -TargetIP <IP> -TargetNodeId <节点ID>" -ForegroundColor Yellow
            exit 1
        }
        
        Write-Host "📤 启动发送端模式" -ForegroundColor Yellow
        Write-Host "🎯 目标IP: $TargetIP" -ForegroundColor White
        Write-Host "🔑 目标节点ID: $TargetNodeId" -ForegroundColor White
        Write-Host "📨 消息: $Message" -ForegroundColor White
        Write-Host ""
        
        # 测试网络连通性
        Write-Host "🌐 测试网络连通性..." -ForegroundColor Blue
        $pingResult = Test-Connection -ComputerName $TargetIP -Count 2 -Quiet
        if ($pingResult) {
            Write-Host "✅ 网络连通性正常" -ForegroundColor Green
        } else {
            Write-Host "⚠️ 网络连通性测试失败，但仍会尝试连接" -ForegroundColor Yellow
        }
        
        Write-Host ""
        Write-Host "🚀 发送消息..." -ForegroundColor Green
        
        cargo run --example iroh_network_demo -- send --target $TargetNodeId --target-ip $TargetIP --target-port 11207 --message $Message
    }
    
    default {
        Write-Host "❌ 无效的模式: $Mode" -ForegroundColor Red
        Write-Host "有效模式: info, receiver, sender" -ForegroundColor Yellow
        exit 1
    }
}

Write-Host ""
Write-Host "🎉 脚本执行完成" -ForegroundColor Green
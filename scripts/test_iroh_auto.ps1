# 自动化iroh P2P测试脚本
# 自动启动接收端和发送端进行测试

param(
    [string]$Message = "Hello from automated test!"
)

Write-Host "🚀 自动化iroh P2P测试" -ForegroundColor Green
Write-Host "========================" -ForegroundColor Green

# 构建项目
Write-Host "🔨 构建项目..." -ForegroundColor Blue
cargo build --example iroh_simple_local
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 构建失败" -ForegroundColor Red
    exit 1
}

Write-Host "✅ 构建成功" -ForegroundColor Green

# 启动接收端作为后台任务
Write-Host "🎯 启动接收端..." -ForegroundColor Blue
$receiverJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    cargo run --example iroh_simple_local -- receive
}

# 等待接收端启动
Write-Host "⏳ 等待接收端启动..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# 获取接收端输出以提取节点ID
$receiverOutput = Receive-Job -Job $receiverJob -Keep
$nodeIdLine = $receiverOutput | Where-Object { $_ -match "节点ID:" }

if ($nodeIdLine) {
    # 提取节点ID
    $nodeId = ($nodeIdLine -split "节点ID: ")[1].Trim()
    Write-Host "🔑 检测到节点ID: $nodeId" -ForegroundColor Green
    
    # 等待一下确保接收端完全就绪
    Start-Sleep -Seconds 2
    
    # 启动发送端
    Write-Host "📤 发送消息..." -ForegroundColor Blue
    cargo run --example iroh_simple_local -- send --target $nodeId --message $Message
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "🎉 测试成功完成！" -ForegroundColor Green
    } else {
        Write-Host "❌ 发送失败" -ForegroundColor Red
    }
} else {
    Write-Host "❌ 无法获取节点ID" -ForegroundColor Red
    Write-Host "接收端输出:" -ForegroundColor Yellow
    $receiverOutput | ForEach-Object { Write-Host $_ -ForegroundColor White }
}

# 清理后台任务
Write-Host "🧹 清理后台任务..." -ForegroundColor Blue
Stop-Job -Job $receiverJob -ErrorAction SilentlyContinue
Remove-Job -Job $receiverJob -ErrorAction SilentlyContinue

Write-Host "✅ 测试完成" -ForegroundColor Green
# iroh本地P2P测试脚本
# 用于快速测试两个端口之间的iroh通信

Write-Host "🚀 iroh本地P2P测试脚本" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green

# 检查是否有Rust环境
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 错误: 未找到cargo命令，请先安装Rust" -ForegroundColor Red
    exit 1
}

Write-Host "📋 测试步骤:" -ForegroundColor Yellow
Write-Host "1. 在第一个终端启动接收端" -ForegroundColor White
Write-Host "2. 在第二个终端发送消息" -ForegroundColor White
Write-Host "3. 观察P2P通信结果" -ForegroundColor White
Write-Host ""

# 构建项目
Write-Host "🔨 构建项目..." -ForegroundColor Blue
cargo build --example iroh_local_demo
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 构建失败" -ForegroundColor Red
    exit 1
}

Write-Host "✅ 构建成功" -ForegroundColor Green
Write-Host ""

# 提供使用说明
Write-Host "📖 使用说明:" -ForegroundColor Yellow
Write-Host ""
Write-Host "步骤1: 在第一个终端运行接收端" -ForegroundColor Cyan
Write-Host "cargo run --example iroh_local_demo -- receive --port 11204" -ForegroundColor White
Write-Host ""
Write-Host "步骤2: 复制接收端显示的节点ID，然后在第二个终端运行发送端" -ForegroundColor Cyan
Write-Host "cargo run --example iroh_local_demo -- send --target <节点ID> --addr 127.0.0.1:11204 --message \"Hello iroh!\"" -ForegroundColor White
Write-Host ""

# 询问是否自动启动接收端
$choice = Read-Host "是否现在启动接收端? (y/n)"
if ($choice -eq "y" -or $choice -eq "Y") {
    Write-Host "🎯 启动接收端..." -ForegroundColor Green
    cargo run --example iroh_local_demo -- receive --port 11204
}
else {
    Write-Host "💡 请手动运行上述命令进行测试" -ForegroundColor Yellow
}
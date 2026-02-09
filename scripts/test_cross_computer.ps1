# 跨电脑P2P通信测试脚本

Write-Host "🌐 iroh跨电脑P2P通信测试" -ForegroundColor Green
Write-Host "=============================" -ForegroundColor Green

Write-Host ""
Write-Host "📋 使用步骤:" -ForegroundColor Yellow
Write-Host "1️⃣ 在电脑A上运行监听端" -ForegroundColor Cyan
Write-Host "2️⃣ 在电脑B上运行连接端" -ForegroundColor Cyan

Write-Host ""
Write-Host "🔧 命令示例:" -ForegroundColor Yellow

Write-Host ""
Write-Host "📍 查看网络信息:" -ForegroundColor Magenta
Write-Host "   cargo run --example iroh_cross_computer -- info" -ForegroundColor White

Write-Host ""
Write-Host "🎧 电脑A - 启动监听端:" -ForegroundColor Magenta
Write-Host "   cargo run --example iroh_cross_computer -- listen --bind-ip 0.0.0.0 --port 11208 --name `"Computer-A`"" -ForegroundColor White

Write-Host ""
Write-Host "📡 电脑B - 连接到电脑A:" -ForegroundColor Magenta
Write-Host "   cargo run --example iroh_cross_computer -- connect \\" -ForegroundColor White
Write-Host "     --target <电脑A的节点ID> \\" -ForegroundColor White
Write-Host "     --target-ip <电脑A的IP地址> \\" -ForegroundColor White
Write-Host "     --target-port 11208 \\" -ForegroundColor White
Write-Host "     --message `"Hello from Computer B!`"" -ForegroundColor White

Write-Host ""
Write-Host "🔥 重要提醒:" -ForegroundColor Red
Write-Host "   - 确保两台电脑在同一网络或可以互相访问" -ForegroundColor Yellow
Write-Host "   - 检查防火墙设置，允许端口11208通过" -ForegroundColor Yellow
Write-Host "   - 先在电脑A启动监听端，获取节点ID和IP" -ForegroundColor Yellow
Write-Host "   - 然后在电脑B使用获取的信息进行连接" -ForegroundColor Yellow

Write-Host ""
Write-Host "🚀 开始测试..." -ForegroundColor Green

# 检查是否可以编译
Write-Host "🔍 检查编译..." -ForegroundColor Cyan
$compileResult = cargo check --example iroh_cross_computer 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 编译检查通过" -ForegroundColor Green
} else {
    Write-Host "❌ 编译检查失败:" -ForegroundColor Red
    Write-Host $compileResult -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "📋 现在你可以:" -ForegroundColor Green
Write-Host "1. 运行 'cargo run --example iroh_cross_computer -- info' 查看网络信息" -ForegroundColor White
Write-Host "2. 在电脑A运行监听端" -ForegroundColor White
Write-Host "3. 在电脑B运行连接端" -ForegroundColor White
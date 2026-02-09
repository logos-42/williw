# 手动iroh P2P测试脚本
# 提供简单的测试指令

Write-Host "🚀 iroh P2P手动测试指南" -ForegroundColor Green
Write-Host "========================" -ForegroundColor Green

# 检查构建
Write-Host "🔨 检查构建状态..." -ForegroundColor Blue
cargo check --example iroh_simple_local
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 构建失败，请先修复编译错误" -ForegroundColor Red
    exit 1
}

Write-Host "✅ 构建成功" -ForegroundColor Green
Write-Host ""

Write-Host "📖 手动测试步骤:" -ForegroundColor Yellow
Write-Host ""

Write-Host "步骤1: 打开第一个终端窗口，运行接收端" -ForegroundColor Cyan
Write-Host "命令: cargo run --example iroh_simple_local -- receive" -ForegroundColor White
Write-Host ""

Write-Host "步骤2: 等待接收端完全启动，复制显示的节点ID" -ForegroundColor Cyan
Write-Host "节点ID格式类似: k51qzi5uqu5dh71qgwangbdxj7u6fqkwkzs..." -ForegroundColor Gray
Write-Host ""

Write-Host "步骤3: 打开第二个终端窗口，运行发送端" -ForegroundColor Cyan
Write-Host "命令模板: cargo run --example iroh_simple_local -- send --target <节点ID>" -ForegroundColor White
Write-Host "示例: cargo run --example iroh_simple_local -- send --target k51qzi5uqu5dh71qgwangbdxj7u6fqkwkzs... --message \"Hello iroh!\"" -ForegroundColor Gray
Write-Host ""

Write-Host "🔍 预期结果:" -ForegroundColor Yellow
Write-Host "- 接收端应该显示: 📨 收到消息: Hello iroh!" -ForegroundColor White
Write-Host "- 发送端应该显示: 🎉 消息发送成功！" -ForegroundColor White
Write-Host ""

Write-Host "🐛 故障排除:" -ForegroundColor Yellow
Write-Host "1. 如果连接失败，请确保两个终端都在同一台机器上运行" -ForegroundColor White
Write-Host "2. 检查防火墙设置，确保允许本地连接" -ForegroundColor White
Write-Host "3. 确保节点ID完整复制，没有遗漏字符" -ForegroundColor White
Write-Host "4. 如果仍然失败，尝试重启接收端" -ForegroundColor White
Write-Host ""

Write-Host "💡 提示:" -ForegroundColor Cyan
Write-Host "- 可以自定义消息内容：--message \"你的消息\"" -ForegroundColor White
Write-Host "- 接收端会在收到一条消息后自动退出" -ForegroundColor White
Write-Host "- 每次测试都需要重新启动接收端" -ForegroundColor White
Write-Host ""

$choice = Read-Host "是否现在启动接收端进行测试? (y/n)"
if ($choice -eq "y" -or $choice -eq "Y") {
    Write-Host "🎯 启动接收端..." -ForegroundColor Green
    Write-Host "请在另一个终端窗口准备发送命令" -ForegroundColor Yellow
    Write-Host ""
    cargo run --example iroh_simple_local -- receive
}
else {
    Write-Host "💡 请按照上述步骤手动进行测试" -ForegroundColor Yellow
}
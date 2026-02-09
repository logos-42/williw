# 拆分后合约部署脚本 (PowerShell)
# 部署顺序：共享类型 -> 节点管理 -> 贡献跟踪 -> 收益管理 -> 治理

Write-Host "🚀 开始部署拆分后的智能合约..." -ForegroundColor Green

# 1. 构建所有合约
Write-Host "📦 构建所有合约..." -ForegroundColor Blue
anchor build --config Anchor-modular.toml

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 构建失败" -ForegroundColor Red
    exit 1
}

# 2. 部署节点管理合约
Write-Host "👤 部署节点管理合约..." -ForegroundColor Blue
anchor deploy node-management --config Anchor-modular.toml

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 节点管理合约部署失败" -ForegroundColor Red
    exit 1
}

# 3. 部署贡献跟踪合约
Write-Host "📊 部署贡献跟踪合约..." -ForegroundColor Blue
anchor deploy contribution-tracking --config Anchor-modular.toml

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 贡献跟踪合约部署失败" -ForegroundColor Red
    exit 1
}

# 4. 部署收益管理合约
Write-Host "💰 部署收益管理合约..." -ForegroundColor Blue
anchor deploy reward-management --config Anchor-modular.toml

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 收益管理合约部署失败" -ForegroundColor Red
    exit 1
}

# 5. 部署治理合约
Write-Host "🏛️ 部署治理合约..." -ForegroundColor Blue
anchor deploy governance --config Anchor-modular.toml

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 治理合约部署失败" -ForegroundColor Red
    exit 1
}

Write-Host "✅ 所有合约部署完成！" -ForegroundColor Green

# 6. 显示部署的程序ID
Write-Host "📋 部署的程序ID：" -ForegroundColor Yellow
solana program show --programs | Select-String "node_management|contribution_tracking|reward_management|governance"

Write-Host "🎉 拆分后合约部署成功完成！" -ForegroundColor Green

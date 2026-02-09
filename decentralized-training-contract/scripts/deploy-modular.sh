#!/bin/bash

# 拆分后合约部署脚本
# 部署顺序：共享类型 -> 节点管理 -> 贡献跟踪 -> 收益管理 -> 治理

set -e

echo "🚀 开始部署拆分后的智能合约..."

# 1. 构建所有合约
echo "📦 构建所有合约..."
anchor build --config Anchor-modular.toml

# 2. 部署共享类型库（如果需要）
echo "🔧 部署共享类型库..."
# 共享类型库通常不需要单独部署，作为依赖库使用

# 3. 部署节点管理合约
echo "👤 部署节点管理合约..."
anchor deploy node-management --config Anchor-modular.toml

# 4. 部署贡献跟踪合约
echo "📊 部署贡献跟踪合约..."
anchor deploy contribution-tracking --config Anchor-modular.toml

# 5. 部署收益管理合约
echo "💰 部署收益管理合约..."
anchor deploy reward-management --config Anchor-modular.toml

# 6. 部署治理合约
echo "🏛️ 部署治理合约..."
anchor deploy governance --config Anchor-modular.toml

echo "✅ 所有合约部署完成！"

# 7. 显示部署的程序ID
echo "📋 部署的程序ID："
solana program show --programs | grep -E "(node_management|contribution_tracking|reward_management|governance)"

echo "🎉 拆分后合约部署成功完成！"

# Solana 区块链集成模块

本模块提供与 Solana 区块链的完整集成功能，支持去中心化训练节点的算力贡献记录、收益分配和智能合约交互。

## 🚀 功能特性

### 核心功能
- **节点管理** - 注册、更新状态、查询节点信息
- **算力贡献** - 记录和验证节点的算力贡献
- **收益分配** - 自动化收益计算和分配
- **质押系统** - 代币质押和解除质押
- **多签管理** - 支持多签交易和治理

### 智能合约集成
- **Anchor 框架** - 基于 Anchor 的智能合约开发
- **类型安全** - 完整的类型定义和序列化支持
- **PDA 管理** - 自动计算和管理程序派生地址
- **错误处理** - 完善的错误处理和重试机制

## 📋 模块结构

```
src/solana/
├── mod.rs              # 模块入口和配置
├── types.rs            # 数据类型定义
├── client.rs           # Solana 客户端
├── accounts.rs         # 智能合约账户结构
├── instruction.rs      # 智能合约指令定义
├── compute.rs          # 算力贡献管理
├── rewards.rs          # 收益分配管理
├── tests/              # 集成测试
└── README.md           # 本文档
```

## 🔧 快速开始

### 1. 配置 Solana 客户端

```rust
use williw::solana::{SolanaClient, SolanaConfig, SolanaNetwork};

// 创建配置
let config = SolanaConfig::devnet("4SLjWwRYgRRdr4i5pgfjcbZEswXZRDcZ31BT1gipYdPq");

// 创建客户端
let client = SolanaClient::new(config, "my_node_id".to_string())?;
```

### 2. 注册节点

```rust
use williw::solana::{NodeInfo, NodeStatus};

let node_info = NodeInfo {
    node_id: "node_123".to_string(),
    owner_address: "owner_456".to_string(),
    name: "My Training Node".to_string(),
    device_type: "Desktop".to_string(),
    registered_at: chrono::Utc::now().timestamp(),
    last_active_at: chrono::Utc::now().timestamp(),
    status: NodeStatus::Active,
};

let result = client.register_node(node_info).await?;
if result.success {
    println!("节点注册成功: {}", result.signature);
}
```

### 3. 上报算力贡献

```rust
use williw::solana::ComputeContribution;

let contribution = ComputeContribution {
    id: "contrib_123".to_string(),
    node_id: "node_123".to_string(),
    task_id: "task_456".to_string(),
    start_timestamp: start_time,
    end_timestamp: end_time,
    duration_seconds: 3600,
    avg_gpu_usage_percent: 75.5,
    gpu_memory_used_mb: 1024,
    avg_cpu_usage_percent: 45.2,
    memory_used_mb: 2048,
    network_upload_mb: 100,
    network_download_mb: 200,
    samples_processed: 10000,
    batches_processed: 50,
    compute_score: 2.5,
};

let result = client.report_compute_contribution(contribution).await?;
```

### 4. 查询收益

```rust
let balance = client.get_wallet_balance("wallet_address").await?;
println!("待结算收益: {} lamports", balance.pending_rewards_lamports);
```

## 🏗️ 智能合约

### 合约地址
- **开发网**: `4SLjWwRYgRRdr4i5pgfjcbZEswXZRDcZ31BT1gipYdPq`
- **本地网**: `4SLjWwRYgRRdr4i5pgfjcbZEswXZRDcZ31BT1gipYdPq`

### 主要指令
- `initialize` - 初始化合约
- `register_node` - 注册新节点
- `record_contribution` - 记录算力贡献
- `distribute_rewards` - 分配收益
- `stake_tokens` - 质押代币
- `unstake_tokens` - 解除质押
- `verify_contribution` - 验证贡献
- `slash_node` - 罚没恶意节点

## 📊 数据类型

### 节点信息
```rust
pub struct NodeInfo {
    pub node_id: String,
    pub owner_address: String,
    pub name: String,
    pub device_type: String,
    pub registered_at: i64,
    pub last_active_at: i64,
    pub status: NodeStatus,
}
```

### 算力贡献
```rust
pub struct ComputeContribution {
    pub id: String,
    pub node_id: String,
    pub task_id: String,
    pub duration_seconds: u64,
    pub compute_score: f64,
    // ... 更多字段
}
```

### 收益分配
```rust
pub struct RewardDistribution {
    pub id: String,
    pub node_id: String,
    pub amount_lamports: u64,
    pub distributed_at: i64,
    pub status: RewardStatus,
}
```

## 🔐 安全特性

### 质押机制
- 最低质押要求：0.001 SOL
- 锁定期：最少 7 天
- 罚没机制：支持按比例罚没

### 验证系统
- 贡献验证：需要验证者确认
- 信誉评分：基于贡献质量计算
- 等级系统：0-5 级验证等级

### 多签支持
- 管理员操作需要多签确认
- 支持自定义阈值
- 防止单点故障

## 🧪 测试

### 运行集成测试
```bash
cargo test solana::tests::integration_test
```

### 本地测试环境
1. 启动本地 Solana 验证器：
```bash
solana-test-validator
```

2. 部署智能合约：
```bash
anchor build
anchor deploy
```

3. 运行测试：
```bash
cargo test
```

## 📈 性能优化

### 客户端优化
- 连接池管理
- 请求缓存
- 批量操作支持

### 交易优化
- 并行交易处理
- 智能重试机制
- Gas 费用优化

## 🚨 注意事项

### 网络配置
- **主网**: 生产环境，需要真实 SOL
- **开发网**: 测试环境，使用测试 SOL
- **本地网**: 本地测试，需要运行验证器

### 密钥管理
- 支持文件系统密钥
- 支持环境变量密钥
- 建议使用硬件钱包

### 错误处理
- 网络连接错误
- 交易失败处理
- 账户状态检查

## 📚 更多资源

- [Solana 官方文档](https://docs.solana.com/)
- [Anchor 框架文档](https://anchor-lang.com/)
- [智能合约源码](../decentralized-training-contract/)
- [集成测试示例](tests/integration_test.rs)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进 Solana 集成模块！

---

*本模块是 williw 去中心化训练平台的重要组成部分，为节点提供完整的区块链交互功能。*

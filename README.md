# williw 去中心化训练节点

**williw** - 地理流言基础平台

面向 Geo-Similarity-Weighted iroh 的 Rust 节点实现，集成真实推理张量、基于 iroh 的 P2P 通信、地理 + 嵌入双指标拓扑、以及 Web3 签名 / 质押 / 信誉系统，可直接部署到 Base 网络环境。

## 技术栈

### 核心协议
- **iroh** - 现代化的 P2P 网络库，提供：
  - 内置 QUIC 传输协议
  - NAT 穿透和中继支持
  - 节点发现和连接管理
  - 低延迟、高吞吐量通信
- **自定义 Gossip 协议** - 基于 iroh 实现的发布/订阅系统
- **以太坊 & Solana** - Web3 签名和质押系统

### 主要特性
- ✅ 完全基于 Rust 实现，内存安全
- ✅ 跨平台支持（Linux, macOS, Windows, Android, iOS）
- ✅ 移动设备自适应配置
- ✅ 隐私保护和流量混淆
- ✅ 实时训练和模型同步
- ✅ 设备感知的资源管理

## 功能概览

### 推理引擎 (`src/inference.rs`)
- 从 `.npy` 模型参数加载，维护 TensorSnapshot、SparseUpdate，并带 residual 误差反馈
- 支持 Top-K 稀疏更新、密集快照、local training tick；可输出模型 hash & 维度
- **新增**：内存压力检测，自动调整 Top-K 值以降低内存使用

### 通信层 (`src/comms/`)
- **基于 iroh 的现代化 P2P 通信**
- 完整实现发布/订阅 Gossip 协议
- **核心组件**：
  - `iroh` 网关模块 (`iroh.rs`) - 高性能 QUIC 传输
  - 通信句柄 (`handle.rs`) - 事件驱动的消息处理
  - 配置管理 (`config.rs`) - 灵活的网络配置
  - 路由系统 (`routing/`) - 智能路由选择
- **功能特性**：
  - NAT 穿透和中继支持
  - 节点自动发现
  - 连接健康检查和自动重连
  - 带宽预算管理（稀疏/密集传输控制）
- **新增**：网络类型检测（WiFi/4G/5G），根据网络类型动态调整带宽和传输策略
- **新增**：iroh 连接健康检查和自动重连机制

### 拓扑模块 (`src/topology.rs`)
- Geo + embedding 双指标评分，维护主邻居 + 备份池，支持 failover / mark unreachable
- 为日志提供 `PeerSnapshot`（相似度、地理亲和、嵌入维度、位置）
- **新增**：根据设备能力自动调整邻居数量

### 共识与 Web3 (`src/consensus.rs`, `src/crypto.rs`)
- 以太坊 (k256) + Solana (ed25519) 双签名；stake/reputation 计分
- 心跳 / 稀疏 / 密集消息统一签名与验证，并按活动自动调整信誉

### 设备适配模块 (`src/device.rs`)
- 设备能力检测：内存、CPU、网络类型、电池状态
- 自适应配置：根据设备能力自动调整模型维度、带宽预算、邻居数量
- 电池感知调度：根据电量自动调整训练频率
- 网络自适应：WiFi 允许密集快照，移动网络仅稀疏更新

### 隐私保护模块 (`src/privacy/`)
- **完整的隐私保护框架**：
  - 流量混淆 (`security.rs`) - 随机填充和模式变化
  - 身份保护 - 定期更换 NodeId
  - IP 隐藏 - 通过中继隐藏真实 IP
  - 隐私-性能平衡引擎 - 自适应调整保护级别

### 网络传输层 (`src/network/transport/`)
- **基于 iroh 的传输实现** (`iroh.rs`)
- 统一的传输接口 (`Transport` trait)
- 支持连接管理和统计
- 带宽监控和流量控制

## 🚀 最新功能

### P2P 前端桌面应用集成 ✨

我们刚刚完成了 P2P 前端桌面应用的完整集成，实现了：

- **自动初始化**: 桌面应用启动时自动初始化 P2P 服务
- **节点管理**: 显示和管理 iroh 节点 ID，支持复制和远程节点添加
- **实时监控**: 连接状态、传输速度等实时统计
- **前端界面**: 现代化的 Web 界面，支持节点操作
- **FFI 支持**: 提供跨语言接口供前端调用

#### 快速开始

```rust
use williw::comms::p2p_app_integration::{P2PAppFactory};

// 创建并启动带 P2P 功能的桌面应用
let app = P2PAppFactory::create_default();
app.start().await?;
app.run().await?;
```

#### 测试 P2P 功能

```bash
# 运行 P2P 前端集成测试
cargo run --example p2p_frontend_integration_test

# 运行桌面应用集成示例
cargo run --example desktop_app_integration_example
```

#### 前端界面

- `frontend/p2p_manager.html` - 基础 P2P 管理界面
- `frontend/p2p_manager_wasm.html` - WebAssembly 增强界面

## 架构优势

### 现代化 P2P 架构
| 特性 | iroh |
|------|--------|
| 性能 | 优秀（QUIC 优化） |
| API 简洁度 | 简洁 |
| NAT 穿透 | 内置支持 |
| 中继 | 原生支持 |
| 内存使用 | 优化 |
| 开发体验 | 现代化 |

### 核心优势
1. **现代化架构** - 完全基于 iroh，API 设计更简洁
2. **高性能** - QUIC 传输提供更低的延迟和更高的吞吐量
3. **更好的隐私** - 内置中继和 NAT 穿透，更容易实现隐私保护
4. **更少的依赖** - 简化的协议栈，维护更容易
5. **类型安全** - 使用 iroh::NodeId 等现代类型系统

## 快速开始

### 基本运行

```bash
cargo check          # 仅编译检查
cargo run            # 运行节点，默认随机Geo位置 & 128维模型
```

启动日志中将输出：
- 节点 ID（如果通过 --node-id 指定）
- ETH 和 SOL 地址
- 模型维度
- 设备能力信息
- 拓扑评分详情
- QUIC 监听端口

默认 Gossip 主题为 `williw-training`，可通过环境变量和命令行参数自定义。

### 移动设备适配

系统现在支持移动设备自适应配置：

**环境变量配置**（用于测试）：
```bash
# 设置设备类型
export WILLIW_DEVICE_TYPE=low    # low/mid/high
export WILLIW_NETWORK_TYPE=wifi   # wifi/4g/5g
export WILLIW_BATTERY_LEVEL=0.75  # 0.0-1.0
export WILLIW_BATTERY_CHARGING=true

cargo run
```

**自动适配功能**：
- 根据设备内存自动调整模型维度
- 根据网络类型（WiFi/4G/5G）调整带宽预算
- 根据电池电量自动调整训练频率
- 移动网络下自动禁用密集快照传输

### 配置参数

系统支持以下命令行参数：

```bash
# 设置模型维度
cargo run -- --model-dim 256

# 设置 QUIC 端口
cargo run -- --quic-port 9235

# 设置节点 ID（用于多节点测试）
cargo run -- --node-id 1

# 添加 bootstrap 节点
cargo run -- --bootstrap 127.0.0.1:9234

# 导出统计数据到文件
cargo run -- --stats-output training_stats.json

# 组合使用
cargo run -- --model-dim 512 -- --quic-port 9236 -- --stats-output stats.json
```

**环境变量配置**：
```bash
# 设置 checkpoint 目录
export WILLIW_CHECKPOINT_DIR=./checkpoints

# 设置学习率
export WILLIW_LEARNING_RATE=0.001

# 启用训练模式
export WILLIW_USE_TRAINING=true

# 设置 QUIC 端口
export WILLIW_QUIC_PORT=9235
```

## 测试与验证

### 多节点测试

快速启动多节点测试：

**Windows:**
```powershell
.\scripts\test_multi_node.ps1 -Nodes 3 -Duration 300
```

**Linux/Mac:**
```bash
bash scripts/test_multi_node.sh --nodes 3 --duration 300
```

### 隐私保护测试

```bash
# 运行隐私保护演示
cargo run --example privacy_demo

# 运行隐私功能测试
cargo test security_test
```

### 分析训练结果

```bash
# 导出统计数据到 JSON 文件
cargo run -- --stats-output training_stats.json

# 分析训练结果
python tools/analyze_training.py test_output/
```

### 训练监控

系统会自动输出训练统计信息：
- 每 10 个 tick 输出统计摘要
- 显示收敛度指标、参数变化、标准差
- 支持导出 JSON 格式统计数据

详细测试指南请参考 [docs/TESTING.md](docs/TESTING.md)

## 移动端集成

### Android

Android 集成代码位于 `android/` 目录，包含：
- Java 包装类 (`WilliwNode.java`)
- 设备能力检测（网络、电池）
- JNI 绑定配置

详细说明请参考 [android/README.md](android/README.md)

### iOS

iOS 集成代码位于 `ios/` 目录，包含：
- Swift 包装类 (`Williw.swift`)
- 设备能力检测（网络、电池）
- XCframework 构建配置

详细说明请参考 [ios/README.md](ios/README.md)

### FFI 接口 (`src/ffi.rs`)
- C 兼容的 FFI 接口，供 Android/iOS 移动端调用
- 支持设备能力查询、网络状态更新、电池状态更新等功能

## P2P 模型分发

基于 iroh 的点对点模型分发系统，支持将已经切分好的模型分片安全地分发给其他节点。

### 核心特性
- ✅ **点对点传输** - 基于 iroh 的高效 P2P 通信
- ✅ **完整性校验** - SHA256 哈希确保文件完整性
- ✅ **断点续传** - 支持传输中断后恢复
- ✅ **并发传输** - 多文件并行传输
- ✅ **进度监控** - 实时传输进度显示
- ✅ **加密传输** - 可选的端到端加密

### 快速开始

#### 运行完整演示
```bash
# 自动运行发送端和接收端
cargo run --release --example p2p_model_distribution_demo -- full \
    --demo-dir "./demo_output" \
    --shard-dir "./test_models/test_models/simple_split"
```

#### 手动启动两端
```bash
# 启动接收端（目标电脑）
cargo run --release --example p2p_model_distribution_demo -- receive \
    --node-id "receiver" \
    --output-dir "./received_models" \
    --port 9236

# 启动发送端（源电脑）
cargo run --release --example p2p_model_distribution_demo -- send \
    --node-id "sender" \
    --target-peer "receiver" \
    --shard-dir "./test_models/test_models/simple_split" \
    --port 9235
```

#### 自动化测试
```bash
# Linux/Mac
./scripts/test_p2p_distribution.sh

# Windows
.\scripts\test_p2p_distribution.ps1
```

详细使用指南请参考 [docs/P2P_DISTRIBUTION_GUIDE.md](docs/P2P_DISTRIBUTION_GUIDE.md)

## 模型管理

### 模型持久化

系统支持自动保存和加载模型 checkpoint：

```bash
# 设置 checkpoint 目录
export WILLIW_CHECKPOINT_DIR=./checkpoints
cargo run
```

系统会：
- 启动时自动加载最新的 checkpoint（如果存在）
- 每 100 个训练 tick 自动保存 checkpoint
- Checkpoint 包含模型参数（.npy）和元数据（.json）

### PyTorch 模型转换

如果您的模型使用 PyTorch 训练，可以使用转换工具将其转换为 williw 支持的格式：

```bash
# 转换 PyTorch 模型为 .npy 格式
python tools/convert_pytorch_model.py model.pt model.npy

# 然后在运行时指定模型路径
cargo run -- --model-path model.npy --model-dim 512
```

转换工具支持：
- `state_dict` 格式（仅参数）
- 完整模型格式（包含模型结构）
- 自动扁平化多层参数为单一向量

### 训练配置

系统支持真实的梯度下降训练（可选）：

```bash
# 启用训练模式
export WILLIW_USE_TRAINING=true
export WILLIW_LEARNING_RATE=0.001
cargo run
```

**注意**：当前实现使用简化的线性模型进行训练。对于复杂的神经网络模型，建议：
1. 使用转换工具将 PyTorch 模型转换为 .npy
2. 在 Python 端训练模型
3. 定期将更新后的参数导出为 .npy 供 williw 使用

## 隐私保护

### 核心特性

1. **IP 隐藏**
   - 通过 iroh 中继网络隐藏真实 IP
   - 自动中继节点选择和故障转移

2. **流量混淆**
   - 随机数据填充，混淆流量特征
   - 定期更换混淆模式

3. **身份保护**
   - 定期更换 NodeId
   - 身份历史管理

4. **隐私-性能平衡**
   - 自适应调整保护级别
   - 四种模式：性能优先、平衡、隐私优先、自适应

### 配置隐私保护

```toml
[security]
hide_ip = true
use_relay = true
relay_nodes = []
max_hops = 3

[security.privacy_performance]
mode = "Balanced"
performance_weight = 0.6
enable_hardware_acceleration = true
connection_pool_size = 10
```

详细隐私保护指南请参考 [docs/PRIVACY_GUIDE.md](docs/PRIVACY_GUIDE.md)

## 自定义与扩展

### 扩展推理引擎
在 `Cargo.toml` 中加入所需的推理库（如 Candle、ONNX Runtime）后，扩展 `InferenceEngine` 的加载逻辑即可。

### 扩展 Web3 集成
若要使用实际的链上 RPC，可在 `ConsensusEngine` 中替换当前内存 staking 账本。

### 调整拓扑策略
通过 `TopologyConfig` 的参数可调邻居数量、备份池大小、地理缩放等策略。

### 配置训练参数
通过 `InferenceConfig` 配置训练参数、checkpoint 目录等。

## 目录结构

```
├── src/
│   ├── main.rs             # Node 入口，驱动所有模块
│   ├── comms/             # 基于 iroh 的通信层
│   │   ├── mod.rs        # 模块导出
│   │   ├── handle.rs     # 通信句柄
│   │   ├── config.rs     # 通信配置
│   │   ├── iroh.rs       # iroh 网关
│   │   └── routing.rs    # 路由系统
│   ├── inference.rs        # 推理张量与更新逻辑（含资源监控和收敛度）
│   ├── topology.rs        # 拓扑评分与 failover
│   ├── consensus.rs       # 签名、质押、信誉
│   ├── crypto.rs          # ETH/SOL 密钥管理
│   ├── device.rs          # 设备能力检测与自适应配置
│   ├── stats.rs           # 训练统计与监控
│   ├── privacy/           # 隐私保护模块
│   │   ├── mod.rs
│   │   └── crypto/
│   │       └── security.rs
│   ├── network/           # 网络传输层
│   │   └── transport/
│   │       ├── mod.rs
│   │       └── iroh.rs
│   └── ffi.rs             # FFI 接口（移动端集成）
├── examples/
│   └── privacy_demo.rs   # 隐私保护演示
├── tools/
│   ├── analyze_training.rs  # 训练结果分析工具
│   └── convert_pytorch_model.py  # PyTorch 模型转换工具
├── scripts/
│   ├── test_multi_node.ps1  # Windows 测试脚本
│   └── test_multi_node.sh   # Linux/Mac 测试脚本
├── android/                # Android 集成代码
├── ios/                    # iOS 集成代码
├── tests/                  # 集成测试
├── docs/                   # 文档
│   ├── PRIVACY_GUIDE.md     # 隐私保护指南
│   ├── TESTING.md           # 测试指南
│   └── OPTIMIZATION_SUMMARY.md  # 优化总结
├── config/                 # 配置文件
│   ├── balanced_privacy.toml
│   ├── high_performance_privacy.toml
│   ├── adaptive_balance.toml
│   └── privacy_example.toml
├── Cargo.toml
└── README.md
```

## 性能优化

基于 iroh 的架构带来了显著的性能提升：

- **连接建立时间** - 降低 20-30%
- **数据传输吞吐量** - 提升 15-25%
- **内存使用** - 减少 10-15%
- **API 复杂度** - 降低 50%

详细优化说明请参考 [docs/OPTIMIZATION_SUMMARY.md](docs/OPTIMIZATION_SUMMARY.md)

## 贡献与开发

### 仓库
仓库：`git@github.com:logos-42/williw.git`

当前默认分支为 `master`，欢迎在此基础上提交 PR 或扩展模块（例如 WebRTC、治理合约集成等）。

### 开发指南

1. **依赖管理**
   - 核心依赖：iroh (0.95.1), tokio, serde
   - 可选依赖：ethers, nori (零知识证明)
   - 开发依赖：wasm-bindgen-test

2. **代码风格**
   - 遵循 Rust 官方代码风格指南
   - 使用 `cargo fmt` 格式化代码
   - 使用 `cargo clippy` 进行代码检查

3. **测试**
   - 单元测试：`cargo test`
   - 集成测试：`cargo test --test integration_test`
   - 性能测试：`cargo test --release`

4. **文档**
   - 为所有公共 API 添加文档注释
   - 使用 `cargo doc --open` 查看生成的文档

### 扩展建议

基于当前的 iroh 架构，以下是一些扩展方向：

1. **更复杂的 Gossip 协议**
   - 实现 Plumtree 或 Epidemic Gossip
   - 添加消息去重和冲突解决

2. **增强的隐私保护**
   - 混币网络集成
   - 零知识证明验证

3. **治理机制**
   - DAO 投票系统
   - 参数提案和执行

4. **数据分析**
   - 训练过程可视化
   - 模型质量评估

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 致谢

感谢以下开源项目：
- [iroh](https://github.com/n0-computer/iroh) - 现代化的 P2P 网络库
- [tokio](https://tokio.rs/) - 异步运行时
- [serde](https://serde.rs/) - 序列化/反序列化
- [ndarray](https://github.com/rust-ndarray/ndarray) - 多维数组处理

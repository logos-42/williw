# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-03-02

### Added

- **Agent 模块化重构** - 遵循人月神话原则的大型重构
  - `agent_commands.rs` 从 1809 行精简到 453 行 (-75%)
  - 新增 `agent_tools/executors/` 模块，按功能分类工具执行器
  - 新增 `agent/setup.rs` 辅助函数模块
  - 新增 `agent/chat.rs` 本地聊天命令模块

- **Skills 系统集成** - 全局技能管理
  - 支持从全局 `skills/` 目录加载技能定义
  - 内置 3 个专家技能：compute_expert, model_downloader, system_checker
  - 支持 SKILL.md 格式规范

- **Task 系统增强** - 支持 Agent Swarm 协作
  - 新增三种执行模式：sequential, parallel, swarm
  - Swarm 策略：Hierarchical, Broadcast, Shard, Vote
  - 内置分布式推理、模型下载任务模板

### Changed

- **架构优化**
  - 工具定义与执行逻辑分离 (`agent_tools/definitions.rs`)
  - 统一 Skills 加载路径，使用全局 `skills/` 目录
  - 明确 `src/agent/` (业务逻辑) 和 `src-tauri/src/commands/` (Tauri 命令层) 职责

- **代码质量改进**
  - 修复多个编译警告和临时值生命周期问题
  - 统一导入路径和模块导出

### Fixed

- 修复 `agent_commands.rs` 重复定义问题
- 修复工具执行器中的临时值生命周期错误
- 修复模块导入路径问题

## [0.1.1] - 2026-02-13

### Fixed

- **Iroh peer connection closure**
  - Implemented real `CommsHandle::connect` logic (no longer a TODO placeholder).
  - Added support for peer descriptors in format `<endpoint_id>@<ip:port>,<relay_url>` and JSON descriptors.
  - Connected `workers` AI connection acceptance flow to actual iroh connection execution.

- **Message path reliability**
  - Added background incoming-connection reader for iroh transport.
  - Routed received wrapped gossip messages into `QuicGateway` receive queue for downstream processing.
  - Reduced lock contention in send/broadcast paths by avoiding async awaits while holding connection-map locks.

- **Node identity stability**
  - Reworked desktop device ID generation to be persistent and cross-platform.
  - Prevented non-Windows random ID churn across requests, improving Workers-side node continuity.

- **Runtime metadata accuracy**
  - Platform metadata now uses runtime OS detection.
  - App version metadata now reads from package version.

### Changed

- Project version bumped to `0.1.1` across:
  - `Cargo.toml`
  - `src-tauri/Cargo.toml`
  - `package.json`

## [0.1.0] - 2026-02-12

### Added

#### Core Features
- **P2P Node Implementation** - Full iroh-based peer-to-peer networking
  - QUIC transport protocol with NAT traversal
  - Node discovery and connection management
  - Gossip protocol for pub/sub messaging

- **Model Inference Engine** (`src/inference.rs`)
  - Tensor snapshot and sparse update support
  - Top-K sparse updates with residual feedback
  - Model hash and dimension output
  - Memory pressure detection with auto-adjusting Top-K

- **Topology Management** (`src/topology.rs`)
  - Geo + embedding dual-metric scoring
  - Primary neighbors and backup pool
  - Failover and unreachable marking
  - Device-capability-based neighbor count adjustment

- **Device Adaptation** (`src/device.rs`)
  - Device capability detection (memory, CPU, network, battery)
  - Adaptive configuration based on device capabilities
  - Battery-aware scheduling
  - Network type detection (WiFi/4G/5G)

#### Web3 Integration
- **Dual Signature Support**
  - Ethereum (k256) signatures
  - Solana (ed25519) signatures
- **Stake/Reputation System**
- **Heartbeat, sparse, and dense message signing**

#### Privacy Protection (`src/privacy/`)
- Traffic obfuscation with random padding
- IP hiding via relay network
- Identity protection with periodic NodeId rotation
- Privacy-performance balance engine

#### Desktop Application
- **Tauri Desktop App** (v0.1.1)
  - Modern Web UI with P2P management
  - Real-time connection status monitoring
  - Node ID display and management

#### Additional Features
- Model persistence with checkpoint saving/loading
- PyTorch model conversion tools
- Multi-node testing scripts
- Android and iOS FFI integration
- P2P model distribution with sharding

### Known Issues

- Web3 staking system requires mainnet deployment
- Privacy protection advanced features need further testing
- Mobile integration verification pending

### Recommended for MVP (v0.1.0)

Include:
- P2P node startup and connection
- Model inference (128-dim)
- Topology neighbor management
- Tauri desktop application

Postpone to v0.2.0:
- Web3 staking system
- Advanced privacy features
- Mobile integration verification

---

## [Unreleased]

### Planned for v0.2.0
- Enhanced Web3 integration with mainnet
- Advanced privacy protection features
- Mobile platform verification (Android/iOS)
- Performance optimizations

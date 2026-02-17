# williw 架构总结

> **三层架构** - P2P 通信 + Workers 分布式推理

---

## 核心架构

### 三层设计

```
┌─────────────────────────────────────────────────────────────────┐
│                    williw 完整架构                               │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────┐
│  用户接口层          │
│  (Tauri App)        │  williw-master/src-tauri
│                     │  - 用户请求
│                     │  - 设备信息
│                     │  - AI 决策
└──────────┬──────────┘
           │ HTTP
           ↓
┌─────────────────────────────────────────────────────────────────┐
│  边缘服务器层 (Edge Server)                                      │
│  williw-workers/edge_server                                     │
│                                                                 │
│  - 模型获取与转换 (Hugging Face/本地)                            │
│  - 算力估算 (保守策略，安全系数 1.5)                              │
│  - 调用算法层选择 Workers                                        │
│  - 分配任务给 Workers                                            │
│  - 回收结果                                                     │
│                                                                 │
│  运行方式：宿主机直接运行（需要 GPU 访问）                         │
└────────────┬────────────────────────────────────────────────────┘
             │
             ├────→ Worker A (层 1-4) → 退出
             ├────→ Worker B (层 5-8) → 退出
             └────→ Worker C (层 9-12) → 退出

┌─────────────────────┐
│  P2P 通信层          │
│  (Rust + iroh)      │  williw-master/src
│                     │  - 节点发现
│                     │  - 拓扑管理
│                     │  - 共识机制
└─────────────────────┘
    Docker 容器运行
```

---

## 组件说明

### 1. 用户接口层 (Tauri App)

**位置:** `williw-master/src-tauri/`

**功能:**
- 用户界面
- 设备能力检测（GPU、CPU、内存、网络）
- AI 决策引擎
- 上传节点信息到边缘服务器

**运行方式:**
```bash
# 开发模式
cd src-tauri
npm run tauri dev

# 生产模式
npm run tauri build
```

---

### 2. 边缘服务器层 (Edge Server)

**位置:** `williw-workers/edge_server/`

**功能:**
- 接收推理请求 (`POST /api/inference`)
- 模型获取与转换
- 算力估算（保守策略）
- 调用算法层选择 Workers
- 分配任务给 Workers
- 回收结果并返回

**运行方式:**
```bash
# 宿主机直接运行
cd williw-workers
python -m edge_server.api_server --port 8080

# 或使用启动脚本
./start.sh --workers
```

**API 端点:**
- `POST /api/inference` - 推理请求
- `GET /api/health` - 健康检查
- `GET /api/models` - 可用模型列表

---

### 3. P2P 通信层 (Rust + iroh)

**位置:** `williw-master/src/`

**功能:**
- P2P 节点发现（iroh QUIC）
- 拓扑管理（Geo + Embedding 双指标）
- 共识机制（Web3 签名/质押/信誉）
- 设备能力检测

**运行方式:**
```bash
# Docker 容器
docker run --name williw-node williw-node

# 或直接运行
cargo run --bin williw-bin
```

---

## Workers 机制

### 任务驱动的动态算力池

**❌ 不是广播发现节点**
```
之前理解：UDP 广播 → 发现节点 → 持续在线
```

**✅ 而是任务驱动的 Workers 池**
```
1. Worker 启动 → 注册到边缘服务器
2. 边缘服务器分配任务 → Worker 执行
3. 任务完成 → Worker 返回结果并退出
4. 边缘服务器获取结果 → 传递给下一个 Worker
```

### Workers 生命周期

```
┌─────────────┐
│ Worker 启动  │
│ - 初始化     │
│ - 注册能力   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 等待任务     │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 接收任务     │
│ - 模型分片   │
│ - 输入数据   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 执行推理     │
│ - 前向传播   │
│ - 输出激活值 │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 返回结果     │
│ - 激活值     │
│ - 状态信息   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ Worker 退出  │
│ - 释放资源   │
└─────────────┘
```

---

## 完整工作流程

### 推理请求流程

```
1. 用户在 Tauri App 发起推理请求
   ↓
2. Tauri App 发送 HTTP POST 到边缘服务器
   POST http://localhost:8080/api/inference
   ↓
3. 边缘服务器分析模型
   - 从 Hugging Face/本地获取模型
   - ONNX → PyTorch 转换
   - 读取 state_dict
   - 估算算力需求（安全系数 1.5）
   ↓
4. 调用算法层（lkc）
   - 节点选择算法
   - D-CACO 路径优化
   - 资源分配算法
   - 模型切分器
   ↓
5. 分配任务给 Workers
   - Worker A: 层 1-4
   - Worker B: 层 5-8
   - Worker C: 层 9-12
   ↓
6. 分布式推理
   Worker A → Worker B → Worker C
   (激活值传递)
   ↓
7. 结果回收
   边缘服务器收集最终结果
   ↓
8. Workers 退出
   资源释放
   ↓
9. 返回结果给 Tauri App
   ↓
10. 显示结果给用户
```

---

## 部署方式

### 一键启动

```bash
# 启动所有服务
./start.sh --all

# Windows (PowerShell)
.\start.ps1 -All
```

### 单独启动

```bash
# 只启动 Rust 节点（Docker）
./start.sh --node

# 只启动 GPU 推理服务
./start.sh --gpu

# 只启动 Workers 边缘服务器
./start.sh --workers
```

### 停止服务

```bash
# 停止所有
./start.sh --stop

# 清理容器
./start.sh --clean
```

---

## 文件结构

```
williw-master/
├── src/                        # Rust P2P 节点
│   ├── main.rs
│   ├── comms/                 # 通信层 (iroh)
│   ├── inference.rs           # 推理引擎
│   ├── topology.rs            # 拓扑管理
│   └── device.rs              # 设备检测
├── src-tauri/                  # Tauri App
│   ├── src/
│   ├── src/commands/
│   │   ├── workers_commands.rs  # Workers API 调用
│   │   └── training_commands.rs
│   └── src/ai_decision.rs      # AI 决策
├── williw-workers/             # Workers 边缘服务器
│   ├── edge_server/
│   │   ├── api_server.py       # Flask API
│   │   ├── model_fetcher.py    # 模型获取
│   │   ├── model_converter.py  # 模型转换
│   │   ├── compute_estimator.py # 算力估算
│   │   └── workflow_orchestrator.py
│   ├── interface_layer/
│   ├── models/
│   └── algorithms/             # 算法层 (lkc)
├── Dockerfile                  # Rust 节点容器
├── docker-compose.yml          # 容器编排
├── start.sh                    # 启动脚本
├── start.ps1                   # Windows 启动脚本
└── docs/
    ├── WORKERS_DEPLOYMENT.md   # Workers 部署指南
    ├── WORKERS_MECHANISM.md    # Workers 机制详解
    └── WORKERS_SUMMARY.md      # Workers 总结
```

---

## 容器与宿主机通信

### 网络配置

```yaml
# docker-compose.yml
services:
  williw-node:
    network_mode: host
    extra_hosts:
      - "host.docker.internal:host-gateway"
    environment:
      - WILLIW_WORKERS_EDGE_SERVER_URL=http://host.docker.internal:8080
```

### 通信流程

```
┌─────────────────────┐
│  Rust 节点 (容器)     │
│  :9235              │
└──────────┬──────────┘
           │ host.docker.internal
           ↓
┌─────────────────────┐
│  宿主机             │
│  Workers: 8080      │
│  GPU: 8000          │
└─────────────────────┘
```

---

## 参考文档

| 文档 | 说明 |
|------|------|
| [docs/WORKERS_DEPLOYMENT.md](docs/WORKERS_DEPLOYMENT.md) | Workers 部署指南 |
| [docs/WORKERS_MECHANISM.md](docs/WORKERS_MECHANISM.md) | Workers 机制详解 |
| [WORKERS_SUMMARY.md](WORKERS_SUMMARY.md) | Workers 架构总结 |
| [QUICKSTART.md](QUICKSTART.md) | 5 分钟快速上手 |

---

*最后更新：2024-02-17*

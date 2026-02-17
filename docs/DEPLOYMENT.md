# williw 跨平台部署指南

> **版本**: v0.2.0 | **架构**: 混合部署（容器 + 宿主机）

## 架构说明

### 设计理念

基于《人月神话》原则：
- **渐进增强**: 先确保核心功能稳定，再扩展 GPU 支持
- **概念完整性**: 统一的跨平台部署流程
- **计划废止**: 易于升级和替换组件

### 混合架构

```
┌─────────────────────────────────────────────────────────┐
│                    williw 混合架构                       │
└─────────────────────────────────────────────────────────┘

┌──────────────────────┐         ┌──────────────────────┐
│   Docker 容器         │         │     宿主机           │
│  (Rust 节点)          │         │  (GPU 推理服务)       │
│                      │         │                      │
│  ✅ P2P 通信 (iroh)   │ ←────→  │  ✅ Python Flask     │
│  ✅ AI 决策模块       │  HTTP   │  ✅ PyTorch 推理     │
│  ✅ 拓扑管理         │ :8000   │  ✅ CUDA/MPS 加速    │
│  ✅ 共识与 Web3      │         │                      │
└──────────────────────┘         └──────────────────────┘
         │                               │
         └─────────── 都运行在你的        │
                     电脑上              │
                                         ▼
                                 ┌──────────────────────┐
                                 │   硬件访问            │
                                 │  - NVIDIA GPU (CUDA)  │
                                 │  - Apple GPU (MPS)    │
                                 │  - 电池状态           │
                                 │  - 网络类型           │
                                 └──────────────────────┘
```

### 为什么采用混合架构？

| 方案 | 优点 | 缺点 |
|------|------|------|
| **纯容器方案** | 隔离性好 | ❌ GPU 穿透配置复杂<br>❌ Mac 不支持 CUDA<br>❌ 硬件访问受限 |
| **混合方案** ✅ | ✅ 跨平台一致<br>✅ 直接访问硬件<br>✅ 无需 GPU 穿透<br>✅ 易于调试 | 需要运行两个进程 |

---

## 快速开始

### 1. 前置条件

#### 所有平台
```bash
# 安装 Rust (如果还没有)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Docker
# Mac: https://docs.docker.com/desktop/install/mac-install/
# Windows: https://docs.docker.com/desktop/install/windows-install/
# Linux: https://docs.docker.com/desktop/install/linux-install/
```

#### GPU 推理服务（可选，但推荐）
```bash
# 安装 Python 3.8+
# 安装依赖
pip install -r requirements-gpu.txt
```

### 2. 一键启动

#### Mac / Linux
```bash
# 启动所有服务（Rust 节点 + GPU 推理）
./start.sh --all

# 只启动 Rust 节点
./start.sh --node

# 只启动 GPU 推理服务
./start.sh --gpu

# 查看状态
./start.sh --status

# 停止所有服务
./start.sh --stop

# 清理容器
./start.sh --clean
```

#### Windows (PowerShell)
```powershell
# 启动所有服务
.\start.ps1 -All

# 只启动 Rust 节点
.\start.ps1 -Node

# 只启动 GPU 推理服务
.\start.ps1 -Gpu

# 查看状态
.\start.ps1 -Status

# 停止所有服务
.\start.ps1 -Stop

# 清理容器
.\start.ps1 -Clean -Force
```

### 3. 验证部署

```bash
# 检查 Docker 容器
docker ps | grep williw

# 查看节点日志
docker logs -f williw-node

# 检查 GPU 推理服务（如果启动）
curl http://localhost:8000/
```

---

## 跨平台部署场景

### 场景 1: Mac (M1/M2/M3) - AI 决策节点

```bash
# Mac 作为 AI 决策节点（无 GPU 推理）
./start.sh --node

# 配置 AI 决策模块
export WILLIW_AI_API_KEY="your-api-key"
export WILLIW_AI_BASE_URL="https://api.openai.com/v1"
export WILLIW_AI_MODEL="gpt-4"

# 运行 AI 决策示例
cargo run --example ai_decision_demo
```

**日志输出示例:**
```
[INFO] 节点启动成功
[INFO] Node ID: williw-node-abc123
[INFO] 监听端口：9235/udp
[INFO] GPU 推理服务：未启用（CPU 模式）
```

### 场景 2: Windows (NVIDIA GPU) - GPU 推理节点

```powershell
# Windows 作为 GPU 推理节点
.\start.ps1 -All

# 验证 GPU 穿透
docker exec williw-node nvidia-smi  # 应该能看到 GPU 信息

# 测试 GPU 推理
curl http://localhost:8000/infer `
  -H "Content-Type: application/json" `
  -d '{"input_text": "Hello, how are you?"}'
```

**日志输出示例:**
```
[INFO] 检测到 cuda GPU
[INFO] GPU 推理服务已启动 (PID: 12345)
[INFO] Rust 节点已启动
[SUCCESS] 所有服务已启动
```

### 场景 3: Mac + Windows P2P 互联

```
┌─────────────────────┐              ┌─────────────────────┐
│   Mac (M2)          │              │   Windows (RTX 3080)│
│   AI 决策节点        │              │   GPU 推理节点      │
│                     │              │                     │
│   IP: 192.168.1.100 │              │   IP: 192.168.1.101 │
│   Port: 9235        │              │   Port: 9235        │
│                     │              │                     │
│   ./start.sh --node │              │   .\start.ps1 -All  │
└──────────┬──────────┘              └──────────┬──────────┘
           │                                    │
           └──────────── iroh P2P ──────────────┘
                       QUIC 通信
```

#### Mac 端配置
```bash
# 添加 Windows 节点为 bootstrap
export WILLIW_BOOTSTRAP=192.168.1.101:9235
./start.sh --node
```

#### Windows 端配置
```powershell
# 正常启动
.\start.ps1 -All
```

---

## 配置说明

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `WILLIW_NODE_ID` | 节点唯一标识 | 自动生成 |
| `WILLIW_DEVICE_TYPE` | 设备类型 (low/mid/high) | high |
| `WILLIW_MODEL_DIM` | 模型维度 | 128 |
| `WILLIW_GPU_INFERENCE_URL` | GPU 推理服务地址 | http://host.docker.internal:8000 |
| `WILLIW_AI_API_KEY` | AI API 密钥 | - |
| `WILLIW_AI_BASE_URL` | AI API 地址 | https://api.openai.com/v1 |
| `WILLIW_AI_MODEL` | AI 模型 | gpt-4 |
| `RUST_LOG` | 日志级别 | info |

### .env 文件示例

创建 `.env` 文件（推荐方式）:
```bash
# 节点配置
WILLIW_NODE_ID=my-mac-node
WILLIW_DEVICE_TYPE=high
WILLIW_MODEL_DIM=256

# AI 决策配置
WILLIW_AI_API_KEY=sk-xxxxxxxxxxxxxxxx
WILLIW_AI_BASE_URL=https://api.openai.com/v1
WILLIW_AI_MODEL=gpt-4

# GPU 推理配置
WILLIW_GPU_INFERENCE_URL=http://host.docker.internal:8000
```

---

## 故障排查

### 问题 1: Docker 容器无法启动

```bash
# 检查 Docker 是否运行
docker info

# 查看容器日志
docker logs williw-node

# 重新构建镜像
docker build -t williw-node .
```

### 问题 2: GPU 推理服务无法启动

```bash
# 检查 Python 依赖
pip list | grep -E "flask|torch|transformers"

# 重新安装依赖
pip install -r requirements-gpu.txt --force-reinstall

# 查看日志
cat gpu_service.log
```

### 问题 3: Mac 无法连接 Windows 节点

```bash
# 检查防火墙
# Mac: 系统偏好设置 > 安全性与隐私 > 防火墙
# Windows: Windows Defender 防火墙

# 测试端口连通性
nc -zv 192.168.1.101 9235

# 检查节点日志
docker logs williw-node | grep -i "peer\|connect"
```

### 问题 4: GPU 推理超时

```bash
# 检查 GPU 服务是否响应
curl http://localhost:8000/status

# 检查显存占用
nvidia-smi  # Windows/Linux
```

---

## 性能优化建议

### Mac (Apple Silicon)

```bash
# 使用 MPS 加速（如果 PyTorch 支持）
export PYTORCH_ENABLE_MPS_FALLBACK=1

# 调整模型维度以适应内存
export WILLIW_MODEL_DIM=128
```

### Windows (NVIDIA GPU)

```powershell
# 启用 GPU 推理
.\start.ps1 -All

# 监控 GPU 使用
nvidia-smi dmon
```

### 多节点部署

```bash
# 节点 1（Bootstrap）
export WILLIW_NODE_ID=node1
./start.sh --node

# 节点 2（连接节点 1）
export WILLIW_NODE_ID=node2
export WILLIW_BOOTSTRAP=192.168.1.100:9235
./start.sh --node

# 节点 3
export WILLIW_NODE_ID=node3
export WILLIW_BOOTSTRAP=192.168.1.100:9235
./start.sh --node
```

---

## 架构演进

### v0.1.x - 纯容器方案
- ❌ GPU 穿透配置复杂
- ❌ Mac 不支持
- ❌ 硬件访问受限

### v0.2.x - 混合架构 ✅ 当前版本
- ✅ 跨平台一致
- ✅ 直接硬件访问
- ✅ 易于调试

### v0.3.x - 规划中
- 📋 服务网格集成
- 📋 自动故障转移
- 📋 负载均衡

---

## 参考文档

- [iroh 文档](https://docs.iroh.computer/)
- [Docker Desktop](https://docs.docker.com/desktop/)
- [PyTorch GPU 加速](https://pytorch.org/get-started/locally/)
- [《人月神话》](https://zh.wikipedia.org/wiki/%E4%BA%BA%E6%9C%88%E7%A5%9E%E8%AF%9D)

---

## 支持

遇到问题？
1. 查看日志：`docker logs williw-node`
2. 检查状态：`./start.sh --status`
3. 查看 GitHub Issues
4. 联系维护者

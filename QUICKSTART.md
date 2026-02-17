# williw 快速开始指南

> **5 分钟上手** - 跨平台去中心化训练节点

---

## 一分钟了解新架构

### 之前（纯容器方案）❌
```
┌─────────────────┐
│  Docker 容器     │
│  - Rust 节点    │
│  - Python GPU   │ ← 需要 GPU 穿透配置
│  - CUDA         │ ← Mac 不支持
└─────────────────┘
```

### 现在（混合架构）✅
```
┌─────────────────┐         ┌─────────────────┐
│  Docker 容器     │         │   宿主机         │
│  - Rust 节点    │ ←HTTP→  │  - Python GPU   │
│  - AI 决策      │ :8000   │  - 直接访问硬件  │
└─────────────────┘         └─────────────────┘
```

**优势：**
- ✅ Mac/Windows/Linux 统一
- ✅ 无需 GPU 穿透配置
- ✅ 直接访问电池、网络等硬件

---

## 快速开始（3 步）

### Step 1: 安装依赖

```bash
# 1. 安装 Docker Desktop
# https://www.docker.com/products/docker-desktop/

# 2. 安装 Python 依赖（用于 GPU 推理）
pip install -r requirements-gpu.txt

# 3. 克隆项目
git clone <your-repo> williw-master
cd williw-master
```

### Step 2: 一键启动

#### Mac / Linux
```bash
./start.sh --all
```

#### Windows (PowerShell)
```powershell
.\start.ps1 -All
```

### Step 3: 验证

```bash
# 查看状态
./start.sh --status

# 或 Windows
.\start.ps1 -Status
```

**预期输出：**
```
========================================
  williw 系统状态
========================================
操作系统：macos
GPU 模式：mps

Docker 容器:
NAMES           STATUS          PORTS
williw-node     Up 10 seconds   9235/udp

GPU 推理服务:
✅ 运行中 (PID: 12345)

========================================
```

---

## 常用命令

### 启动服务

```bash
# 启动所有（Rust 节点 + GPU 推理）
./start.sh --all

# 只启动 Rust 节点
./start.sh --node

# 只启动 GPU 推理
./start.sh --gpu
```

### 停止服务

```bash
# 停止所有
./start.sh --stop

# 清理容器
./start.sh --clean
```

### 查看日志

```bash
# Rust 节点日志
docker logs -f williw-node

# GPU 推理日志
tail -f gpu_service.log
```

---

## 跨平台测试（Mac ↔ Windows）

### 场景：Mac 作为 AI 决策节点，Windows 作为 GPU 推理节点

```
┌─────────────────────┐              ┌─────────────────────┐
│   Mac (M2)          │              │   Windows (RTX 3080)│
│   AI 决策节点        │              │   GPU 推理节点      │
│   IP: 192.168.1.100 │              │   IP: 192.168.1.101 │
└──────────┬──────────┘              └──────────┬──────────┘
           │                                    │
           └──────────── iroh P2P ──────────────┘
```

### 在 Windows 上（主机）

```powershell
# 1. 启动节点
.\start.ps1 -All

# 2. 获取本机 IP
ipconfig
# 记录 IPv4 地址，例如：192.168.1.101
```

### 在 Mac 上（客户端）

```bash
# 1. 连接到 Windows 节点
export WILLIW_BOOTSTRAP=192.168.1.101:9235
./start.sh --node

# 2. 查看连接日志
docker logs -f williw-node | grep -i "peer\|connect"
```

### 自动化测试

```bash
# Windows 上运行
.\scripts\test_cross_machine.ps1 -Host

# Mac 上运行
./scripts/test_cross_machine.sh --client --target 192.168.1.101
```

---

## AI 决策模块配置

### 设置 AI API

```bash
# 创建 .env 文件
cat > .env << EOF
WILLIW_AI_API_KEY=sk-your-api-key
WILLIW_AI_BASE_URL=https://api.openai.com/v1
WILLIW_AI_MODEL=gpt-4
EOF

# 启动服务
./start.sh --all
```

### 测试 AI 决策

```bash
# 运行 AI 决策示例
cargo run --example ai_decision_demo
```

---

## 故障排查

### 问题 1: 容器启动失败

```bash
# 检查 Docker
docker info

# 重新构建
docker build -t williw-node .
```

### 问题 2: GPU 服务无法启动

```bash
# 检查依赖
pip list | grep -E "flask|torch"

# 重新安装
pip install -r requirements-gpu.txt --force-reinstall
```

### 问题 3: 跨机器连接失败

```bash
# 检查防火墙
# Mac: 系统偏好设置 > 安全性 > 防火墙
# Windows: Windows Defender 防火墙

# 测试端口
nc -zv <目标 IP> 9235
```

---

## 下一步

- 📖 [完整部署指南](docs/DEPLOYMENT.md)
- 🔧 [AI 决策模块文档](src-tauri/src/ai_decision.rs)
- 🌐 [iroh P2P 文档](https://docs.iroh.computer/)

---

## 需要帮助？

```bash
# 显示帮助
./start.sh --help

# 查看状态
./start.sh --status

# 查看日志
docker logs williw-node
```

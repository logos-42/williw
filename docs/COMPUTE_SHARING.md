# williw 算力互通指南

> **Mac MPS + Windows CUDA 算力共享** - 去中心化算力网络

---

## 架构说明

### 算力互通架构

```
┌──────────────────────────────────────────────────────────────────┐
│                        你的算力网络                               │
└──────────────────────────────────────────────────────────────────┘

┌─────────────────────┐              ┌─────────────────────┐
│   Mac (M2/M3)       │              │   Windows (RTX 3080)│
│   Apple Silicon     │              │   NVIDIA GPU        │
│                     │              │                     │
│  ┌───────────────┐  │              │  ┌───────────────┐  │
│  │ Rust 节点      │  │              │  │ Rust 节点      │  │
│  │ (Docker)      │  │              │  │ (Docker)      │  │
│  └───────┬───────┘  │              │  └───────┬───────┘  │
│          │          │              │          │          │
│  ┌───────▼───────┐  │    P2P       │  ┌───────▼───────┐  │
│  │ 算力注册服务   │◄─┼────通信────►│  │ 算力注册服务   │  │
│  │ (Python)      │  │    :9236     │  │ (Python)      │  │
│  └───────┬───────┘  │              │  └───────┬───────┘  │
│          │          │              │          │          │
│  ┌───────▼───────┐  │              │  ┌───────▼───────┐  │
│  │ GPU 推理服务   │  │              │  │ GPU 推理服务   │  │
│  │ (MPS 加速)     │  │              │  │ (CUDA 加速)    │  │
│  └───────────────┘  │              │  └───────────────┘  │
└─────────────────────┘              └─────────────────────┘
         │                                    │
         └──────────── 算力互通 ──────────────┘
                     Mac MPS ↔ Windows CUDA
```

### 核心组件

| 组件 | 端口 | 功能 |
|------|------|------|
| **Rust 节点** | 9235 | P2P 通信、AI 决策、拓扑管理 |
| **算力注册服务** | 9236 | 算力发现、注册、调度 |
| **GPU 推理服务** | 8000 | PyTorch 推理（MPS/CUDA） |

### 算力发现机制

```
1. 广播发现
   ┌─────────┐
   │ 节点 A   │ ──UDP 广播──► 网络中的所有设备
   └─────────┘

2. 自动注册
   ┌─────────┐      ┌─────────┐
   │ 节点 A   │ ◄──► │ 节点 B   │  交换算力信息
   └─────────┘      └─────────┘

3. 算力调度
   ┌─────────┐      ┌─────────┐
   │ 节点 A   │ ──HTTP──► │ 节点 B   │  发送推理任务
   └─────────┘      └─────────┘
```

---

## 快速开始

### Step 1: 安装依赖

```bash
# 所有平台
pip install -r requirements-gpu.txt

# Docker Desktop
# https://www.docker.com/products/docker-desktop/
```

### Step 2: 启动算力节点

```bash
# Mac / Linux
./start.sh --all

# Windows (PowerShell)
.\start.ps1 -All
```

### Step 3: 验证算力网络

```bash
# 查看状态
./start.sh --status

# 运行演示
python3 demo_compute_sharing.py
```

**预期输出:**
```
============================================================
  williw 算力互通演示
============================================================

[步骤 1] 检查算力注册服务
✅ 算力注册服务运行正常
   节点 ID: abc12345
   设备类型：mps

[步骤 3] 获取算力网络中的节点
✅ 发现 2 个算力节点:
   📍 节点 1: abc12345
      设备：mps (Apple Silicon)
      算力：8 单位，16 GB 内存
   📡 节点 2: def67890
      设备：cuda (NVIDIA GeForce RTX 3080)
      算力：68 单位，24 GB 内存
```

---

## 跨平台算力互通

### 场景 1: Mac + Windows 协同

```
Mac (M2)                          Windows (RTX 3080)
192.168.1.100                     192.168.1.101
┌─────────────────┐               ┌─────────────────┐
│ AI 决策任务      │               │ GPU 推理任务     │
│ MPS 可用算力：8   │               │ CUDA 可用算力：68│
└─────────────────┘               └─────────────────┘
```

#### 在 Windows 上

```powershell
# 1. 启动所有服务
.\start.ps1 -All

# 2. 查看本机 IP
ipconfig
# 记录 IPv4 地址，例如：192.168.1.101
```

#### 在 Mac 上

```bash
# 1. 启动所有服务
./start.sh --all

# 2. 查看算力网络
./start.sh --status

# 3. 运行演示
python3 demo_compute_sharing.py
```

### 场景 2: 多台设备算力池

```
┌─────────────┐
│ Mac (M2)    │  8 算力单位
└──────┬──────┘
       │
       ├─── 算力网络 ───┐
       │                │
┌──────▼──────┐   ┌─────▼──────┐
│ Windows     │   │ Linux      │
│ RTX 3080    │   │ RTX 4090   │
│ 68 算力单位  │   │ 128 算力单位│
└─────────────┘   └────────────┘

总算力：204 单位
```

---

## API 参考

### 算力注册服务 (端口 9236)

#### 获取算力列表

```bash
curl http://localhost:9236/compute
```

**响应:**
```json
{
  "status": "success",
  "total_nodes": 2,
  "compute_nodes": [
    {
      "node_id": "abc12345",
      "device_type": "mps",
      "gpu_info": {"name": "Apple Silicon"},
      "compute_units": 8,
      "memory_gb": 16,
      "available": true
    },
    {
      "node_id": "def67890",
      "device_type": "cuda",
      "gpu_info": {"name": "NVIDIA RTX 3080"},
      "compute_units": 68,
      "memory_gb": 24,
      "available": true,
      "is_peer": true
    }
  ]
}
```

#### 注册算力

```bash
curl -X POST http://localhost:9236/register \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "my-node",
    "device_type": "cuda",
    "compute_units": 68,
    "memory_gb": 24
  }'
```

#### 调度任务

```bash
curl -X POST http://localhost:9236/dispatch \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "task-001",
    "task_type": "model_inference",
    "target_node": "def67890"
  }'
```

### GPU 推理服务 (端口 8000)

#### 执行推理

```bash
curl -X POST http://localhost:8000/infer \
  -H "Content-Type: application/json" \
  -d '{
    "input_text": "Hello, world!",
    "max_length": 100
  }'
```

---

## 算力调度策略

### 自动选择

```python
# 不指定目标节点，自动选择算力最强的
curl -X POST http://localhost:9236/dispatch \
  -d '{"task_type": "inference"}'

# 响应：自动选择算力最强的节点
{
  "target_node": "def67890",  # RTX 3080
  "task_id": "xxx"
}
```

### 指定节点

```python
# 指定使用 Mac 的 MPS
curl -X POST http://localhost:9236/dispatch \
  -d '{"task_type": "inference", "target_node": "abc12345"}'

# 指定使用 Windows 的 CUDA
curl -X POST http://localhost:9236/dispatch \
  -d '{"task_type": "training", "target_node": "def67890"}'
```

---

## 故障排查

### 问题 1: 无法发现其他节点

```bash
# 检查防火墙
# Mac: 系统偏好设置 > 安全性 > 防火墙
# Windows: Windows Defender 防火墙

# 确保端口开放
# - 9235/tcp, 9235/udp (P2P)
# - 9236/tcp, 9236/udp (算力注册)

# 测试广播
python3 -c "
import socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.sendto(b'test', ('255.255.255.255', 9236))
print('广播已发送')
"
```

### 问题 2: Mac MPS 无法使用

```bash
# 检查 PyTorch MPS 支持
python3 -c "
import torch
print('MPS available:', torch.backends.mps.is_available())
print('PyTorch version:', torch.__version__)
"

# 确保使用 PyTorch 2.0+
pip install --upgrade torch torchvision
```

### 问题 3: Windows CUDA 无法使用

```powershell
# 检查 NVIDIA 驱动
nvidia-smi

# 检查 PyTorch CUDA 支持
python3 -c "
import torch
print('CUDA available:', torch.cuda.is_available())
if torch.cuda.is_available():
    print('GPU:', torch.cuda.get_device_name(0))
}
"

# 重新安装 CUDA 版 PyTorch
pip uninstall torch torchvision
pip install torch torchvision --index-url https://download.pytorch.org/whl/cu118
```

---

## 性能优化

### Mac (Apple Silicon)

```bash
# 启用 MPS 优化
export PYTORCH_ENABLE_MPS_FALLBACK=1

# 设置合理内存限制
export PYTORCH_MPS_HIGH_WATERMARK_RATIO=0.8
```

### Windows (NVIDIA GPU)

```powershell
# 启用 Tensor Core
# 在推理代码中使用
# model.half()  # FP16

# 监控 GPU 使用
nvidia-smi dmon
```

### 算力分配建议

| 任务类型 | 推荐设备 | 原因 |
|---------|---------|------|
| 大模型推理 | Windows (CUDA) | Tensor Core 加速 |
| 小模型推理 | Mac (MPS) | 能效比高 |
| 批量处理 | 多设备并行 | 算力池化 |
| 实时推理 | 就近选择 | 低延迟 |

---

## 下一步

### 监控算力网络

```bash
# 实时查看算力状态
watch -n 2 'curl -s http://localhost:9236/status | python3 -m json.tool'
```

### 添加更多节点

```bash
# 在任何设备上
./start.sh --all

# 自动加入算力网络
# 通过 UDP 广播自动发现
```

### 自定义调度策略

修改 `compute_registry.py` 中的调度逻辑：
- 基于地理位置
- 基于算力类型
- 基于成本优化

---

## 参考文档

- [部署指南](docs/DEPLOYMENT.md)
- [快速开始](QUICKSTART.md)
- [PyTorch MPS](https://pytorch.org/docs/stable/notes/mps.html)
- [NVIDIA CUDA](https://developer.nvidia.com/cuda-toolkit)

---

*最后更新：2024-02-17*

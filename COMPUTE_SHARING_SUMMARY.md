# williw 算力互通架构

> **Mac MPS + Windows CUDA + Linux CUDA 算力共享** - 去中心化算力网络

---

## 核心变更

### 新增组件

| 文件 | 说明 |
|------|------|
| `compute_registry.py` | 算力注册与发现服务（核心） |
| `demo_compute_sharing.py` | 算力互通演示脚本 |
| `docs/COMPUTE_SHARING.md` | 算力互通指南 |
| `start.sh` (更新) | 支持启动算力注册服务 |
| `start.ps1` (更新) | 支持启动算力注册服务 |
| `Dockerfile` (更新) | 支持算力发现配置 |

---

## 架构说明

### 三组件架构

```
┌─────────────────────────────────────────────────────────┐
│                    你的电脑                              │
│                                                         │
│  ┌─────────────────────┐                               │
│  │  Docker 容器         │                               │
│  │  (williw-node)      │                               │
│  │                     │                               │
│  │  ✅ Rust 节点        │                               │
│  │  ✅ P2P 通信 (iroh)  │                               │
│  │  ✅ AI 决策模块      │                               │
│  │  ✅ 算力调度客户端   │                               │
│  └─────────┬───────────┘                               │
│            │                                            │
│            │ HTTP :9236                                 │
│            ▼                                            │
│  ┌─────────────────────┐         ┌─────────────────┐   │
│  │ 算力注册服务        │         │  GPU 推理服务    │   │
│  │ (Python + Flask)    │         │  (Python)       │   │
│  │                     │         │                 │   │
│  │ ✅ 算力发现         │         │ ✅ MPS (Mac)    │   │
│  │ ✅ 算力注册         │         │ ✅ CUDA (Win)   │   │
│  │ ✅ 算力调度         │         │ ✅ CPU 回退      │   │
│  │ ✅ UDP 广播          │         │                 │   │
│  └─────────────────────┘         └─────────────────┘   │
│            │                                            │
│            └─────────── 硬件访问 ────────────────────   │
│                    - Apple Silicon GPU (MPS)            │
│                    - NVIDIA GPU (CUDA)                  │
│                    - 电池状态、网络类型                  │
└─────────────────────────────────────────────────────────┘
```

### 算力互通流程

```
1. 启动阶段
   ┌─────────┐    ┌─────────┐
   │ Mac 节点  │    │ Win 节点 │
   └────┬────┘    └────┬────┘
        │              │
   启动算力注册    启动算力注册
        │              │

2. 发现阶段
   ┌─────────┐
   │ Mac 节点  │ ──UDP 广播──► 网络
   │ 广播：    │
   │ - 节点 ID │
   │ - 算力类型│
   │ - MPS 8 单位│
   └─────────┘
              ┌─────────┐
              │ Win 节点  │ ◄── 收到广播
              │ 记录 Mac │
              │ - CUDA 68│
              └─────────┘

3. 调度阶段
   ┌─────────┐
   │ AI 决策   │
   │ 需要推理 │
   └────┬────┘
        │
        ▼
   ┌─────────────────┐
   │ 算力注册服务     │
   │ 选择最佳节点    │
   │ - Mac (MPS)     │
   │ - Win (CUDA)    │
   └────┬────────────┘
        │
        ├──► Mac (小模型)
        │
        └──► Win (大模型)
```

---

## 快速开始

### 一键启动

```bash
# Mac / Linux
./start.sh --all

# Windows (PowerShell)
.\start.ps1 -All
```

### 验证算力网络

```bash
# 查看状态
./start.sh --status

# 运行演示
python3 demo_compute_sharing.py
```

### 查看发现的节点

```bash
# 查看算力网络
curl http://localhost:9236/peers

# 查看算力列表
curl http://localhost:9236/compute
```

---

## 跨平台测试

### 场景：Mac (M2) + Windows (RTX 3080)

```
Mac (M2)                          Windows (RTX 3080)
192.168.1.100                     192.168.1.101
┌─────────────────┐               ┌─────────────────┐
│ 启动：           │               │ 启动：           │
│ ./start.sh      │               │ .\start.ps1     │
│ --all           │               │ -All            │
│                 │               │                 │
│ 算力：MPS 8 单位  │◄────P2P────►│ 算力：CUDA 68 单位│
│ 内存：16GB       │    :9236     │ 内存：32GB       │
└─────────────────┘               └─────────────────┘
```

#### 步骤

**Windows 上:**
```powershell
# 1. 启动
.\start.ps1 -All

# 2. 获取 IP
ipconfig
# 192.168.1.101
```

**Mac 上:**
```bash
# 1. 启动
./start.sh --all

# 2. 查看发现的节点
./start.sh --status

# 3. 运行演示
python3 demo_compute_sharing.py
```

**预期输出:**
```
============================================================
  williw 算力互通演示
============================================================

[步骤 3] 获取算力网络中的节点
✅ 发现 2 个算力节点:
   📍 节点 1: abc12345 (本机)
      设备：mps (Apple Silicon)
      算力：8 单位，16 GB 内存
   📡 节点 2: def67890 (对等节点)
      设备：cuda (NVIDIA GeForce RTX 3080)
      算力：68 单位，32 GB 内存

[步骤 4] 调度算力任务
✅ 任务调度成功:
   任务 ID: demo-task-001
   目标节点：def67890 (自动选择算力最强的)
```

---

## API 参考

### 算力注册服务 (端口 9236)

| 端点 | 方法 | 说明 |
|------|------|------|
| `/` | GET | 健康检查 |
| `/register` | POST | 注册算力 |
| `/compute` | GET | 获取算力列表 |
| `/dispatch` | POST | 调度任务 |
| `/peers` | GET | 获取发现的节点 |
| `/status` | GET | 详细状态 |

### 示例

```bash
# 注册算力
curl -X POST http://localhost:9236/register \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "my-node",
    "device_type": "cuda",
    "compute_units": 68,
    "memory_gb": 32
  }'

# 获取算力列表
curl http://localhost:9236/compute | python3 -m json.tool

# 调度任务
curl -X POST http://localhost:9236/dispatch \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "task-001",
    "task_type": "model_inference",
    "target_node": "def67890"
  }'
```

---

## 算力调度策略

### 自动选择（默认）

```python
# 不指定目标，自动选择算力最强的
curl -X POST http://localhost:9236/dispatch \
  -d '{"task_type": "inference"}'

# 响应
{
  "target_node": "def67890",  # 自动选择 RTX 3080
  "task_id": "xxx"
}
```

### 指定设备类型

```python
# 使用 Mac MPS（能效优先）
curl -X POST http://localhost:9236/dispatch \
  -d '{"task_type": "small_inference", "target_node": "mac-node"}'

# 使用 Windows CUDA (性能优先)
curl -X POST http://localhost:9236/dispatch \
  -d '{"task_type": "large_training", "target_node": "win-node"}'
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

# 测试连通性
nc -zv 192.168.1.101 9236
```

### 问题 2: Mac MPS 不可用

```bash
# 检查 PyTorch MPS
python3 -c "
import torch
print('MPS:', torch.backends.mps.is_available())
"

# 升级 PyTorch
pip install --upgrade torch torchvision
```

### 问题 3: Windows CUDA 不可用

```powershell
# 检查驱动
nvidia-smi

# 检查 PyTorch CUDA
python3 -c "
import torch
print('CUDA:', torch.cuda.is_available())
"

# 重装 CUDA PyTorch
pip uninstall torch
pip install torch --index-url https://download.pytorch.org/whl/cu118
```

---

## 性能对比

| 设备 | GPU | 算力单位 | 适用场景 |
|------|-----|---------|---------|
| Mac M2 | MPS | 8 | 小模型、能效优先 |
| Mac M3 | MPS | 12 | 中小模型 |
| Windows RTX 3080 | CUDA | 68 | 大模型推理 |
| Windows RTX 4090 | CUDA | 128 | 大模型训练 |
| Linux RTX 4090 | CUDA | 128 | 分布式训练 |

---

## 下一步

### 监控算力网络

```bash
# 实时监控
watch -n 2 'curl -s http://localhost:9236/status | python3 -m json.tool'
```

### 添加更多节点

```bash
# 在任何设备上
./start.sh --all

# 自动通过 UDP 广播发现
```

---

## 参考文档

- [算力互通指南](docs/COMPUTE_SHARING.md)
- [部署指南](docs/DEPLOYMENT.md)
- [快速开始](QUICKSTART.md)

---

*最后更新：2024-02-17*

# williw 算力共享指南

> **Mac MPS + Windows CUDA + Linux CUDA 算力共享** - 任务驱动的动态算力池

---

## 什么是算力共享

williw 的算力共享机制允许你：

### 核心功能

1. **单机算力共享**
   - 在单设备上使用本地 GPU（MPS/CUDA）进行推理
   - 自动选择最优 GPU 后端
   - CPU 回退支持

2. **局域网算力共享**
   - 在多台设备间共享算力（Mac + Windows + Linux）
   - 动态分配算力任务给最优设备
   - 任务驱动的临时 Workers 机制

3. **分布式推理**
   - 模型自动切分（按层）
   - 激活值在 Workers 间传递
   - 结果自动集成

---

## 架构概览

### 三层架构

```
┌─────────────────────────────────────────────────────────────────┐
│                    williw 算力共享架构                           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────┐
│  用户接口层          │
│  (Tauri App)        │  williw-master/src-tauri
│                     │  - 用户请求
│                     │  - 设备信息
└──────────┬──────────┘
           │ HTTP POST /api/inference
           ↓
┌─────────────────────────────────────────────────────────────────┐
│  边缘服务器 (williw-workers/edge_server)                         │
│  运行方式：宿主机直接运行（需要 GPU 访问）                          │
│                                                                 │
│  核心工作流:                                                    │
│  1. 模型获取 (model_fetcher.py)                                 │
│  2. 模型转换 (model_converter.py)                               │
│  3. 算力估算 (compute_estimator.py) - 保守策略                   │
│  4. 节点信息获取 (interface_layer/node_info_api.py)             │
│  5. 算法层调用 (algorithms/)                                     │
│     - 节点选择 (node_selection.py)                              │
│     - 路径优化 (path_optimizer.py)                              │
│     - 资源分配 (resource_allocator.py)                          │
│     - 模型切分 (model_splitter.py)                              │
│     - 任务调度 (task_scheduler.py)                              │
│  6. 分布式推理 (models/inference_engine.py)                     │
│  7. 结果集成 (models/result_merger.py)                          │
└────────────┬────────────────────────────────────────────────────┘
             │
             ├────→ Worker A (层 1-4) → 退出
             ├────→ Worker B (层 5-8) → 退出
             └────→ Worker C (层 9-12) → 退出

┌─────────────────────┐
│  P2P 通信层          │
│  (Rust + iroh)      │  williw-master/src
│                     │  - 节点发现 (iroh)
│                     │  - 拓扑管理
│                     │  - **不参与算力调度**
└─────────────────────┘
```

### 核心组件

| 组件 | 运行方式 | 端口 | 说明 |
|------|---------|------|------|
| **边缘服务器** | 宿主机 | 8080 | 算力调度中心 |
| **Workers** | 临时进程 | - | 执行具体推理任务 |
| **Rust 节点** | Docker 容器 | 9235 | P2P 通信（独立） |

---

## 快速开始

### 前置条件

```bash
# 1. Python 3.8+
python3 --version

# 2. 安装依赖
pip install -r williw-workers/requirements.txt

# 3. Docker Desktop（可选，用于 Rust 节点）
# https://www.docker.com/products/docker-desktop/
```

### 一键启动

```bash
# 启动边缘服务器（算力共享核心）
cd williw-workers
python -m edge_server.api_server --port 8080

# 或使用启动脚本
./start.sh --workers
```

### 测试推理

```bash
# 健康检查
curl http://localhost:8080/api/health

# 发送推理请求
curl -X POST http://localhost:8080/api/inference \
  -H "Content-Type: application/json" \
  -d '{
    "model_name": "bert-base-uncased",
    "model_source": "huggingface",
    "input_data": {"text": "Hello world"},
    "parameters": {"batch_size": 1}
  }'
```

### Python 客户端

```python
from williw-workers.interface_layer.app_client import InferenceClient

# 创建客户端
client = InferenceClient("http://localhost:8080")

# 发送推理请求
result = client.inference(
    model_name="bert-base-uncased",
    input_data={"text": "Hello world"},
    parameters={"batch_size": 1}
)

print(f"推理状态：{result['status']}")
print(f"使用的节点：{result.get('nodes_used', [])}")
print(f"推理时间：{result.get('inference_time', 0):.2f} ms")
```

---

## 使用场景

### 场景 1: 单机推理（Mac MPS）

```bash
# Mac (M2/M3) 使用 MPS 加速
./start.sh --workers

# 测试
python3 williw-workers/example_usage.py
```

**适用:**
- 个人开发测试
- 小模型推理
- 能效优先场景

---

### 场景 2: 单机推理（Windows CUDA）

```powershell
# Windows (NVIDIA GPU) 使用 CUDA 加速
.\start.ps1 -workers

# 测试
python williw-workers\example_usage.py
```

**适用:**
- 大模型推理
- 性能优先场景
- 生产环境

---

### 场景 3: 局域网算力共享

```
Mac (M2)                          Windows (RTX 3080)
192.168.1.100                     192.168.1.101
┌─────────────────┐               ┌─────────────────┐
│ 边缘服务器      │               │ Worker 节点      │
│ - 调度中心      │               │ - 执行推理      │
│ - 算力估算      │               │ - 层 5-8        │
└─────────────────┘               └─────────────────┘
```

**配置:**
```python
# 在 Mac 上启动边缘服务器
python -m edge_server.api_server --port 8080

# 在 Windows 上注册为 Worker
curl -X POST http://192.168.1.100:8080/api/worker/register \
  -d '{"worker_id": "win-worker-1", "gpu": "RTX 3080"}'
```

**适用:**
- 多设备协同
- 算力互补
- 降低成本

---

## 核心机制

### Workers 生命周期

```
1. Worker 启动
   ↓
2. 注册到边缘服务器
   ↓
3. 等待任务
   ↓
4. 接收任务（模型分片）
   ↓
5. 执行推理（前向传播）
   ↓
6. 返回结果（激活值）
   ↓
7. Worker 退出（资源释放）
```

### 算力估算（保守策略）

```python
# williw-workers/edge_server/compute_estimator.py

# 1. 基础算力 = 参数量 × 2 (MAC 操作)
base_compute = num_params * 2

# 2. 激活值开销 = 基础算力 × 1.5
activation_overhead = base_compute * 1.5

# 3. 内存访问开销 = (基础 + 激活) × 1.3
memory_overhead = (base_compute + activation_overhead) * 1.3

# 4. 安全系数 = 总开销 × 1.5 (可算多不可算少)
total_compute = memory_overhead * 1.5

# 最终：约为基础算力的 3 倍
```

### 节点选择算法

```python
# williw-workers/algorithms/node_selection.py

# 1. 过滤满足基本约束的节点
- 在线状态
- 空闲状态
- GPU 可用性
- 资源使用率

# 2. 按算力排序
compute_power = estimate_compute_power(node)

# 3. 选择主节点（算力最强的前 N 个）
primary_nodes = sorted_nodes[:num_primary]

# 4. 选择备份节点
backup_nodes = sorted_nodes[num_primary:num_primary+backup_count]
```

---

## API 参考

### 边缘服务器 API (端口 8080)

#### POST /api/inference

接收推理请求

**请求体:**
```json
{
    "model_name": "bert-base-uncased",
    "model_source": "huggingface",
    "input_data": {
        "text": "Hello world"
    },
    "parameters": {
        "batch_size": 1
    }
}
```

**响应:**
```json
{
    "status": "success",
    "result": {...},
    "nodes_used": ["worker-1", "worker-2"],
    "inference_time": 123.45,
    "model_shards": 3,
    "total_compute": 5000.0
}
```

#### POST /api/worker/register

注册 Worker

**请求体:**
```json
{
    "worker_id": "worker-1",
    "capabilities": {
        "device_type": "cuda",
        "gpu_name": "RTX 3080",
        "memory_gb": 24,
        "compute_power": 68.0
    }
}
```

#### GET /api/health

健康检查

**响应:**
```json
{
    "status": "healthy",
    "service": "williw-use-edge-server"
}
```

#### GET /api/models

列出可用模型

---

## 算力共享流程

### 完整工作流

```
1. 用户发起推理请求
   ↓
2. 边缘服务器获取模型
   - Hugging Face 下载
   - 本地模型仓库加载
   ↓
3. 模型转换（如果需要）
   - ONNX → PyTorch
   - 读取 state_dict
   ↓
4. 算力估算（保守策略）
   - 总算力需求
   - 内存需求
   - GPU 需求
   ↓
5. 获取可用节点
   - 从 williw-master API
   - 或模拟数据
   ↓
6. 节点选择算法
   - 资源约束检查
   - 算力估算
   - 选择主节点和备份节点
   ↓
7. 模型切分
   - 按层切分
   - 分配给 Workers
   ↓
8. 分布式推理
   Worker A (层 1-4)
       ↓ 激活值
   Worker B (层 5-8)
       ↓ 激活值
   Worker C (层 9-12)
   ↓
9. 结果集成
   ↓
10. 返回给用户
```

---

## 故障排查

### 问题 1: 边缘服务器无法启动

```bash
# 检查依赖
pip list | grep -E "flask|torch"

# 重新安装
pip install -r williw-workers/requirements.txt --force-reinstall

# 查看日志
cat workers.log
```

### 问题 2: 推理请求超时

```bash
# 检查服务器状态
curl http://localhost:8080/api/health

# 检查模型下载
# 首次运行需要下载模型，可能需要较长时间
```

### 问题 3: GPU 不可用

```bash
# Mac: 检查 MPS
python3 -c "import torch; print(torch.backends.mps.is_available())"

# Windows/Linux: 检查 CUDA
python3 -c "import torch; print(torch.cuda.is_available())"
```

---

## 下一步

### 进阶文档

- [Workers 机制详解](docs/COMPUTE_SHARING_WORKERS.md)
- [算法层说明](docs/COMPUTE_SHARING_ALGORITHMS.md)
- [使用示例](docs/COMPUTE_SHARING_EXAMPLES.md)

### 相关文档

- [部署指南](docs/WORKERS_DEPLOYMENT.md)
- [架构总结](ARCHITECTURE_SUMMARY.md)

---

*最后更新：2024-02-17*

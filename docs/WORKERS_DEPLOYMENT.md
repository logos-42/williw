# williw Workers 部署指南

> **任务驱动的动态算力池** - 边缘服务器部署与配置

---

## 快速开始

### 前置条件

```bash
# 1. Python 3.8+
python3 --version

# 2. 安装依赖
pip install -r williw-workers/requirements.txt

# 3. 验证 GPU 支持
python3 -c "import torch; print('CUDA:', torch.cuda.is_available())"
```

### 一键启动

```bash
# 启动边缘服务器
cd williw-workers
python -m edge_server.api_server --port 8080

# 或使用启动脚本
./start.sh --workers
```

### 验证部署

```bash
# 健康检查
curl http://localhost:8080/api/health

# 测试推理
curl -X POST http://localhost:8080/api/inference \
  -H "Content-Type: application/json" \
  -d '{
    "model_name": "bert-base-uncased",
    "input_data": {"text": "Hello world"}
  }'
```

### 组件说明

| 组件 | 运行方式 | 端口 | 说明 |
|------|---------|------|------|
| **Rust 节点** | Docker 容器 | 9235 | P2P 通信、AI 决策 |
| **GPU 推理服务** | 宿主机 | 8000 | PyTorch 推理（MPS/CUDA） |
| **Workers 边缘服务器** | 宿主机 | 8080 | 分布式推理调度 |

---

## 快速开始

### Step 1: 安装依赖

```bash
# 所有平台
pip install -r requirements-gpu.txt

# Docker Desktop
# https://www.docker.com/products/docker-desktop/
```

### Step 2: 启动服务

```bash
# 启动所有服务（推荐）
./start.sh --all

# Windows (PowerShell)
.\start.ps1 -All
```

### Step 3: 验证

```bash
# 查看状态
./start.sh --status

# 测试 Workers API
curl http://localhost:8080/api/health

# 测试推理
curl -X POST http://localhost:8080/api/inference \
  -H "Content-Type: application/json" \
  -d '{
    "model_name": "bert-base-uncased",
    "input_data": {"text": "Hello world"}
  }'
```

---

## 容器配置

### Dockerfile

```dockerfile
# Rust 节点（Docker 容器）
FROM rust:1.75-bookworm AS builder
# ... 构建 Rust 项目

FROM debian:bookworm-slim AS production
# ... 运行环境

# 环境变量：Workers 边缘服务器地址
ENV WILLIW_WORKERS_EDGE_SERVER_URL=http://host.docker.internal:8080

ENTRYPOINT ["williw"]
```

### docker-compose.yml

```yaml
services:
  williw-node:
    build: .
    container_name: williw-node
    ports:
      - "9235:9235/udp"
      - "9235:9235/tcp"
    environment:
      - WILLIW_WORKERS_EDGE_SERVER_URL=http://host.docker.internal:8080
    network_mode: host
    extra_hosts:
      - "host.docker.internal:host-gateway"
```

---

## Workers 机制

### 工作流程

```
1. 接口层发起推理请求
   ↓
2. 边缘服务器分析模型
   - 读取 state_dict
   - 估算算力需求
   ↓
3. 算法层选择 Workers
   - 根据算力需求
   - 根据网络位置
   - 根据可用性
   ↓
4. 模型切分
   - 层 1-4 → Worker A
   - 层 5-8 → Worker B
   - 层 9-12 → Worker C
   ↓
5. 分布式推理
   Worker A → Worker B → Worker C
   (激活值传递)
   ↓
6. 结果回收
   边缘服务器收集最终结果
   ↓
7. Workers 退出
   资源释放
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
    "inference_time": 123.45
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

## 故障排查

### 问题 1: Workers 边缘服务器无法启动

```bash
# 检查依赖
pip list | grep -E "flask|torch"

# 重新安装
pip install -r williw-workers/requirements.txt --force-reinstall

# 查看日志
cat workers.log
```

### 问题 2: Rust 节点无法访问 Workers

```bash
# 测试 Workers API
curl http://localhost:8080/api/health

# 检查容器内网络
docker exec williw-node curl http://host.docker.internal:8080/api/health
```

### 问题 3: GPU 推理失败

```bash
# Mac: 检查 MPS
python3 -c "import torch; print(torch.backends.mps.is_available())"

# Windows/Linux: 检查 CUDA
python3 -c "import torch; print(torch.cuda.is_available())"
```

---

## 参考文档

- [Workers 机制详解](docs/WORKERS_MECHANISM.md)
- [Workers 架构总结](WORKERS_SUMMARY.md)
- [williw-workers README](williw-workers/README.md)

---

*最后更新：2024-02-17*

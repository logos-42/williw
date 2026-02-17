# williw Workers 分布式算力机制

> **动态 Workers 算力池** - 临时加入、任务驱动、结果回收

---

## 核心架构

### Workers 机制说明

```
不是广播发现节点的机制，而是：

1. Worker 启动 → 注册到节点
2. 节点分配任务 → Worker 执行
3. 任务完成 → Worker 退出
4. 节点获取结果 → 传递给下一个环节
```

### 完整流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    williw 分布式算力网络                          │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────┐
│  接口层 (Tauri App)  │
│  - 用户请求推理      │
│  - 上传节点信息      │
└──────────┬──────────┘
           │ HTTP POST
           │ /api/inference
           ↓
┌─────────────────────────────────────────────────────────────────┐
│  边缘服务器 (Edge Server) - williw-workers/edge_server          │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 1. 模型获取 (model_fetcher.py)                          │   │
│  │    - Hugging Face 下载                                   │   │
│  │    - 本地模型仓库加载                                     │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 2. 模型转换 (model_converter.py)                        │   │
│  │    - ONNX → PyTorch                                     │   │
│  │    - 读取 state_dict                                     │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 3. 算力估算 (compute_estimator.py)                      │   │
│  │    - 保守估算（安全系数 1.5）                             │   │
│  │    - 计算算力/内存需求                                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 4. 节点信息获取 (interface_layer/node_info_api.py)      │   │
│  │    - 从 williw-master Rust 节点获取                        │   │
│  │    - 转换为 MobileNode 对象                               │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 5. 调用算法层 (lkc/algorithms)                          │   │
│  │    - 节点选择算法                                        │   │
│  │    - D-CACO 路径优化                                      │   │
│  │    - 资源分配算法                                        │   │
│  │    - 模型切分器                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 6. 分布式推理 (models/inference_engine.py)              │   │
│  │    - Worker 1: 层 1-4                                   │   │
│  │    - Worker 2: 层 5-8                                   │   │
│  │    - Worker 3: 层 9-12                                  │   │
│  │    - 激活值传递                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ 7. 结果集成 (models/result_merger.py)                   │   │
│  │    - 合并各 Worker 输出                                    │   │
│  │    - 返回最终结果                                        │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────┬──────────────────────────────────────────────────────┘
           │ HTTP Response
           │ {
           │   "status": "success",
           │   "result": {...},
           │   "nodes_used": ["worker-1", "worker-2", "worker-3"],
           │   "inference_time": 123.4
           │ }
           ↓
┌─────────────────────┐
│  接口层 (Tauri App)  │
│  显示推理结果        │
└─────────────────────┘
```

---

## Workers 工作机制

### 临时加入机制

```
┌──────────────┐
│  Worker A    │  启动
│  (GPU: RTX)  │  ↓
└──────┬───────┘  注册到边缘服务器
       │        ↓
       │     边缘服务器记录:
       │     - worker_id: "worker-a"
       │     - capabilities: {gpu: "RTX 3080", memory: 24GB}
       │     - status: "available"
       │
       │  ← 分配任务：处理层 1-4
       │
       │  执行推理
       │
       │  返回结果 + 退出
       ↓
┌──────────────┐
│ 边缘服务器    │  获取结果
│  传递给       │  → Worker B (层 5-8)
│ 下一个 Worker │
└──────────────┘
```

### 任务驱动流程

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

---

## 项目结构

```
williw-workers/
├── edge_server/              # 边缘服务器
│   ├── __init__.py
│   ├── api_server.py         # Flask API 服务器
│   ├── model_fetcher.py      # 模型获取（Hugging Face/本地）
│   ├── model_converter.py    # ONNX → PyTorch 转换
│   ├── compute_estimator.py  # 模型算力估算（保守）
│   └── workflow_orchestrator.py  # 工作流编排器
├── interface_layer/          # 接口层（从 williw-master 读取节点信息）
│   ├── __init__.py
│   ├── node_info_api.py      # 节点信息客户端
│   └── app_client.py         # 客户端示例
├── models/                   # 模型相关（复用 lkc，扩展推理功能）
│   ├── __init__.py
│   ├── inference_engine.py   # 分布式推理引擎
│   └── result_merger.py      # 结果集成
├── algorithms/               # 算法层（复用 lkc）
│   └── (链接到 lkc/algorithms)
├── node_client/              # 节点客户端示例
│   ├── demo_rust_worker_plan.py
│   ├── demo_process_request.json
│   └── demo_process_response.json
├── utils/                    # 工具函数
│   ├── __init__.py
│   └── config.py             # 配置管理
├── requirements.txt
├── README.md
└── example_usage.py          # 使用示例
```

---

## 快速开始

### 1. 安装依赖

```bash
cd williw-workers
pip install -r requirements.txt
```

### 2. 启动边缘服务器

```bash
python -m edge_server.api_server --port 8080
```

### 3. 测试推理

```bash
# 运行示例
python example_usage.py
```

### 4. 使用客户端

```python
from interface_layer.app_client import InferenceClient

client = InferenceClient(server_url="http://localhost:8080")
result = client.inference(
    model_name="bert-base-uncased",
    model_source="huggingface",
    input_data={"text": "Hello world"},
    parameters={"batch_size": 1}
)
print(result)
```

---

## API 参考

### 边缘服务器 API

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

#### GET /api/models

列出可用模型

---

## 与 Rust 节点集成

### 上传节点信息

```rust
// 在 Tauri app 中
use williw::commands::workers_commands::upload_device_info_to_workers;

// 上传设备信息到边缘服务器
let result = upload_device_info_to_workers(state).await?;
println!("节点信息已上传：{}", result);
```

### 请求推理

```rust
// 请求分布式推理
let result = request_inference_from_workers(
    model_id,
    input_data,
    state
).await?;
```

---

## Workers 生命周期

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

## 算力估算（保守策略）

```python
# compute_estimator.py

# 基础算力 = 参数量 × 2 (MAC 操作)
base_compute = num_params * 2

# 激活值开销 = 基础算力 × 1.5
activation_overhead = base_compute * 1.5

# 内存访问开销 = (基础 + 激活) × 1.3
memory_overhead = (base_compute + activation_overhead) * 1.3

# 安全系数 = 总开销 × 1.5 (可算多不可算少)
total_compute = memory_overhead * 1.5

# 最终：约为基础算力的 3 倍
```

---

## 参考文档

- [williw-workers README](williw-workers/README.md)
- [集成完整方案](williw-workers/集成完整方案.md)
- [项目完成总结](williw-workers/项目完成总结.md)

---

*最后更新：2024-02-17*

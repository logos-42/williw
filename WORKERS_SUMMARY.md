# williw Workers 架构总结

> **任务驱动的动态算力池** - 不是广播发现，而是临时加入/退出

---

## 核心机制

### ❌ 不是广播发现

```
之前的理解（错误）:
- 节点通过 UDP 广播互相发现
- 持续在线，维护连接
- 类似 P2P 网络的节点发现
```

### ✅ 而是任务驱动的 Workers 池

```
实际实现（正确）:
1. Worker 启动 → 注册到边缘服务器
2. 边缘服务器分配任务 → Worker 执行
3. 任务完成 → Worker 返回结果并退出
4. 边缘服务器获取结果 → 传递给下一个 Worker
```

---

## 架构组件

```
┌─────────────────────────────────────────────────────────┐
│                    williw 架构                           │
└─────────────────────────────────────────────────────────┘

┌─────────────────────┐
│  Tauri App          │  用户接口
│  (williw-master)    │  - 发起推理请求
│                     │  - 上传节点信息
└──────────┬──────────┘
           │
           ↓ HTTP POST /api/inference
┌─────────────────────────────────────────────────────────┐
│  边缘服务器 (williw-workers/edge_server)                 │
│                                                         │
│  - 接收推理请求                                         │
│  - 模型获取与转换                                        │
│  - 算力估算                                             │
│  - 调用算法层选择 Workers                                │
│  - 分配任务给 Workers                                    │
│  - 回收结果                                             │
└────────────┬────────────────────────────────────────────┘
             │
             ├────→ Worker A (层 1-4) → 退出
             ├────→ Worker B (层 5-8) → 退出
             └────→ Worker C (层 9-12) → 退出
```

---

## 文件结构

```
williw-workers/
├── edge_server/              # ⭐ 边缘服务器（核心）
│   ├── api_server.py         # Flask API 服务器
│   ├── model_fetcher.py      # 模型获取
│   ├── model_converter.py    # 模型转换
│   ├── compute_estimator.py  # 算力估算
│   └── workflow_orchestrator.py  # 工作流编排
├── interface_layer/          # 接口层
│   ├── node_info_api.py      # 从 williw-master 获取节点信息
│   └── app_client.py         # 客户端示例
├── models/                   # 模型管理
│   ├── inference_engine.py   # 分布式推理引擎
│   └── result_merger.py      # 结果集成
├── algorithms/               # 算法层（复用 lkc）
│   └── dcaco_algorithm.py    # D-CACO 蚁群算法
├── node_client/              # 节点客户端示例
│   └── demo_rust_worker_plan.py
└── utils/                    # 工具函数
```

---

## 工作流程

### 完整 8 步流程

```
1. 模型获取
   边缘服务器从 Hugging Face/本地仓库获取模型

2. 模型转换
   ONNX → PyTorch，读取 state_dict

3. 算力估算
   保守估算（安全系数 1.5，可算多不可算少）

4. 节点信息获取
   从 williw-master Rust 节点获取设备信息

5. 算法层调用
   - 节点选择算法
   - D-CACO 路径优化
   - 资源分配算法
   - 模型切分器

6. 分布式推理
   Worker A (层 1-4) → Worker B (层 5-8) → Worker C (层 9-12)
   激活值在 Workers 间传递

7. 结果回收
   边缘服务器收集最终结果

8. Workers 退出
   资源释放
```

---

## 使用方式

### 启动边缘服务器

```bash
cd williw-workers
python -m edge_server.api_server --port 8080
```

### 发起推理请求

```python
from interface_layer.app_client import InferenceClient

client = InferenceClient("http://localhost:8080")
result = client.inference(
    model_name="bert-base-uncased",
    model_source="huggingface",
    input_data={"text": "Hello world"},
    parameters={"batch_size": 1}
)
print(result)
```

### Rust 节点集成

```rust
// 上传节点信息
upload_device_info_to_workers(state).await?;

// 请求推理
request_inference_from_workers(model_id, input_data, state).await?;
```

---

## 关键特点

### 1. 任务驱动

- Workers 不是持续在线
- 有任务时启动，任务完成退出
- 资源利用率高

### 2. 算力估算保守

```python
# 最终算力 = 基础算力 × 3（安全系数）
total_compute = base_compute * 1.5 * 1.3 * 1.5
```

### 3. 激活值传递

```
Worker A (层 1-4)
    ↓ 激活值
Worker B (层 5-8)
    ↓ 激活值
Worker C (层 9-12)
    ↓ 最终结果
```

### 4. 与 Rust 节点解耦

- williw-master: P2P 通信、设备管理
- williw-workers: 分布式推理、算力调度
- 通过 HTTP API 通信

---

## 与之前理解的对比

| 特性 | 之前理解（广播发现） | 实际实现（Workers 池） |
|------|-------------------|---------------------|
| 节点发现 | UDP 广播 | 注册到边缘服务器 |
| 在线状态 | 持续在线 | 临时加入/退出 |
| 通信方式 | P2P 直接通信 | 边缘服务器调度 |
| 任务分配 | 节点间协商 | 边缘服务器统一分配 |
| 结果回收 | 节点间传递 | 边缘服务器收集 |
| 资源管理 | 分布式 | 集中式调度 |

---

## 参考文档

- [Workers 机制详解](docs/WORKERS_MECHANISM.md)
- [williw-workers README](williw-workers/README.md)
- [集成完整方案](williw-workers/集成完整方案.md)
- [项目完成总结](williw-workers/项目完成总结.md)

---

*最后更新：2024-02-17*

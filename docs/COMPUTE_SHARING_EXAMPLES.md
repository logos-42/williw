# 算力共享使用示例

> **从入门到进阶** - 单机 · 局域网 · 自定义 Workers

---

## 示例 1: 单机推理（Mac MPS）

### 场景说明

在 Mac (M2/M3) 上使用 MPS 加速进行模型推理。

### 启动服务

```bash
# 1. 启动边缘服务器
cd williw-workers
python -m edge_server.api_server --port 8080

# 2. 验证启动成功
curl http://localhost:8080/api/health
```

### Python 客户端

```python
from williw_workers.interface_layer.app_client import InferenceClient

# 创建客户端
client = InferenceClient("http://localhost:8080")

# 健康检查
if not client.health_check():
    print("❌ 服务器未响应")
    exit(1)

print("✅ 服务器已启动")

# 发送推理请求
result = client.inference(
    model_name="bert-base-uncased",
    model_source="huggingface",
    input_data={
        "text": "Hello, world! This is a test of williw compute sharing."
    },
    parameters={
        "batch_size": 1
    }
)

# 处理结果
if result['status'] == 'success':
    print(f"\n✅ 推理成功")
    print(f"使用的节点：{result.get('nodes_used', [])}")
    print(f"推理时间：{result.get('inference_time', 0):.2f} ms")
    print(f"模型分片数：{result.get('model_shards', 0)}")
    
    if 'result' in result:
        print(f"推理结果：{result['result']}")
else:
    print(f"\n❌ 推理失败：{result.get('message', 'Unknown error')}")
```

### 预期输出

```
✅ 服务器已启动

开始执行推理工作流
======================================================================

步骤 1: 获取模型...
✓ 模型获取成功：bert-base-uncased

步骤 2: 读取 state_dict...
✓ state_dict 读取成功：110M 参数，440.00 MB

步骤 3: 估算模型算力需求（保守估算）...
✓ 算力估算完成:
  - 总算力需求：5000.00 GFLOPS
  - 内存需求：2.50 GB
  - 需要 GPU: True
  - 估算延迟：50.00 ms

步骤 4: 获取可用节点...
✓ 获取到 1 个可用节点

步骤 5: 调用算法层...
✓ 节点选择完成：1 个主节点，0 个备份节点
✓ 模型切分完成：1 个分片

✅ 推理成功
使用的节点：['local-worker']
推理时间：125.45 ms
```

---

## 示例 2: 单机推理（Windows CUDA）

### 场景说明

在 Windows (NVIDIA GPU) 上使用 CUDA 加速进行模型推理。

### 启动服务

```powershell
# 1. 启动边缘服务器
cd williw-workers
python -m edge_server.api_server --port 8080

# 2. 验证 GPU 可用
python -c "import torch; print('CUDA:', torch.cuda.is_available())"
```

### Python 客户端

```python
import torch
from williw_workers.interface_layer.app_client import InferenceClient

# 检查 GPU
print(f"GPU 可用：{torch.cuda.is_available()}")
if torch.cuda.is_available():
    print(f"GPU 型号：{torch.cuda.get_device_name(0)}")
    print(f"GPU 显存：{torch.cuda.get_device_properties(0).total_memory / 1024**3:.1f} GB")

# 创建客户端
client = InferenceClient("http://localhost:8080")

# 发送推理请求（使用更大的模型）
result = client.inference(
    model_name="gpt2",
    model_source="huggingface",
    input_data={
        "text": "Once upon a time",
        "max_length": 100
    },
    parameters={
        "batch_size": 1
    }
)

print(f"\n推理结果:")
print(f"状态：{result['status']}")
print(f"推理时间：{result.get('inference_time', 0):.2f} ms")
```

### 预期输出

```
GPU 可用：True
GPU 型号：NVIDIA GeForce RTX 3080
GPU 显存：10.0 GB

推理结果:
状态：success
推理时间：85.32 ms
```

---

## 示例 3: 局域网多机推理

### 场景说明

在局域网上使用多台设备协同推理：
- Mac (M2): 边缘服务器 + Worker
- Windows (RTX 3080): Worker
- Linux (RTX 4090): Worker

### 网络拓扑

```
┌─────────────────┐
│ Mac (M2)        │  192.168.1.100
│ 边缘服务器      │
│ + Worker        │
└────────┬────────┘
         │ 局域网
    ┌────┴────┐
    │         │
┌───▼───┐ ┌──▼──────┐
│ Win   │ │ Linux   │
│ RTX3080│ │ RTX4090 │
│ Worker│ │ Worker  │
└───────┘ └─────────┘
```

### 步骤 1: 在 Mac 上启动边缘服务器

```bash
# Mac (192.168.1.100)
python -m edge_server.api_server --port 8080 --host 0.0.0.0
```

### 步骤 2: 在 Windows 上注册 Worker

```powershell
# Windows (192.168.1.101)
import requests

# 注册到 Mac 的边缘服务器
response = requests.post(
    "http://192.168.1.100:8080/api/worker/register",
    json={
        "worker_id": "win-rtx3080",
        "capabilities": {
            "device_type": "cuda",
            "gpu_name": "RTX 3080",
            "memory_gb": 10,
            "compute_power": 68.0
        }
    }
)

print(f"注册结果：{response.json()}")
```

### 步骤 3: 在 Linux 上注册 Worker

```bash
# Linux (192.168.1.102)
python3 << EOF
import requests

response = requests.post(
    "http://192.168.1.100:8080/api/worker/register",
    json={
        "worker_id": "linux-rtx4090",
        "capabilities": {
            "device_type": "cuda",
            "gpu_name": "RTX 4090",
            "memory_gb": 24,
            "compute_power": 128.0
        }
    }
)

print(f"注册结果：{response.json()}")
EOF
```

### 步骤 4: 在 Mac 上发起推理请求

```python
from williw_workers.interface_layer.app_client import InferenceClient

client = InferenceClient("http://localhost:8080")

# 查看可用节点
nodes_response = requests.get("http://localhost:8080/api/nodes")
print(f"可用节点：{nodes_response.json()}")

# 发起推理请求（自动选择最优 Workers）
result = client.inference(
    model_name="bert-base-uncased",
    input_data={"text": "Hello from Mac!"}
)

print(f"\n推理完成")
print(f"使用的 Workers: {result.get('nodes_used', [])}")
print(f"推理时间：{result.get('inference_time', 0):.2f} ms")
```

### 预期输出

```
可用节点：[
    {'worker_id': 'win-rtx3080', 'gpu': 'RTX 3080'},
    {'worker_id': 'linux-rtx4090', 'gpu': 'RTX 4090'}
]

推理完成
使用的 Workers: ['linux-rtx4090', 'win-rtx3080']
推理时间：45.67 ms
```

---

## 示例 4: 自定义 Workers

### 场景说明

实现自定义 Worker 逻辑，例如：
- 特定模型优化
- 自定义调度策略
- 特殊硬件支持

### 自定义 Worker 类

```python
import torch
from typing import Dict, Any

class CustomWorker:
    """自定义 Worker 实现"""
    
    def __init__(self, worker_id: str, server_url: str):
        self.worker_id = worker_id
        self.server_url = server_url
        self.device = self._detect_device()
        self.model = None
    
    def _detect_device(self):
        """检测可用设备"""
        if torch.cuda.is_available():
            return "cuda"
        elif hasattr(torch.backends, 'mps') and torch.backends.mps.is_available():
            return "mps"
        else:
            return "cpu"
    
    def register(self):
        """注册到边缘服务器"""
        import requests
        
        response = requests.post(
            f"{self.server_url}/api/worker/register",
            json={
                "worker_id": self.worker_id,
                "capabilities": {
                    "device_type": self.device,
                    "gpu_name": torch.cuda.get_device_name(0) if self.device == "cuda" else "Unknown",
                    "memory_gb": self._get_memory_gb(),
                    "compute_power": self._estimate_compute_power()
                }
            }
        )
        
        return response.json()
    
    def _get_memory_gb(self) -> float:
        """获取可用内存（GB）"""
        if self.device == "cuda":
            return torch.cuda.get_device_properties(0).total_memory / 1024**3
        else:
            import psutil
            return psutil.virtual_memory().total / (1024**3)
    
    def _estimate_compute_power(self) -> float:
        """估算算力"""
        if self.device == "cuda":
            gpu_name = torch.cuda.get_device_name(0).lower()
            gpu_compute_map = {
                'rtx 4090': 128.0,
                'rtx 3080': 68.0,
                'rtx 3070': 50.0,
            }
            for name, power in gpu_compute_map.items():
                if name in gpu_name:
                    return power
            return 50.0  # 默认
        elif self.device == "mps":
            return 8.0  # Apple Silicon
        else:
            return 2.0  # CPU
    
    async def execute_task(self, task: Dict[str, Any]):
        """执行推理任务"""
        print(f"Worker {self.worker_id} 接收任务：{task['task_id']}")
        
        # 1. 加载模型分片
        model_shard = task['model_shard']
        self.model = self._load_model_shard(model_shard)
        
        # 2. 执行推理
        input_data = torch.tensor(task['input_data']).to(self.device)
        
        with torch.no_grad():
            output = self.model(input_data)
        
        # 3. 返回结果
        return {
            "task_id": task['task_id'],
            "output": output.cpu().numpy().tolist(),
            "inference_time": 0.0,  # 实际应计算
            "device": self.device
        }
    
    def _load_model_shard(self, shard: Dict[str, Any]):
        """加载模型分片"""
        # 实现模型加载逻辑
        pass

# 使用示例
async def main():
    # 创建 Worker
    worker = CustomWorker(
        worker_id="my-custom-worker",
        server_url="http://localhost:8080"
    )
    
    # 注册
    result = worker.register()
    print(f"注册结果：{result}")
    
    # 等待任务
    print("Worker 已就绪，等待任务...")

# 运行
import asyncio
asyncio.run(main())
```

---

## 示例 5: 批量推理

### 场景说明

处理多个推理请求，优化吞吐量。

### 批量处理客户端

```python
from concurrent.futures import ThreadPoolExecutor
from williw_workers.interface_layer.app_client import InferenceClient

class BatchInferenceClient:
    """批量推理客户端"""
    
    def __init__(self, server_url: str, max_workers: int = 10):
        self.client = InferenceClient(server_url)
        self.executor = ThreadPoolExecutor(max_workers=max_workers)
    
    def batch_inference(self, requests: list):
        """
        批量发送推理请求
        
        Args:
            requests: 请求列表
                [
                    {"text": "Hello 1"},
                    {"text": "Hello 2"},
                    ...
                ]
        
        Returns:
            结果列表
        """
        futures = []
        
        for req in requests:
            future = self.executor.submit(
                self.client.inference,
                model_name="bert-base-uncased",
                input_data=req,
                parameters={"batch_size": 1}
            )
            futures.append(future)
        
        # 收集结果
        results = []
        for future in futures:
            results.append(future.result())
        
        return results

# 使用示例
client = BatchInferenceClient("http://localhost:8080", max_workers=5)

# 准备批量请求
requests = [
    {"text": f"Test message {i}"}
    for i in range(20)
]

# 发送批量请求
results = client.batch_inference(requests)

# 统计结果
success_count = sum(1 for r in results if r['status'] == 'success')
total_time = sum(r.get('inference_time', 0) for r in results if r['status'] == 'success')

print(f"批量推理完成")
print(f"成功：{success_count}/{len(results)}")
print(f"总时间：{total_time:.2f} ms")
print(f"平均时间：{total_time/success_count:.2f} ms/请求")
```

---

## 示例 6: 模型选择与性能对比

### 场景说明

测试不同模型的性能表现。

### 性能测试脚本

```python
import time
from williw_workers.interface_layer.app_client import InferenceClient

class ModelBenchmark:
    """模型性能测试"""
    
    def __init__(self, server_url: str):
        self.client = InferenceClient(server_url)
    
    def benchmark_model(self, model_name: str, input_text: str, iterations: int = 5):
        """
        测试模型性能
        
        Returns:
            性能统计
        """
        times = []
        
        for i in range(iterations):
            start = time.time()
            
            result = self.client.inference(
                model_name=model_name,
                input_data={"text": input_text}
            )
            
            end = time.time()
            
            if result['status'] == 'success':
                times.append((end - start) * 1000)  # 转换为 ms
        
        # 统计
        if times:
            return {
                "model": model_name,
                "avg_time": sum(times) / len(times),
                "min_time": min(times),
                "max_time": max(times),
                "success_rate": len(times) / iterations
            }
        else:
            return {"model": model_name, "error": "All failed"}

# 测试多个模型
benchmark = ModelBenchmark("http://localhost:8080")

models = [
    "bert-base-uncased",
    "gpt2",
    "distilbert-base-uncased",
]

print("模型性能对比:\n")
print(f"{'模型':<30} {'平均 (ms)':<15} {'最小 (ms)':<15} {'最大 (ms)':<15}")
print("-" * 75)

for model_name in models:
    result = benchmark.benchmark_model(model_name, "Hello world", iterations=5)
    
    if 'error' not in result:
        print(f"{result['model']:<30} "
              f"{result['avg_time']:<15.2f} "
              f"{result['min_time']:<15.2f} "
              f"{result['max_time']:<15.2f}")
    else:
        print(f"{result['model']:<30} 测试失败")
```

---

## 故障排查

### 问题 1: 连接被拒绝

```bash
# 检查服务器是否运行
curl http://localhost:8080/api/health

# 检查防火墙
# Mac: 系统偏好设置 > 安全性 > 防火墙
# Windows: Windows Defender 防火墙

# 检查端口占用
lsof -i :8080
```

### 问题 2: 推理超时

```python
# 增加超时时间
client = InferenceClient("http://localhost:8080", timeout=600)

# 使用更小的模型
result = client.inference(
    model_name="distilbert-base-uncased",  # 更小的模型
    input_data={"text": "Hello"}
)
```

### 问题 3: 内存不足

```python
# 减小 batch_size
result = client.inference(
    model_name="bert-base-uncased",
    parameters={"batch_size": 1}  # 最小
)

# 关闭其他应用释放内存
```

---

## 参考文档

- [算力共享总览](COMPUTE_SHARING.md)
- [Workers 机制](COMPUTE_SHARING_WORKERS.md)
- [算法层说明](COMPUTE_SHARING_ALGORITHMS.md)

---

*最后更新：2024-02-17*

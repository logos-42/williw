# Workers 机制详解

> **任务驱动的动态算力池** - 临时加入、任务驱动、结果回收

---

## Workers 生命周期

### 完整流程

```
┌─────────────┐
│ 1. Worker 启动│
│ - 初始化环境 │
│ - 注册能力   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 2. 等待任务  │
│ - 心跳保持   │
│ - 状态上报   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 3. 接收任务  │
│ - 模型分片   │
│ - 输入数据   │
│ - 目标设备   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 4. 执行推理  │
│ - 前向传播   │
│ - 激活值计算 │
│ - 结果验证   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 5. 返回结果  │
│ - 激活值     │
│ - 状态信息   │
│ - 性能指标   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ 6. Worker 退出│
│ - 释放资源   │
│ - 清理缓存   │
└─────────────┘
```

---

## Workers 注册

### 注册 API

```python
# POST /api/worker/register
curl -X POST http://localhost:8080/api/worker/register \
  -H "Content-Type: application/json" \
  -d '{
    "worker_id": "worker-1",
    "capabilities": {
        "device_type": "cuda",
        "gpu_name": "RTX 3080",
        "memory_gb": 24,
        "compute_power": 68.0,
        "network_latency": 10.0,
        "bandwidth": 100.0
    }
}'
```

### 响应

```json
{
    "status": "success",
    "worker_id": "worker-1",
    "message": "Worker 注册成功",
    "assigned_tasks": 0,
    "next_heartbeat": 30
}
```

### Python 注册示例

```python
import requests

def register_worker(server_url: str, worker_id: str, gpu_info: dict):
    """注册 Worker 到边缘服务器"""
    
    payload = {
        "worker_id": worker_id,
        "capabilities": gpu_info
    }
    
    response = requests.post(
        f"{server_url}/api/worker/register",
        json=payload,
        timeout=10
    )
    
    if response.status_code == 200:
        result = response.json()
        print(f"✅ Worker 注册成功：{result['worker_id']}")
        return result
    else:
        print(f"❌ 注册失败：{response.text}")
        return None

# 使用示例
gpu_info = {
    "device_type": "cuda",
    "gpu_name": "RTX 3080",
    "memory_gb": 24,
    "compute_power": 68.0
}

register_worker("http://localhost:8080", "my-worker-1", gpu_info)
```

---

## 任务分配机制

### 边缘服务器决策流程

```
1. 接收推理请求
   ↓
2. 算力估算
   - 总算力需求 (GFLOPS)
   - 内存需求 (GB)
   - GPU 需求
   ↓
3. 获取可用节点
   - 在线状态
   - 空闲状态
   - 资源使用情况
   ↓
4. 节点选择算法
   - 资源约束检查
   - 算力排序
   - 选择主节点 + 备份节点
   ↓
5. 模型切分
   - 按层切分
   - 分配策略
   ↓
6. 任务分发
   - Worker A: 层 1-4
   - Worker B: 层 5-8
   - Worker C: 层 9-12
```

### 节点选择算法

```python
# williw-workers/algorithms/node_selection.py

class NodeSelector:
    def select_nodes(self,
                     available_nodes: List,
                     compute_requirement: Dict,
                     num_primary_nodes: int = None) -> Dict:
        """
        选择主节点和备份节点
        
        1. 过滤满足基本约束的节点
           - 在线状态
           - 空闲状态
           - GPU 可用性
           - 资源使用率
        
        2. 按算力排序
        
        3. 选择主节点（算力最强的前 N 个）
        
        4. 选择备份节点
        """
        
        # 1. 过滤
        candidate_nodes = []
        for node in available_nodes:
            is_valid, violations = self.check_resource_constraints(
                node, compute_requirement
            )
            if is_valid:
                compute_power = self.estimate_compute_power(node)
                candidate_nodes.append((node, compute_power))
        
        # 2. 排序
        candidate_nodes.sort(key=lambda x: x[1], reverse=True)
        
        # 3. 选择主节点
        primary_nodes = candidate_nodes[:num_primary_nodes]
        
        # 4. 选择备份节点
        backup_nodes = candidate_nodes[num_primary_nodes:]
        
        return {
            'primary_nodes': primary_nodes,
            'backup_nodes': backup_nodes
        }
```

### 资源约束检查

```python
def check_resource_constraints(self, node, compute_requirement):
    """检查节点是否满足资源约束"""
    
    violations = []
    
    # 检查在线状态
    if not node.is_online:
        violations.append("节点离线")
    
    # 检查空闲状态
    if not node.is_idle:
        violations.append("节点忙碌")
    
    # 检查 GPU 需求
    if compute_requirement['gpu_required']:
        if not node.gpu_available:
            violations.append("无 GPU")
        elif node.gpu_usage > 80.0:
            violations.append(f"GPU 使用率过高 ({node.gpu_usage:.1f}%)")
    
    # 检查 CPU 使用率
    if node.cpu_usage > 85.0:
        violations.append(f"CPU 使用率过高 ({node.cpu_usage:.1f}%)")
    
    # 检查内存使用率
    if node.memory_usage > 80.0:
        violations.append(f"内存使用率过高 ({node.memory_usage:.1f}%)")
    
    # 检查电池电量
    if node.battery_level < 20.0:
        violations.append(f"电池电量过低 ({node.battery_level:.1f}%)")
    
    return len(violations) == 0, violations
```

---

## Workers 管理

### 健康检查

```python
class WorkerManager:
    def __init__(self):
        self.active_workers = {}
        self.heartbeat_interval = 30  # 30 秒
    
    async def health_check_loop(self):
        """定期健康检查"""
        while True:
            await asyncio.sleep(self.heartbeat_interval)
            
            for worker_id, worker in list(self.active_workers.items()):
                if not await self.check_worker_health(worker):
                    # Worker 失联，标记为离线
                    await self.mark_worker_offline(worker_id)
    
    async def check_worker_health(self, worker) -> bool:
        """检查单个 Worker 健康状态"""
        try:
            # 发送心跳请求
            response = await worker.send_heartbeat()
            return response.success
        except:
            return False
```

### 超时处理

```python
class WorkerManager:
    def __init__(self):
        self.task_timeout = 300  # 5 分钟超时
        self.retry_count = 3
    
    async def allocate_task(self, worker, task):
        """分配任务给 Worker（带超时处理）"""
        
        try:
            # 设置超时
            result = await asyncio.wait_for(
                worker.execute(task),
                timeout=self.task_timeout
            )
            
            # 验证结果
            if self.validate_result(result):
                return result
            else:
                raise ValueError("结果验证失败")
        
        except asyncio.TimeoutError:
            # 超时处理：重新分配
            log.warning(f"任务超时，重新分配：{task.id}")
            return await self.reassign_task(task)
        
        except WorkerCrashed:
            # Worker 崩溃：替换 Worker
            log.error(f"Worker 崩溃：{worker.id}")
            return await self.replace_worker(worker, task)
```

### 崩溃恢复

```python
class WorkerManager:
    async def replace_worker(self, failed_worker, task):
        """替换崩溃的 Worker"""
        
        # 1. 从活跃列表移除
        del self.active_workers[failed_worker.id]
        
        # 2. 从备份节点选择新的 Worker
        backup_worker = self.select_backup_worker()
        
        if backup_worker:
            # 3. 重新分配任务
            return await self.allocate_task(backup_worker, task)
        else:
            # 4. 没有可用 Worker，任务失败
            raise RuntimeError("没有可用的 Worker")
    
    def select_backup_worker(self):
        """从备份节点选择 Worker"""
        if self.backup_workers:
            return self.backup_workers.pop(0)
        return None
```

### 结果验证

```python
class ResultValidator:
    def validate_result(self, result) -> bool:
        """验证 Worker 返回的结果"""
        
        # 1. 检查形状
        if not self.check_shape(result):
            log.error("结果形状错误")
            return False
        
        # 2. 检查数据类型
        if not self.check_dtype(result):
            log.error("数据类型错误")
            return False
        
        # 3. 检查 NaN/Inf
        if self.contains_nan_or_inf(result):
            log.error("结果包含 NaN 或 Inf")
            return False
        
        # 4. 检查数值范围
        if not self.check_value_range(result):
            log.error("数值范围异常")
            return False
        
        return True
    
    def check_shape(self, result) -> bool:
        """检查输出形状是否符合预期"""
        expected_shape = self.get_expected_shape()
        return result.shape == expected_shape
    
    def check_dtype(self, result) -> bool:
        """检查数据类型"""
        return result.dtype in [torch.float32, torch.float16]
    
    def contains_nan_or_inf(self, result) -> bool:
        """检查是否包含 NaN 或 Inf"""
        return torch.isnan(result).any() or torch.isinf(result).any()
    
    def check_value_range(self, result) -> bool:
        """检查数值范围"""
        return result.min() > -1e6 and result.max() < 1e6
```

---

## Workers 通信协议

### 任务分配消息

```json
{
    "message_type": "task_assignment",
    "task_id": "task-001",
    "worker_id": "worker-1",
    "model_shard": {
        "layers": [1, 2, 3, 4],
        "state_dict": {...},
        "input_shape": [1, 768]
    },
    "input_data": {...},
    "parameters": {
        "batch_size": 1,
        "device": "cuda:0"
    },
    "timeout": 300
}
```

### 结果返回消息

```json
{
    "message_type": "task_result",
    "task_id": "task-001",
    "worker_id": "worker-1",
    "status": "success",
    "output": {
        "activations": {...},
        "shape": [1, 768],
        "dtype": "float32"
    },
    "performance": {
        "inference_time": 45.2,
        "memory_used": 2.5,
        "gpu_utilization": 85.0
    }
}
```

---

## 实战示例

### 示例 1: 注册单机 Worker

```python
import torch
from williw_workers.interface_layer.app_client import InferenceClient

# 1. 检测本地 GPU
device = "cuda" if torch.cuda.is_available() else "mps" if hasattr(torch.backends, 'mps') and torch.backends.mps.is_available() else "cpu"

print(f"使用设备：{device}")

# 2. 注册 Worker
gpu_info = {
    "device_type": device,
    "gpu_name": torch.cuda.get_device_name(0) if device == "cuda" else "Apple Silicon",
    "memory_gb": torch.cuda.get_device_properties(0).total_memory / 1024**3 if device == "cuda" else 16,
    "compute_power": 68.0 if device == "cuda" else 8.0
}

register_worker("http://localhost:8080", "local-worker", gpu_info)

# 3. 测试推理
client = InferenceClient("http://localhost:8080")
result = client.inference(
    model_name="bert-base-uncased",
    input_data={"text": "Hello world"}
)

print(f"推理完成：{result['status']}")
```

### 示例 2: 多 Workers 协同

```python
# 在多台设备上注册 Workers

# 设备 1 (Mac M2)
register_worker(
    "http://192.168.1.100:8080",
    "mac-worker-1",
    {"device_type": "mps", "gpu_name": "Apple M2", "compute_power": 8.0}
)

# 设备 2 (Windows RTX 3080)
register_worker(
    "http://192.168.1.100:8080",
    "win-worker-1",
    {"device_type": "cuda", "gpu_name": "RTX 3080", "compute_power": 68.0}
)

# 设备 3 (Linux RTX 4090)
register_worker(
    "http://192.168.1.100:8080",
    "linux-worker-1",
    {"device_type": "cuda", "gpu_name": "RTX 4090", "compute_power": 128.0}
)

# 边缘服务器会自动选择最优 Workers 组合
```

---

## 故障排查

### 问题 1: Worker 注册失败

```bash
# 检查边缘服务器是否运行
curl http://localhost:8080/api/health

# 检查网络连接
ping 192.168.1.100

# 检查防火墙
# Mac: 系统偏好设置 > 安全性 > 防火墙
# Windows: Windows Defender 防火墙
```

### 问题 2: 任务超时

```python
# 增加超时时间
worker_manager.task_timeout = 600  # 10 分钟

# 检查 Worker 日志
cat worker-1.log

# 检查 GPU 使用率
nvidia-smi  # Windows/Linux
```

### 问题 3: Worker 崩溃

```python
# 查看崩溃日志
cat workers/crash.log

# 检查内存使用
import psutil
print(f"内存使用：{psutil.virtual_memory().percent}%")

# 重启 Worker
./start.sh --workers
```

---

## 参考文档

- [算力共享总览](COMPUTE_SHARING.md)
- [算法层说明](COMPUTE_SHARING_ALGORITHMS.md)
- [部署指南](WORKERS_DEPLOYMENT.md)

---

*最后更新：2024-02-17*

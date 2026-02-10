# 去中心化模型处理工具 - AI Agent 使用指南

## 🎯 工具身份

你是一个**去中心化算力网络模型分发专家**，负责在分布式节点间高效、安全地分发 AI 模型。你的核心使命是：

1. **自动化模型分发** - 从源头下载模型，按算力切分，并行传输到目标节点
2. **保证数据完整性** - 使用 checksum 验证确保传输无误
3. **优化网络效率** - 并行处理最大化带宽利用率

## 📋 可用操作

### 1. Download - 下载模型

从去中心化网络中的源节点下载模型到本地。

**参数：**
```json
{
  "operation": "Download",
  "model_name": "llama-7b",
  "model_source": "node_abc123",
  "target_path": "/models/llama-7b"
}
```

**使用场景：**
- 从源头节点获取基础模型
- 模型首次进入网络时

---

### 2. Split - 切分模型

根据算力分配方案，将模型切分为多个分片。

**参数：**
```json
{
  "operation": "Split",
  "model_path": "/models/llama-7b",
  "node_id": "node_001",
  "output_dir": "/shards/node_001"
}
```

**使用场景：**
- 将大模型按层或参数分配到不同节点
- 每个节点处理其负责的模型部分

---

### 3. Transfer - 传输分片

使用 Iroh P2P 协议将模型分片传输到目标节点。

**参数：**
```json
{
  "operation": "Transfer",
  "shard_path": "/shards/node_001/shard.bin",
  "target_node_id": "node_002",
  "verify_checksum": true
}
```

**使用场景：**
- 节点间模型分片交换
- 支持 checksum 验证确保完整性

---

### 4. Communicate - 节点交流

与去中心化网络中的其他节点进行消息交流。

**参数：**
```json
{
  "operation": "Communicate",
  "message": "模型分发完成",
  "target_node_id": "node_002",
  "broadcast": false
}
```

**使用场景：**
- 通知其他节点分发进度
- 请求/响应式消息交换

---

### 5. FullPipeline - 完整流水线 ⭐ 推荐

**自动执行**：下载 → 切分 → 传输 → 交流

所有切分和传输任务**并行执行**，最大化效率。

**参数：**
```json
{
  "operation": "FullPipeline",
  "model_name": "llama-7b",
  "model_source": "node_source",
  "output_dir": "/distributed_models",
  "target_nodes": ["node_001", "node_002", "node_003"]
}
```

**典型工作流：**
1. 从 `model_source` 下载模型到 `output_dir`
2. 并行为每个 `target_nodes` 切分模型
3. 并行传输所有分片到对应节点
4. 广播分发完成消息

---

## 🔄 AI Agent 自主决策指南

### 场景1: 首次模型分发

当需要将新模型引入网络时：

```json
{
  "operation": "FullPipeline",
  "model_name": "[模型名称]",
  "model_source": "[源头节点ID]",
  "output_dir": "[本地存储路径]",
  "target_nodes": ["[节点1]", "[节点2]", "..."]
}
```

### 场景2: 增量模型更新

当只需更新部分模型时：

```json
// 1. 先下载更新
{
  "operation": "Download",
  "model_name": "llama-7b-updates",
  "model_source": "node_source",
  "target_path": "/models/updates"
}

// 2. 切分更新部分
{
  "operation": "Split",
  "model_path": "/models/updates",
  "node_id": "node_target",
  "output_dir": "/shards/updates"
}

// 3. 传输到目标节点
{
  "operation": "Transfer",
  "shard_path": "/shards/updates/shard_node_target.bin",
  "target_node_id": "node_target",
  "verify_checksum": true
}
```

### 场景3: 节点间模型交换

当节点需要获取其他节点持有的模型分片时：

```json
{
  "operation": "Communicate",
  "message": "请求模型 shard_003",
  "target_node_id": "node_003",
  "broadcast": false
}
```

---

## ⚡ 性能优化建议

1. **FullPipeline 优先**：复杂任务使用 FullPipeline 自动并行化
2. **Checksum 验证**：生产环境建议 `verify_checksum: true`
3. **批量操作**：多个 Transfer 操作会并行执行

---

## 📊 返回结果格式

所有操作返回标准 ToolResult：

```json
{
  "success": true,
  "data": {
    "operation": "...",
    "result": { ... }
  },
  "execution_time_ms": 100,
  "output": "操作成功"
}
```

---

## 🚨 错误处理

- **缺少参数**: 返回 `InvalidArguments` 错误
- **网络问题**: 返回 `InternalError` 包含详细原因
- **AI 建议**: 查看 `error` 字段获取修复建议

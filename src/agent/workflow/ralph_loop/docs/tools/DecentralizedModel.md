# DecentralizedModel 工具使用指南

## 概述
DecentralizedModel工具是去中心化算力网络的核心组件，用于模型的下载、切分、传输、通信和完整流水线执行。

## 可用操作

### 1. Download - 下载模型

**用途**: 从远程源下载模型到本地

**参数**:
```json
{
  "operation": "Download",
  "model_name": "模型名称",
  "model_source": "模型来源URL或路径",
  "target_path": "本地保存路径"
}
```

**示例**:
```json
{
  "operation": "Download",
  "model_name": "llama-7b",
  "model_source": "https://huggingface.co/models/llama-7b",
  "target_path": "./models/llama-7b.gguf"
}
```

**成功标志**: 返回包含 `local_path` 和 `file_size` 的结果

---

### 2. Split - 切分模型

**用途**: 将模型切分为多个分片

**参数**:
```json
{
  "operation": "Split",
  "model_path": "模型文件路径",
  "node_id": "目标节点ID",
  "output_dir": "输出目录"
}
```

**示例**:
```json
{
  "operation": "Split",
  "model_path": "./models/llama-7b.gguf",
  "node_id": "node_001",
  "output_dir": "./shards/"
}
```

**成功标志**: 返回包含 `shard_path` 的结果

**注意事项**:
- 切分前确保有足够磁盘空间
- 切分后验证分片大小是否合理
- 大模型切分可能需要较长时间

---

### 3. Transfer - 传输分片

**用途**: 将分片传输到目标节点

**参数**:
```json
{
  "operation": "Transfer",
  "shard_path": "分片文件路径",
  "target_node_id": "目标节点ID",
  "verify_checksum": true
}
```

**示例**:
```json
{
  "operation": "Transfer",
  "shard_path": "./shards/shard_node_001.bin",
  "target_node_id": "node_001",
  "verify_checksum": true
}
```

**成功标志**: 返回包含 `verified: true` 的结果

**失败处理**:
- 网络超时：重试传输
- 校验失败：重新传输
- 节点离线：记录并跳过

---

### 4. Communicate - 节点通信

**用途**: 向节点发送消息或广播

**参数**:
```json
{
  "operation": "Communicate",
  "message": "消息内容",
  "target_node_id": "目标节点ID(可选)",
  "broadcast": false
}
```

**示例** (点对点):
```json
{
  "operation": "Communicate",
  "message": "status_request",
  "target_node_id": "node_001",
  "broadcast": false
}
```

**示例** (广播):
```json
{
  "operation": "Communicate",
  "message": "heartbeat",
  "broadcast": true
}
```

---

### 5. FullPipeline - 完整流水线

**用途**: 执行下载-切分-分发的完整流程

**参数**:
```json
{
  "operation": "FullPipeline",
  "model_name": "模型名称",
  "model_source": "模型来源",
  "output_dir": "输出目录",
  "target_nodes": ["node_001", "node_002", "node_003"]
}
```

**示例**:
```json
{
  "operation": "FullPipeline",
  "model_name": "llama-7b",
  "model_source": "./models/llama-7b.gguf",
  "output_dir": "./shards/",
  "target_nodes": ["node_001", "node_002", "node_003", "node_004"]
}
```

**执行流程**:
1. 下载/加载模型
2. 并行切分为N个分片(N=target_nodes数量)
3. 并行分发到所有节点
4. 验证分发结果

**成功标志**: 返回 `{"operation": "full_pipeline", "status": "completed"}`

---

## 最佳实践

### 模型切分策略选择

| 模型类型 | 推荐策略 | 说明 |
|---------|---------|------|
| Transformer | 按层切分 | 每层一个分片，便于推理时逐层传递 |
| CNN | 按卷积块切分 | 每个卷积块独立计算 |
| 大Embedding | 按大小切分 | 均匀分配参数量 |

### 传输优化

1. **压缩传输**: 大分片启用压缩减少网络负载
2. **增量传输**: 只传输变更部分
3. **并行传输**: 多个分片同时传输到不同节点
4. **断点续传**: 大文件支持断点续传

### 错误处理

```rust
// 重试逻辑示例
for attempt in 1..=3 {
    match transfer_shard(shard, node).await {
        Ok(result) => return Ok(result),
        Err(e) if attempt < 3 => {
            println!("传输失败，{}秒后重试...", attempt * 5);
            sleep(Duration::from_secs(attempt * 5)).await;
        }
        Err(e) => return Err(e),
    }
}
```

### 验证清单

每次操作后检查:
- [ ] 文件存在且大小正确
- [ ] 校验和匹配
- [ ] 节点返回确认
- [ ] 网络状态正常

## 常见问题

**Q: 切分后模型还能合并吗？**  
A: 可以，使用相同的切分策略逆序合并。

**Q: 节点离线怎么办？**  
A: 重试3次后标记为不可用，将分片分配到备用节点。

**Q: 如何监控传输进度？**  
A: 使用Communicate操作轮询节点状态，或订阅进度事件。
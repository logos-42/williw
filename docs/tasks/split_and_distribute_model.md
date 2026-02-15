# 任务：切分AI模型并分发到多节点

## 目标
将已下载的AI模型切分为多个分片，并行分发到去中心化网络中的多个算力节点。

## 描述
这是一个模型切分分发任务。模型已经下载到本地，需要：
1. 分析模型结构
2. 将模型切分为N个分片（N=目标节点数量）
3. 计算每个分片的校验和
4. 将分片传输到对应节点
5. 验证传输完整性

## 输入参数
- **模型路径**: `./models/llama-2-7b/` (模型目录或文件)
- **目标节点数**: 3 或更多
- **目标节点ID**: node_001, node_002, node_003
- **输出目录**: `./shards/llama-2-7b/`

## 验收标准（必须全部达成）
- [ ] 模型文件分析完成
- [ ] 模型被成功切分为N个分片
- [ ] 所有分片生成SHA256校验和
- [ ] N个分片分别成功传输到对应节点
- [ ] 每个节点验证接收的分片校验和正确
- [ ] 生成分发报告

## 执行步骤

### 步骤1: 分析模型结构
- **操作**: 使用BashTool检查模型文件
- **验证**: 获取到模型的文件列表、总大小
- **命令**:
```json
{
  "operation": "Execute",
  "shell": "bash",
  "command": "ls -la ./models/llama-2-7b/"
}
```

### 步骤2: 切分模型（AI自主执行）
- **操作**: 调用DecentralizedModel::Split获取切分命令
- **工具**: DecentralizedModel工具
- **参数**:
```json
{
  "operation": "Split",
  "model_path": "./models/llama-2-7b/pytorch_model.bin",
  "node_id": "node_001",
  "output_dir": "./shards/llama-2-7b"
}
```
- **响应**: 返回AI可执行的Python脚本命令

### 步骤3: AI执行切分
- **操作**: 使用BashTool执行返回的Python脚本
- **工具**: BashTool
- **执行**: 解析上一步返回的 `ai_execution.command`，使用Python执行
- **验证**: 确认分片文件创建成功

### 步骤4: 为其他节点切分
- **操作**: 重复步骤2-3，为每个节点创建分片
- **建议**: 可以并行执行

### 步骤5: 计算校验和
- **操作**: 为每个分片计算SHA256
- **工具**: BashTool
```json
{
  "operation": "Execute",
  "shell": "bash",
  "command": "sha256sum ./shards/llama-2-7b/shard_*.bin > ./shards/llama-2-7b/checksums.txt"
}
```

### 步骤6: 传输分片到节点
- **操作**: 使用DecentralizedModel::Transfer传输
- **工具**: DecentralizedModel工具
- **参数**:
```json
{
  "operation": "Transfer",
  "shard_path": "./shards/llama-2-7b/shard_node_001.bin",
  "target_node_id": "node_001",
  "verify_checksum": true
}
```

### 步骤7: 验证完整性
- **操作**: 确认每个节点接收的分片校验和匹配

## 约束条件
- **最大执行时间**: 30分钟
- **单分片最大大小**: 根据模型大小
- **网络要求**: 稳定的P2P连接

## 故障处理
1. **模型文件不存在**: 检查路径是否正确
2. **切分失败**: 检查Python环境和磁盘空间
3. **传输失败**: 检查节点连接状态，重试3次
4. **校验失败**: 重新传输该分片

## 完成信号
任务完成时，应该：
1. `./shards/llama-2-7b/shard_node_001.bin` 等分片文件存在
2. `checksums.txt` 校验和文件存在
3. 所有节点确认接收

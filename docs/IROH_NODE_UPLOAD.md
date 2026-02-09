# Iroh节点信息自动上传功能

## 功能说明

系统现在支持每30秒自动将iroh节点信息和设备信息上传到Workers后端。

## 上传的数据内容

### 1. 设备信息 (DeviceInfo)
- GPU类型
- GPU使用率
- GPU显存总量/使用
- CPU核心数
- 总内存

### 2. Iroh节点信息 (IrohNodeInfo)
- 节点ID
- 运行状态
- 计数器
- 设备能力（内存、CPU、GPU等）
- 训练统计（准确率、损失、样本数）
- 连接的 peers（主节点和备份节点）

### 3. 元数据 (Metadata)
- 平台信息
- 应用版本
- 系统能力

## 后端API端点

```
POST /api/node-info
```

请求体格式：
```json
{
  "device_id": "uuid-string",
  "node_id": "uuid-string",
  "timestamp": "2026-02-02T07:19:35Z",
  "device_info": { ... },
  "iroh_node": {
    "node_id": "iroh-node-id",
    "is_running": true,
    "tick_counter": 1234,
    "device_capabilities": { ... },
    "training_stats": { ... },
    "peers": [ ... ]
  },
  "metadata": { ... }
}
```

## 自动上传机制

- **间隔**: 每30秒
- **触发条件**: 无论iroh节点是否运行都会上传
- **日志**: 上传成功/失败会记录在日志中

## 手动上传命令

前端可以通过以下命令手动触发上传：

```javascript
import { invoke } from '@tauri-apps/api/core';

// 手动上传完整节点信息
const result = await invoke('upload_full_node_info_to_workers');
console.log(result);
```

## 后端存储格式

Workers后端会将接收到的信息存储在KV中，键名为：
- `node_info:{node_id}` - 存储节点完整信息
- `node_heartbeat:{node_id}` - 存储心跳时间

## 故障排查

### 查看上传日志

在应用日志中搜索 `[AutoUpload]` 标签：

```
[AutoUpload] Starting automatic node info upload task (every 30s)
[AutoUpload] Node info uploaded successfully
[AutoUpload] Upload failed: ...
[AutoUpload] Upload error: ...
```

### 常见问题

1. **上传失败**
   - 检查VPN连接
   - 检查Workers后端是否运行
   - 查看日志中的错误信息

2. **iroh_node为空**
   - 确保已调用 `start_training` 启动节点
   - 未启动节点时只会上传设备信息

3. **数据不完整**
   - 检查设备信息是否正确获取
   - 检查iroh节点状态

## 配置文件

上传间隔可以在 `main.rs` 中修改：

```rust
let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
```

## 代码结构

```
src-tauri/src/
├── api_client.rs      # API客户端，包含上传方法
├── commands.rs        # 命令定义，包含手动上传命令
├── main.rs           # 主程序，包含自动上传任务
└── state.rs          # 状态管理
```

## 新增的数据结构

### IrohNodeInfo
```rust
pub struct IrohNodeInfo {
    pub node_id: String,
    pub is_running: bool,
    pub tick_counter: u64,
    pub device_capabilities: IrohDeviceCapabilities,
    pub training_stats: IrohTrainingStats,
    pub peers: Vec<IrohPeerInfo>,
}
```

### FullNodeInfoPayload
```rust
pub struct FullNodeInfoPayload {
    pub device_id: String,
    pub node_id: String,
    pub timestamp: String,
    pub iroh_node: Option<IrohNodeInfo>,
    pub device_info: DeviceInfo,
    pub metadata: DeviceMetadata,
}
```

# iroh + Workers 集成使用指南

> **真正的分布式算力共享** - 代码已更新完成

---

## ✅ 已完成的更新

### Rust 代码更新

| 文件 | 更新内容 | 状态 |
|------|---------|------|
| `src-tauri/src/api_client.rs` | 添加 `register_iroh_node()`, `get_nodes()` | ✅ 完成 |
| `src-tauri/src/commands/workers_commands.rs` | 添加 `register_iroh_node_to_workers()`, `get_available_nodes_from_workers()` | ✅ 完成 |
| `src-tauri/src/main.rs` | 注册新的 Tauri commands | ✅ 完成 |

### Python 代码更新

| 文件 | 更新内容 | 状态 |
|------|---------|------|
| `williw-workers/interface_layer/node_info_api_updated.py` | 支持 iroh 节点注册 | ✅ 完成 |
| `williw-workers/edge_server/api_server_updated.py` | 添加 iroh 节点注册端点 | ✅ 完成 |

---

## 🚀 使用方式

### Step 1: 启动边缘服务器

```bash
cd williw-workers

# 使用更新后的 API 服务器
python -m edge_server.api_server_updated --port 8080
```

**预期输出:**
```
启动边缘服务器：http://0.0.0.0:8080
iroh 节点注册端点：http://0.0.0.0:8080/api/iroh-node/register
```

---

### Step 2: 在 Tauri App 中注册节点

#### 前端调用（TypeScript）

```typescript
// 在 Tauri 前端调用
import { invoke } from '@tauri-apps/api/core';

// 注册 iroh 节点到 Workers
async function registerNode() {
  try {
    const result = await invoke('register_iroh_node_to_workers');
    console.log('注册成功:', result);
    // 输出：✅ iroh 节点注册成功：iroh-node-xxx
  } catch (error) {
    console.error('注册失败:', error);
  }
}

// 获取所有可用节点
async function getAvailableNodes() {
  try {
    const result = await invoke('get_available_nodes_from_workers');
    console.log('可用节点:', result);
    /* 输出:
    {
      "success": true,
      "nodes": [
        {
          "node_id": "iroh-node-a",
          "endpoint": "http://192.168.1.100:8080",
          "max_memory_gb": 32,
          "gpu_type": "cuda",
          "gpu_memory_gb": 24,
          "cpu_cores": 8,
          "current_load": 0.3,
          "reliability": 0.95
        }
      ],
      "total": 1
    }
    */
  } catch (error) {
    console.error('获取节点失败:', error);
  }
}
```

#### Rust 调用

```rust
// 在 Rust 代码中调用
use tauri::Manager;

// 注册节点
let result = app_handle
    .invoke_command::<String, _>("register_iroh_node_to_workers", ())
    .await?;

println!("注册成功：{}", result);

// 获取节点列表
let nodes = app_handle
    .invoke_command::<serde_json::Value, _>("get_available_nodes_from_workers", ())
    .await?;

println!("可用节点：{}", nodes);
```

---

### Step 3: 发起推理请求

```typescript
// 1. 先获取可用节点
const nodes = await invoke('get_available_nodes_from_workers');

// 2. 选择最优节点（或使用 Workers 的自动选择）
// Workers 会自动选择算力最强的节点组合

// 3. 发起推理请求
const result = await invoke('request_inference_from_workers', {
  modelId: 'bert-base-uncased',
  inputData: { text: 'Hello world' }
});

// 4. 使用 iroh 连接到选中的节点
const selectedNodes = result.selected_nodes;
for (const node of selectedNodes) {
  await invoke('connect_to_node', {
    nodeId: node.node_id,
    addresses: node.addresses
  });
}
```

---

## 📋 完整流程示例

### 单机测试

```bash
# 1. 启动边缘服务器
cd williw-workers
python -m edge_server.api_server_updated --port 8080

# 2. 启动 Tauri App
cd src-tauri
npm run tauri dev

# 3. 在前端界面点击"注册节点"
# 调用 register_iroh_node_to_workers()

# 4. 查看日志
# Rust 节点日志：
# ✅ iroh 节点注册成功：iroh-node-xxx

# Python 边缘服务器日志：
# ✅ iroh 节点注册成功：iroh-node-xxx
```

---

### 多机测试

```bash
# 电脑 A (Mac M2) - 边缘服务器 + 节点
# 1. 启动边缘服务器
python -m edge_server.api_server_updated --port 8080 --host 0.0.0.0

# 2. 启动 Tauri App
npm run tauri dev

# 3. 注册节点
# 前端调用 register_iroh_node_to_workers()

# 电脑 B (Windows RTX 3080) - 节点
# 1. 设置环境变量
export WILLIW_WORKERS_URL=http://192.168.1.100:8080

# 2. 启动 Tauri App
npm run tauri dev

# 3. 注册节点
# 前端调用 register_iroh_node_to_workers()

# 电脑 C (Linux RTX 4090) - 节点
# 同上...

# 在任一电脑上查看可用节点
const nodes = await invoke('get_available_nodes_from_workers');
console.log('可用节点数:', nodes.total);
// 输出：可用节点数：3
```

---

## 🔧 API 参考

### POST /api/iroh-node/register

注册 iroh 节点

**请求体:**
```json
{
    "node_id": "iroh-node-xxx",
    "endpoint": "http://192.168.1.100:8080",
    "device_info": {
        "gpu_type": "cuda",
        "gpu_memory_total": 24,
        "cpu_cores": 8,
        "max_memory_mb": 32768,
        "network_type": "WiFi",
        "battery_level": 0.85,
        "is_charging": false
    },
    "iroh_node": {
        "node_id": "iroh-node-xxx",
        "addresses": []
    }
}
```

**响应:**
```json
{
    "status": "success",
    "message": "iroh 节点注册成功",
    "node_id": "iroh-node-xxx"
}
```

---

### GET /api/nodes

获取所有可用节点

**响应:**
```json
{
    "status": "success",
    "nodes": [
        {
            "node_id": "iroh-node-a",
            "endpoint": "http://192.168.1.100:8080",
            "max_memory_gb": 32,
            "gpu_type": "cuda",
            "gpu_memory_gb": 24,
            "cpu_cores": 8,
            "current_load": 0.3,
            "reliability": 0.95
        }
    ],
    "total": 1
}
```

---

## 🐛 故障排查

### 问题 1: 注册失败

```bash
# 检查边缘服务器是否运行
curl http://localhost:8080/api/health

# 检查日志
tail -f workers.log

# 检查网络连接
ping 192.168.1.100
```

### 问题 2: 节点列表为空

```bash
# 确认已注册节点
curl http://localhost:8080/api/nodes

# 如果没有节点，先注册
# 在 Tauri App 中调用 register_iroh_node_to_workers()
```

### 问题 3: iroh 连接失败

```rust
// 检查 iroh 节点是否运行
let node_id = comms.node_id();
println!("本地节点 ID: {}", node_id);

// 测试连接
match comms.connect(target_node_id).await {
    Ok(_) => println!("连接成功"),
    Err(e) => println!("连接失败：{}", e),
}
```

---

## 📖 参考文档

- [iroh + Workers 集成指南](docs/IROH_WORKERS_INTEGRATION.md)
- [算力共享总览](docs/COMPUTE_SHARING.md)
- [Workers 机制](docs/COMPUTE_SHARING_WORKERS.md)

---

*最后更新：2024-02-17*
*代码更新完成*

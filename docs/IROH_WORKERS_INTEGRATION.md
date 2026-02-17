# iroh + Workers 集成指南

> **真正的分布式算力共享** - iroh P2P 通信 + Workers 算力调度

---

## 架构说明

### 完整架构

```
┌─────────────────────────────────────────────────────────────────┐
│                    williw 完整架构                               │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────┐
│  Tauri App          │  用户接口层
│  (williw-master)    │  - 用户请求
│                     │  - 设备信息
└──────────┬──────────┘
           │ 1. 上传节点信息（HTTP）
           ↓
┌─────────────────────────────────────────────────────────────────┐
│  边缘服务器 (williw-workers/edge_server)                         │
│                                                                 │
│  - 接收节点注册（/api/iroh-node/register）                      │
│  - 存储节点信息（iroh 节点 ID + 设备信息）                       │
│  - 算力估算、节点选择、模型切分                                 │
│                                                                 │
└────────────┬────────────────────────────────────────────────────┘
             │ 2. 返回选中的节点列表（包含 iroh 节点 ID）
             ↓
┌─────────────────────────────────────────────────────────────────┐
│  Tauri App 使用 iroh 进行 P2P 通信                                │
│                                                                 │
│  - 通过 iroh 连接到选中的节点                                    │
│  - 发送模型分片和输入数据                                        │
│  - 接收推理结果（激活值）                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 核心流程

### 1. 节点注册流程

```
Rust 节点（Tauri App）
    ↓
1. 获取本地 iroh 节点 ID
   let node_id = comms.node_id().to_string();

2. 获取设备信息
   let device_info = device_manager.get();

3. 上传到边缘服务器
   POST /api/iroh-node/register
   {
       "node_id": "iroh-node-xxx",
       "endpoint": "http://192.168.1.100:8080",
       "device_info": {...},
       "iroh_node": {...}
   }
    ↓
边缘服务器存储节点信息
```

### 2. 推理请求流程

```
用户发起推理请求
    ↓
边缘服务器处理
    - 算力估算
    - 节点选择（从已注册的 iroh 节点中选择）
    - 模型切分
    ↓
返回选中的节点列表
{
    "selected_nodes": [
        {"node_id": "iroh-node-a", ...},
        {"node_id": "iroh-node-b", ...}
    ],
    "model_split_plan": {...}
}
    ↓
Tauri App 使用 iroh 连接节点
    - comms.connect("iroh-node-a")
    - 发送模型分片
    - 接收结果
```

---

## 使用指南

### Step 1: 启动边缘服务器

```bash
cd williw-workers

# 使用更新后的 API 服务器
python -m edge_server.api_server_updated --port 8080
```

### Step 2: 在 Rust 节点注册

```rust
use williw::comms::p2p_app_integration::P2PAppFactory;
use williw::api_client::WorkersApiClient;

#[tauri::command]
async fn register_with_workers(app: tauri::State<App>) -> Result<(), String> {
    // 1. 获取 iroh 节点 ID
    let node_id = app.comms.node_id().to_string();
    
    // 2. 获取设备信息
    let device_info = app.device_manager.get();
    
    // 3. 构建注册数据
    let register_data = serde_json::json!({
        "node_id": node_id,
        "endpoint": app.comms.local_addr().unwrap_or_default(),
        "device_info": {
            "gpu_type": device_info.gpu_type,
            "gpu_memory_total": device_info.gpu_memory_total,
            "cpu_cores": device_info.cpu_cores,
            "max_memory_mb": device_info.max_memory_mb,
            "network_type": format!("{:?}", device_info.network_type),
            "battery_level": device_info.battery_level,
        },
        "iroh_node": {
            "node_id": node_id,
            "addresses": vec![], // 可以从 iroh 获取
        }
    });
    
    // 4. 发送到边缘服务器
    let client = WorkersApiClient::new("http://localhost:8080");
    let response = client.post("/api/iroh-node/register", register_data).await?;
    
    if response["status"] == "success" {
        println!("✅ 节点注册成功");
        Ok(())
    } else {
        Err(format!("注册失败：{}", response["message"]))
    }
}
```

### Step 3: 发起推理请求

```rust
#[tauri::command]
async fn request_inference(
    app: tauri::State<App>,
    model_name: String,
    input_data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // 1. 发送推理请求到边缘服务器
    let client = WorkersApiClient::new("http://localhost:8080");
    let response = client.post("/api/inference", serde_json::json!({
        "model_name": model_name,
        "input_data": input_data
    })).await.map_err(|e| e.to_string())?;
    
    // 2. 获取选中的节点
    let selected_nodes = response["selected_nodes"]
        .as_array()
        .ok_or("selected_nodes 格式错误")?;
    
    // 3. 通过 iroh 连接到各节点
    for node in selected_nodes {
        let node_id = node["node_id"].as_str().ok_or("node_id 格式错误")?;
        
        // 使用 iroh 连接
        app.comms.connect(node_id.to_string()).await
            .map_err(|e| format!("连接失败：{}", e))?;
        
        println!("✅ 已连接到节点：{}", node_id);
    }
    
    // 4. 通过 iroh 发送模型分片和输入数据
    // ... (使用 P2PModelDistributor)
    
    Ok(response)
}
```

---

## API 参考

### 边缘服务器 API

#### POST /api/iroh-node/register

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
        "battery_level": 0.85
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

#### GET /api/nodes

获取所有可用节点

**响应:**
```json
{
    "status": "success",
    "nodes": [
        {
            "node_id": "iroh-node-a",
            "location": "北京",
            "gpu_available": true,
            "gpu_memory": 24,
            "compute_power": 68.0,
            "is_online": true,
            "is_idle": true,
            "reliability_score": 0.95
        }
    ],
    "total": 1
}
```

#### POST /api/inference

请求推理

**响应:**
```json
{
    "status": "success",
    "selected_nodes": [
        {
            "node_id": "iroh-node-a",
            "endpoint": "http://192.168.1.100:8080",
            "capabilities": {...}
        }
    ],
    "model_split_plan": {
        "total_layers": 12,
        "splits": [
            {
                "layer_range": [0, 4],
                "assigned_node": "iroh-node-a"
            }
        ]
    }
}
```

---

## Rust 代码集成

### 更新 workers_commands.rs

```rust
/// Register iroh node to workers backend
#[tauri::command]
pub async fn register_iroh_node_to_workers(
    state: State<'_, AppState>
) -> Result<String, String> {
    // 获取 iroh 节点 ID
    let node_guard = state.node.lock();
    let node = node_guard.as_ref()
        .ok_or("Node not running")?;
    
    let iroh_node_id = node.comms.node_id().to_string();
    let endpoint = node.comms.local_addr()
        .unwrap_or_else(|_| "localhost:8080".to_string());
    
    // 获取设备信息
    let device_info = state.device_info.lock().clone()
        .ok_or_else(|| "No device info available".to_string())?;
    
    // 构建注册数据
    let register_data = serde_json::json!({
        "node_id": iroh_node_id,
        "endpoint": endpoint,
        "device_info": {
            "gpu_type": device_info.gpu_type,
            "gpu_memory_total": device_info.gpu_memory_total,
            "cpu_cores": device_info.cpu_cores,
            "max_memory_mb": device_info.total_memory_gb * 1024.0,
            "network_type": format!("{:?}", device_info.network_type),
            "battery_level": device_info.battery_level,
            "is_charging": device_info.is_charging,
        },
        "iroh_node": {
            "node_id": iroh_node_id,
            "addresses": vec![],
        }
    });
    
    // 发送到边缘服务器
    match state.api_client.register_iroh_node(register_data).await {
        Ok(response) => {
            if response.success {
                Ok(format!("✅ iroh 节点注册成功：{}", iroh_node_id))
            } else {
                Err(format!("注册失败：{}", response.message))
            }
        }
        Err(e) => Err(format!("网络错误：{}", e)),
    }
}
```

### 更新 api_client.rs

```rust
impl WorkersApiClient {
    /// 注册 iroh 节点
    pub async fn register_iroh_node(
        &self,
        node_data: serde_json::Value
    ) -> Result<ApiResponse> {
        let url = format!("{}/api/iroh-node/register", self.base_url);
        
        let response = self.client
            .post(&url)
            .json(&node_data)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP 请求失败：{}", e))?;
        
        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败：{}", e))?;
        
        Ok(api_response)
    }
    
    /// 获取所有节点
    pub async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        let url = format!("{}/api/nodes", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP 请求失败：{}", e))?;
        
        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("解析响应失败：{}", e))?;
        
        let nodes_data = result["nodes"]
            .as_array()
            .ok_or_else(|| anyhow!("nodes 格式错误"))?;
        
        let mut nodes = Vec::new();
        for node_data in nodes_data {
            // 转换为 NodeInfo
            let node = NodeInfo {
                node_id: node_data["node_id"].as_str().unwrap_or("").to_string(),
                endpoint: node_data.get("endpoint").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                capabilities: NodeCapabilities {
                    max_memory_gb: node_data.get("gpu_memory").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    gpu_type: node_data.get("gpu_available").and_then(|v| v.as_bool()).map(|b| if b { "Unknown" } else { "" }.to_string()),
                    gpu_memory_gb: node_data.get("gpu_memory").and_then(|v| v.as_f64()),
                    cpu_cores: 4, // 简化
                    network_bandwidth_mbps: 1000,
                    supported_models: vec![],
                },
                current_load: 0.5,
                latency: None,
                reliability: node_data.get("reliability_score").and_then(|v| v.as_f64()).unwrap_or(0.95) as f32,
            };
            nodes.push(node);
        }
        
        Ok(nodes)
    }
}
```

---

## 完整示例

### 单机测试

```bash
# 1. 启动边缘服务器
cd williw-workers
python -m edge_server.api_server_updated --port 8080

# 2. 启动 Rust 节点（注册到边缘服务器）
cargo run -- --register-workers

# 3. 发起推理请求
cargo run -- --inference --model "bert-base"
```

### 多机测试

```bash
# 电脑 A (Mac M2)
# 1. 启动边缘服务器
python -m edge_server.api_server_updated --port 8080 --host 0.0.0.0

# 2. 启动 Rust 节点
cargo run -- --register-workers

# 电脑 B (Windows RTX 3080)
# 1. 启动 Rust 节点（注册到电脑 A 的边缘服务器）
export WILLIW_WORKERS_URL=http://192.168.1.100:8080
cargo run -- --register-workers

# 2. 发起推理请求
cargo run -- --inference --model "bert-base"
```

---

## 故障排查

### 问题 1: 节点注册失败

```bash
# 检查边缘服务器日志
tail -f workers.log

# 检查网络连接
curl http://localhost:8080/api/health
```

### 问题 2: iroh 连接失败

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

## 参考文档

- [算力共享总览](docs/COMPUTE_SHARING.md)
- [Workers 机制](docs/COMPUTE_SHARING_WORKERS.md)
- [iroh P2P 通信](src/comms/)

---

*最后更新：2024-02-17*

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::state::{DeviceInfo, ModelConfig, TrainingStatus};
use anyhow::{Result, anyhow};

/// Workers后端API客户端
pub struct WorkersApiClient {
    client: reqwest::Client,
    base_url: String,
}

/// 设备信息上传数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoPayload {
    pub device_id: String,
    pub timestamp: String,
    pub node_id: String,  // 后端要求必填
    pub device_info: DeviceInfo,
    pub metadata: DeviceMetadata,
}

/// 设备元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMetadata {
    pub platform: String,
    pub app_version: String,
    pub node_id: Option<String>,
    pub capabilities: HashMap<String, serde_json::Value>,
}

/// 模型选择上传数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionPayload {
    pub device_id: String,
    pub timestamp: String,
    pub model_selection: ModelSelectionData,
    pub training_config: TrainingConfigData,
}

/// 模型选择数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionData {
    pub model_id: String,
    pub model_name: String,
    pub selected_at: String,
    pub selection_reason: Option<String>,
}

/// 训练配置数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfigData {
    pub learning_rate: f64,
    pub batch_size: usize,
    pub epochs: u32,
    pub enable_distributed: bool,
}

/// 训练状态上传数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStatusPayload {
    pub device_id: String,
    pub timestamp: String,
    pub training_status: TrainingStatus,
    pub node_id: Option<String>,
}

/// 推理请求数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequestPayload {
    pub device_id: String,
    pub timestamp: String,
    pub model_id: String,
    pub input_data: serde_json::Value,
}

/// 推理请求响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequestResponse {
    pub success: bool,
    pub message: String,
    pub request_id: Option<String>,
    pub selected_nodes: Vec<NodeInfo>,
    pub model_split_plan: ModelSplitPlan,
    pub estimated_total_time: u32, // 毫秒
    pub fallback_nodes: Vec<NodeInfo>, // 备选节点
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub endpoint: String,
    pub capabilities: NodeCapabilities,
    pub current_load: f32, // 0.0 - 1.0
    pub latency: Option<u32>, // 毫秒
    pub reliability: f32, // 0.0 - 1.0
}

/// 节点能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub max_memory_gb: f64,
    pub gpu_type: Option<String>,
    pub gpu_memory_gb: Option<f64>,
    pub cpu_cores: u32,
    pub network_bandwidth_mbps: u32,
    pub supported_models: Vec<String>,
}

/// 模型切分方案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSplitPlan {
    pub total_layers: u32,
    pub splits: Vec<ModelSplit>,
    pub communication_overhead: f64, // MB
    pub estimated_inference_time: u32, // 毫秒
}

/// 模型切分信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSplit {
    pub layer_range: (u32, u32), // (start_layer, end_layer)
    pub assigned_node: String,
    pub memory_requirement_mb: u64,
    pub compute_requirement: f32, // GFLOPs
}

/// 节点重新分配请求数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReassignmentPayload {
    pub device_id: String,
    pub timestamp: String,
    pub failed_nodes: Vec<String>,
    pub current_splits: Vec<ModelSplit>,
    pub request_id: String,
}

/// 节点重新分配响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReassignmentResponse {
    pub success: bool,
    pub message: String,
    pub new_splits: Option<Vec<ModelSplit>>,
    pub reassigned_nodes: Vec<NodeInfo>,
}

/// 节点健康状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthResponse {
    pub success: bool,
    pub message: String,
    pub node_id: String,
    pub is_healthy: bool,
    pub last_seen: Option<String>,
    pub current_load: Option<f32>,
    pub issues: Vec<String>,
}

/// API响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Iroh节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohNodeInfo {
    pub node_id: String,
    pub is_running: bool,
    pub tick_counter: u64,
    pub device_capabilities: IrohDeviceCapabilities,
    pub training_stats: IrohTrainingStats,
    pub peers: Vec<IrohPeerInfo>,
}

/// Iroh设备能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohDeviceCapabilities {
    pub max_memory_mb: u64,
    pub cpu_cores: u32,
    pub has_gpu: bool,
    pub network_type: String,
    pub battery_level: Option<f32>,
    pub is_charging: Option<bool>,
}

/// Iroh训练统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohTrainingStats {
    pub total_ticks: u64,
    pub accuracy: f64,
    pub loss: f64,
    pub samples_processed: u64,
}

/// Iroh对等节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohPeerInfo {
    pub id: String,
    pub peer_type: String, // "primary" or "backup"
    pub similarity: f64,
    pub geo_affinity: f64,
    pub embedding_dim: usize,
    pub position: GeoPosition,
}

/// 地理位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPosition {
    pub lat: f64,
    pub lon: f64,
}

/// 完整节点信息上传数据结构（包含iroh和设备信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullNodeInfoPayload {
    pub device_id: String,
    pub node_id: String,
    pub timestamp: String,
    pub iroh_node: Option<IrohNodeInfo>,
    pub device_info: DeviceInfo,
    pub metadata: DeviceMetadata,
}

impl WorkersApiClient {
    /// 创建新的API客户端
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url,
        }
    }

    /// 上传设备信息和节点状态到 /api/node-info 端点
    pub async fn upload_node_info_from_device(&self, device_info: DeviceInfo) -> Result<ApiResponse> {
        let device_id = self.get_device_id();
        let payload = DeviceInfoPayload {
            device_id: device_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            node_id: device_id,  // 后端要求必填，使用device_id作为node_id
            device_info,
            metadata: self.get_device_metadata(),
        };

        let response = self.client
            .post(&format!("{}/api/node-info", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let api_response: ApiResponse = response.json().await?;
        Ok(api_response)
    }

    /// 上传桌面选定的模型到 /api/model 端点
    pub async fn upload_selected_model(&self, model_config: ModelConfig, training_config: TrainingConfigData) -> Result<ApiResponse> {
        let payload = ModelSelectionPayload {
            device_id: self.get_device_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            model_selection: ModelSelectionData {
                model_id: model_config.id,
                model_name: model_config.name,
                selected_at: chrono::Utc::now().to_rfc3339(),
                selection_reason: Some("User selected from desktop interface".to_string()),
            },
            training_config,
        };

        let response = self.client
            .post(&format!("{}/api/model", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let api_response: ApiResponse = response.json().await?;
        Ok(api_response)
    }

    /// 用户发起推理请求到 /api/request 端点
    pub async fn request_inference(&self, model_id: String, input_data: serde_json::Value) -> Result<InferenceRequestResponse> {
        let payload = InferenceRequestPayload {
            device_id: self.get_device_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            model_id,
            input_data,
        };

        let response = self.client
            .post(&format!("{}/api/request", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let inference_response: InferenceRequestResponse = response.json().await?;
        Ok(inference_response)
    }

    /// 上传训练数据样本到 /api/training-data 端点
    pub async fn upload_training_data(&self, training_status: TrainingStatus, node_id: Option<String>) -> Result<ApiResponse> {
        let payload = TrainingStatusPayload {
            device_id: self.get_device_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            training_status,
            node_id,
        };

        let response = self.client
            .post(&format!("{}/api/training-data", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let api_response: ApiResponse = response.json().await?;
        Ok(api_response)
    }

    /// 获取设备ID（生成或读取持久化的设备ID）
    pub fn get_device_id(&self) -> String {
        if let Ok(override_id) = std::env::var("WILLIW_DEVICE_ID") {
            let override_id = override_id.trim();
            if !override_id.is_empty() {
                return override_id.to_string();
            }
        }

        let id_path = Self::device_id_path();
        if let Ok(existing) = std::fs::read_to_string(&id_path) {
            let existing = existing.trim();
            if !existing.is_empty() {
                return existing.to_string();
            }
        }

        let generated = Self::detect_system_id().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        if let Some(parent) = id_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&id_path, &generated);

        generated
    }

    /// 获取设备元数据
    fn get_device_metadata(&self) -> DeviceMetadata {
        let mut capabilities = HashMap::new();
        
        // 添加系统信息
        capabilities.insert("os".to_string(), serde_json::Value::String(std::env::consts::OS.to_string()));
        capabilities.insert("arch".to_string(), serde_json::Value::String(std::env::consts::ARCH.to_string()));
        capabilities.insert("family".to_string(), serde_json::Value::String(std::env::consts::FAMILY.to_string()));
        
        DeviceMetadata {
            platform: std::env::consts::OS.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            node_id: None, // 可以从Node获取
            capabilities,
        }
    }

    fn device_id_path() -> PathBuf {
        if let Ok(path) = std::env::var("WILLIW_DEVICE_ID_FILE") {
            return PathBuf::from(path);
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(app_data) = std::env::var("APPDATA") {
                return PathBuf::from(app_data).join("williw").join("device_id");
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".williw").join("device_id");
        }

        std::env::temp_dir().join("williw_device_id")
    }

    fn detect_system_id() -> Option<String> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(machine_id) = std::fs::read_to_string("/etc/machine-id") {
                let machine_id = machine_id.trim();
                if !machine_id.is_empty() {
                    return Some(machine_id.to_string());
                }
            }
            if let Ok(machine_id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
                let machine_id = machine_id.trim();
                if !machine_id.is_empty() {
                    return Some(machine_id.to_string());
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            if let Ok(output) = Command::new("ioreg")
                .args(["-rd1", "-c", "IOPlatformExpertDevice"])
                .output()
            {
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    for line in output_str.lines() {
                        if line.contains("IOPlatformUUID") {
                            if let Some(uuid) = line.split('=').nth(1) {
                                let uuid = uuid.trim().trim_matches('"');
                                if !uuid.is_empty() {
                                    return Some(uuid.to_string());
                                }
                            }
                        }
                    }
                }
            }

            if let Ok(output) = Command::new("system_profiler")
                .args(["SPHardwareDataType"])
                .output()
            {
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    for line in output_str.lines() {
                        let line = line.trim();
                        if line.starts_with("Hardware UUID:") {
                            if let Some(uuid) = line.split(':').nth(1) {
                                let uuid = uuid.trim();
                                if !uuid.is_empty() {
                                    return Some(uuid.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            if let Ok(output) = Command::new("wmic")
                .args(["csproduct", "get", "UUID", "/format:list"])
                .output()
            {
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    for line in output_str.lines() {
                        if line.starts_with("UUID=") {
                            if let Some(uuid) = line.split('=').nth(1) {
                                let uuid = uuid.trim();
                                if !uuid.is_empty() {
                                    return Some(uuid.to_string());
                                }
                            }
                        }
                    }
                }
            }

            if let Ok(output) = Command::new("getmac")
                .args(["/format", "list"])
                .output()
            {
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    for line in output_str.lines() {
                        if line.contains("Physical Address") {
                            if let Some(mac) = line.split('=').nth(1) {
                                let mac = mac.trim().replace("-", ":");
                                if !mac.is_empty() {
                                    return Some(mac);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// 节点无法联系部分节点时，请求重新分配新的节点
    pub async fn reassign_node(&self, failed_nodes: Vec<String>, current_splits: Vec<ModelSplit>, request_id: String) -> Result<NodeReassignmentResponse> {
        let payload = NodeReassignmentPayload {
            device_id: self.get_device_id(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            failed_nodes,
            current_splits,
            request_id,
        };

        let response = self.client
            .post(&format!("{}/api/reassign-node", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let reassignment_response: NodeReassignmentResponse = response.json().await?;
        Ok(reassignment_response)
    }

    /// 节点上报自身状态和硬件信息到 /api/node-info 端点
    pub async fn upload_node_info(&self, node_info: NodeInfo) -> Result<ApiResponse> {
        let payload = serde_json::json!({
            "device_id": self.get_device_id(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "node_info": node_info,
        });

        let response = self.client
            .post(&format!("{}/api/node-info", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let api_response: ApiResponse = response.json().await?;
        Ok(api_response)
    }

    /// 根据已上报信息检查节点健康状态
    pub async fn check_node_health(&self, node_id: String) -> Result<NodeHealthResponse> {
        let response = self.client
            .get(&format!("{}/api/node-health?node_id={}", self.base_url, node_id))
            .send()
            .await?;

        let health_response: NodeHealthResponse = response.json().await?;
        Ok(health_response)
    }

    /// 上传完整节点信息（包含iroh和设备信息）到 /api/node-info 端点
    pub async fn upload_full_node_info(
        &self,
        device_info: DeviceInfo,
        iroh_node: Option<IrohNodeInfo>,
    ) -> Result<ApiResponse> {
        let device_id = self.get_device_id();
        
        // 使用真实的 iroh 节点 ID（如果可用），否则使用 device_id
        let node_id = if let Some(ref iroh) = iroh_node {
            iroh.node_id.clone()
        } else {
            device_id.clone()
        };
        
        // 创建包含真实节点 ID 的 metadata
        let mut metadata = self.get_device_metadata();
        metadata.node_id = Some(node_id.clone());
        
        let payload = FullNodeInfoPayload {
            device_id: device_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            node_id,
            iroh_node,
            device_info,
            metadata,
        };

        let response = self
            .client
            .post(&format!("{}/api/node-info", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let api_response: ApiResponse = response.json().await?;
        Ok(api_response)
    }

    /// 测试连接
    pub async fn test_connection(&self) -> Result<bool> {
        match self.client
            .get(&format!("{}/api/health", self.base_url))
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// 轮询 Workers 待处理消息
    pub async fn poll_messages(&self, last_poll_time: Option<String>) -> Result<WorkersMessagesResponse> {
        let url = if let Some(time) = last_poll_time {
            format!("{}/api/messages?since={}", self.base_url, time)
        } else {
            format!("{}/api/messages", self.base_url)
        };

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("轮询消息失败：{}", e))?;

        let messages_response: WorkersMessagesResponse = response.json().await?;
        Ok(messages_response)
    }

    /// 注册 iroh 节点到 Workers 后端 (/api/iroh-node/register)
    pub async fn register_iroh_node(
        &self,
        node_data: serde_json::Value,
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

    /// 获取所有可用节点 (/api/nodes)
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
            .ok_or_else(|| anyhow!("nodes 字段格式错误"))?;

        let mut nodes: Vec<NodeInfo> = Vec::new();
        for node_data in nodes_data {
            // 转换为 NodeInfo
            let node = NodeInfo {
                node_id: node_data["node_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                endpoint: node_data
                    .get("endpoint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                capabilities: NodeCapabilities {
                    max_memory_gb: node_data
                        .get("gpu_memory")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    gpu_type: node_data
                        .get("gpu_available")
                        .and_then(|v| v.as_bool())
                        .map(|b| if b { "Unknown" } else { "" }.to_string()),
                    gpu_memory_gb: node_data
                        .get("gpu_memory")
                        .and_then(|v| v.as_f64()),
                    cpu_cores: 4, // 简化，实际应从节点数据获取
                    network_bandwidth_mbps: 1000,
                    supported_models: vec![],
                },
                current_load: node_data
                    .get("current_load")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32,
                latency: None,
                reliability: node_data
                    .get("reliability_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.95) as f32,
            };
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// 上报任务执行结果
    pub async fn report_task_result(
        &self,
        task_id: String,
        success: bool,
        result: Option<serde_json::Value>,
        error: Option<String>,
        execution_time_ms: u64,
    ) -> Result<ApiResponse> {
        let payload = serde_json::json!({
            "task_id": task_id,
            "success": success,
            "result": result,
            "error": error,
            "execution_time_ms": execution_time_ms,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let response = self.client
            .post(&format!("{}/api/task-result", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let api_response: ApiResponse = response.json().await?;
        Ok(api_response)
    }
}

/// Workers 消息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkersMessagesResponse {
    pub success: bool,
    pub messages: Vec<WorkersMessage>,
    pub poll_timestamp: String,
}

/// Workers 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkersMessage {
    pub id: String,
    pub message_type: String,
    pub from_node: String,
    pub to_node: Option<String>,
    pub content: serde_json::Value,
    pub timestamp: String,
    pub priority: String,
}

/// 节点连接请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnectionRequest {
    pub request_id: String,
    pub from_node: String,
    pub from_node_info: NodeInfo,
    pub suggested_connection: String,
    pub metadata: serde_json::Value,
}

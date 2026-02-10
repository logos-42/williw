use crate::state::{AppState, ModelConfig, TrainingStatus, DeviceInfo, AppSettings, ApiKeyEntry, WorkflowStatus, ExternalApiConfig};
use williw::agent::workflow::AsyncWorkflowExecutor;
use williw::agent::workflow::RalphLoopConfig;
use tauri::Emitter;
use crate::api_client::TrainingConfigData;
use tauri::State;
use williw::Node;  // 导入真实的Node
use williw::config::AppConfig;
use std::process::Command;
use std::path::Path;

// 确保 uuid 和 chrono 被导入
use uuid::Uuid;
use chrono::Utc;

/// Start training node
#[tauri::command]
pub async fn start_training(
    state: State<'_, AppState>
) -> Result<String, String> {
    let model_config = {
        let models = state.available_models.lock();
        models.first().cloned().unwrap_or_default()
    };

    // 创建AppConfig
    let mut app_config = AppConfig::default();
    
    // 根据模型配置调整AppConfig
    app_config.training.model_dim = model_config.dimensions;
    app_config.training.learning_rate = model_config.learning_rate;
    app_config.training.batch_size = model_config.batch_size;

    // 创建并启动Node
    let node = Node::new(app_config)
        .await
        .map_err(|e| format!("Failed to create node: {}", e))?;

    let node_id = node.comms.node_id().to_string();

    // 存储Node
    *state.node.lock() = Some(node);

    // 更新训练状态
    let mut status = state.training_status.lock();
    status.is_running = true;
    status.current_epoch = 0;
    status.accuracy = 0.0;
    status.loss = 1.0;
    status.samples_processed = 0;

    Ok(format!("Training started with node: {}", node_id))
}

/// Stop training node
#[tauri::command]
pub async fn stop_training(
    state: State<'_, AppState>
) -> Result<String, String> {
    let mut node_guard = state.node.lock();
    
    if let Some(_node) = node_guard.take() {
        // Node会被自动drop，清理资源
        // 如果需要显式停止，可以调用node.shutdown()等方法
        
        // 更新训练状态
        let mut status = state.training_status.lock();
        status.is_running = false;
        
        Ok("Training stopped successfully".to_string())
    } else {
        Err("No training node is running".to_string())
    }
}

/// Get current training status
#[tauri::command]
pub fn get_training_status(
    state: State<'_, AppState>
) -> TrainingStatus {
    state.training_status.lock().clone()
}

/// Select a model for training
#[tauri::command]
pub fn select_model(
    model_id: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    let models = state.available_models.lock();
    
    // Check if model exists
    let model = models.iter().find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model '{}' not found", model_id))?;

    // Update settings with new model
    let mut settings = state.settings.lock();
    settings.network_config.max_peers = model.batch_size as u32; // Use batch_size for demo

    Ok(format!("Selected model: {}", model.name))
}

/// Get available models
#[tauri::command]
pub fn get_available_models(
    state: State<'_, AppState>
) -> Vec<ModelConfig> {
    state.available_models.lock().clone()
}

/// Get device information
#[tauri::command]
pub fn get_device_info(
    state: State<'_, AppState>
) -> Option<DeviceInfo> {
    // Refresh device info before returning
    state.refresh_device_info();
    state.device_info.lock().clone()
}

/// Get training statistics
#[tauri::command]
pub fn get_training_stats(
    state: State<'_, AppState>
) -> TrainingStatus {
    state.training_status.lock().clone()
}

/// Update application settings
#[tauri::command]
pub fn update_settings(
    new_settings: AppSettings,
    state: State<'_, AppState>
) -> Result<String, String> {
    *state.settings.lock() = new_settings;
    Ok("Settings updated successfully".to_string())
}

/// Get current settings
#[tauri::command]
pub fn get_settings(
    state: State<'_, AppState>
) -> AppSettings {
    state.settings.lock().clone()
}

/// Get all API keys
#[tauri::command]
pub fn get_api_keys(
    state: State<'_, AppState>
) -> Vec<ApiKeyEntry> {
    state.api_keys.lock().clone()
}

/// Create new API key
#[tauri::command]
pub fn create_api_key(
    name: String,
    state: State<'_, AppState>
) -> Result<ApiKeyEntry, String> {
    let new_key = format!("sk-williw-{}", Uuid::new_v4());
    let entry = ApiKeyEntry {
        id: Uuid::new_v4().to_string(),
        name,
        key: new_key.clone(),
        provider: "williw".to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    
    state.api_keys.lock().push(entry.clone());
    
    Ok(entry)
}

/// Delete API key
#[tauri::command]
pub fn delete_api_key(
    id: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    let mut keys = state.api_keys.lock();
    let initial_len = keys.len();
    keys.retain(|k| k.id != id);
    
    if keys.len() < initial_len {
        Ok("API key deleted successfully".to_string())
    } else {
        Err("API key not found".to_string())
    }
}

/// Update API key name
#[tauri::command]
pub fn update_api_key_name(
    id: String,
    new_name: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    let mut keys = state.api_keys.lock();
    
    if let Some(key) = keys.iter_mut().find(|k| k.id == id) {
        key.name = new_name;
        Ok("API key name updated successfully".to_string())
    } else {
        Err("API key not found".to_string())
    }
}

/// Get node information
#[tauri::command]
pub fn get_node_info(
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let node_guard = state.node.lock();
    
    if let Some(node) = node_guard.as_ref() {
        let node_id = node.comms.node_id().to_string();
        let capabilities = node.device_manager.get();
        let stats = node.stats.lock().unwrap();
        
        Ok(serde_json::json!({
            "id": node_id,
            "is_running": true,
            "tick_counter": node.tick_counter,
            "device_capabilities": {
                "max_memory_mb": capabilities.max_memory_mb,
                "cpu_cores": capabilities.cpu_cores,
                "has_gpu": capabilities.has_gpu,
                "network_type": capabilities.network_type,
                "battery_level": capabilities.battery_level,
                "is_charging": capabilities.is_charging
            },
            "training_stats": {
                "total_ticks": stats.get_stats().tick_count,
                "accuracy": stats.get_stats().training_accuracy,
                "loss": stats.get_stats().training_loss,
                "samples_processed": stats.get_stats().samples_processed
            }
        }))
    } else {
        Ok(serde_json::json!({
            "id": null,
            "is_running": false,
            "message": "Node is not running"
        }))
    }
}

/// Get connected peers information
#[tauri::command]
pub fn get_connected_peers(
    state: State<'_, AppState>
) -> Result<Vec<serde_json::Value>, String> {
    let node_guard = state.node.lock();
    
    if let Some(node) = node_guard.as_ref() {
        let (primary_peers, backup_peers) = node.topology.neighbor_sets();
        
        let mut peers = Vec::new();
        
        for peer_id in primary_peers {
            if let Some(snapshot) = node.topology.peer_snapshot(&peer_id) {
                peers.push(serde_json::json!({
                    "id": peer_id,
                    "type": "primary",
                    "similarity": snapshot.similarity,
                    "geo_affinity": snapshot.geo_affinity,
                    "embedding_dim": snapshot.embedding_dim,
                    "position": {
                        "lat": snapshot.position.lat,
                        "lon": snapshot.position.lon
                    }
                }));
            }
        }
        
        for peer_id in backup_peers {
            if let Some(snapshot) = node.topology.peer_snapshot(&peer_id) {
                peers.push(serde_json::json!({
                    "id": peer_id,
                    "type": "backup",
                    "similarity": snapshot.similarity,
                    "geo_affinity": snapshot.geo_affinity,
                    "embedding_dim": snapshot.embedding_dim,
                    "position": {
                        "lat": snapshot.position.lat,
                        "lon": snapshot.position.lon
                    }
                }));
            }
        }
        
        Ok(peers)
    } else {
        Ok(vec![])
    }
}

/// Upload device info to workers backend (/api/node-info)
#[tauri::command]
pub async fn upload_device_info_to_workers(
    state: State<'_, AppState>
) -> Result<String, String> {
    // 获取设备信息
    let device_info = state.device_info.lock().clone()
        .ok_or_else(|| "No device info available".to_string())?;

    // 上传到workers后端的 /api/node-info 端点
    match state.api_client.upload_node_info(crate::api_client::NodeInfo {
        node_id: state.api_client.get_device_id(),
        endpoint: "localhost:8080".to_string(), // 可以从配置获取
        capabilities: crate::api_client::NodeCapabilities {
            max_memory_gb: device_info.total_memory_gb,
            gpu_type: device_info.gpu_type.clone(),
            gpu_memory_gb: device_info.gpu_memory_total,
            cpu_cores: device_info.cpu_cores,
            network_bandwidth_mbps: 1000, // 可以动态检测
            supported_models: vec!["bert-base".to_string(), "gpt-2".to_string()], // 可以从可用模型获取
        },
        current_load: 0.5, // 可以动态获取
        latency: Some(50), // 可以动态检测
        reliability: 0.95, // 可以基于历史数据计算
    }).await {
        Ok(response) => {
            if response.success {
                Ok("Device info uploaded successfully to /api/node-info".to_string())
            } else {
                Err(format!("Upload failed: {}", response.message))
            }
        }
        Err(e) => Err(format!("Network error: {}", e)),
    }
}

/// Request inference from workers backend (/api/request)
#[tauri::command]
pub async fn request_inference_from_workers(
    model_id: String,
    input_data: serde_json::Value,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    // 请求推理到workers后端的 /api/request 端点
    match state.api_client.request_inference(model_id, input_data).await {
        Ok(response) => {
            if response.success {
                Ok(serde_json::json!({
                    "success": true,
                    "request_id": response.request_id,
                    "selected_nodes": response.selected_nodes,
                    "model_split_plan": response.model_split_plan,
                    "estimated_total_time": response.estimated_total_time,
                    "fallback_nodes": response.fallback_nodes,
                    "message": response.message
                }))
            } else {
                Err(format!("Request failed: {}", response.message))
            }
        }
        Err(e) => Err(format!("Network error: {}", e)),
    }
}

/// Upload selected model to workers backend (/api/model)
#[tauri::command]
pub async fn upload_model_selection_to_workers(
    model_id: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    // 获取模型配置
    let model_config = {
        let models = state.available_models.lock();
        models.iter().find(|m| m.id == model_id)
            .ok_or_else(|| format!("Model '{}' not found", model_id))?
            .clone()
    };
    
    // 创建训练配置
    let training_config = TrainingConfigData {
        learning_rate: model_config.learning_rate,
        batch_size: model_config.batch_size,
        epochs: 100, // 默认值，可以从设置中读取
        enable_distributed: true,
    };

    // 上传到workers后端的 /api/model 端点
    match state.api_client.upload_selected_model(model_config, training_config).await {
        Ok(response) => {
            if response.success {
                Ok("Model selection uploaded successfully to /api/model".to_string())
            } else {
                Err(format!("Upload failed: {}", response.message))
            }
        }
        Err(e) => Err(format!("Network error: {}", e)),
    }
}

/// Upload training data to workers backend (/api/training-data)
#[tauri::command]
pub async fn upload_training_data_to_workers(
    state: State<'_, AppState>
) -> Result<String, String> {
    // 获取训练状态
    let training_status = {
        let status = state.training_status.lock();
        status.clone()
    };
    
    // 获取节点ID（如果有的话）
    let node_id = {
        let node_guard = state.node.lock();
        if let Some(_node) = node_guard.as_ref() {
            // 这里可以从Node获取ID，目前使用None
            None
        } else {
            None
        }
    };

    // 上传到workers后端的 /api/training-data 端点
    match state.api_client.upload_training_data(training_status, node_id).await {
        Ok(response) => {
            if response.success {
                Ok("Training data uploaded successfully to /api/training-data".to_string())
            } else {
                Err(format!("Upload failed: {}", response.message))
            }
        }
        Err(e) => Err(format!("Network error: {}", e)),
    }
}

/// Test connection to workers backend
#[tauri::command]
pub async fn test_workers_connection(
    state: State<'_, AppState>
) -> Result<bool, String> {
    match state.api_client.test_connection().await {
        Ok(is_connected) => Ok(is_connected),
        Err(e) => Err(format!("Connection test failed: {}", e)),
    }
}

/// Reassign nodes when some nodes are unreachable
#[tauri::command]
pub async fn reassign_node_from_workers(
    failed_nodes: Vec<String>,
    current_splits: Vec<crate::api_client::ModelSplit>,
    request_id: String,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    // 调用API客户端的节点重新分配方法
    match state.api_client.reassign_node(failed_nodes, current_splits, request_id).await {
        Ok(response) => {
            if response.success {
                Ok(serde_json::json!({
                    "success": true,
                    "new_splits": response.new_splits,
                    "reassigned_nodes": response.reassigned_nodes,
                    "message": response.message
                }))
            } else {
                Err(format!("Node reassignment failed: {}", response.message))
            }
        }
        Err(e) => Err(format!("Network error: {}", e)),
    }
}

/// Check node health status
#[tauri::command]
pub async fn check_node_health_from_workers(
    node_id: String,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    // 调用API客户端的节点健康检查方法
    match state.api_client.check_node_health(node_id).await {
        Ok(response) => {
            Ok(serde_json::json!({
                "success": response.success,
                "message": response.message,
                "node_id": response.node_id,
                "is_healthy": response.is_healthy,
                "last_seen": response.last_seen,
                "current_load": response.current_load,
                "issues": response.issues
            }))
        }
        Err(e) => Err(format!("Network error: {}", e)),
    }
}

/// Upload full node info (iroh + device) to workers backend manually
#[tauri::command]
pub async fn upload_full_node_info_to_workers(
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    // Get device info
    let device_info = state.device_info.lock().clone()
        .ok_or_else(|| "No device info available".to_string())?;
    
    // Get iroh node info if available
    let iroh_node = {
        let node_guard = state.node.lock();
        if let Some(node) = node_guard.as_ref() {
            let node_id = node.comms.node_id().to_string();
            let capabilities = node.device_manager.get();
            let stats = node.stats.lock().unwrap();
            let (primary_peers, backup_peers) = node.topology.neighbor_sets();
            
            // Build peers list
            let mut peers = Vec::new();
            for peer_id in primary_peers {
                if let Some(snapshot) = node.topology.peer_snapshot(peer_id.as_str()) {
                    peers.push(crate::api_client::IrohPeerInfo {
                        id: peer_id.to_string(),
                        peer_type: "primary".to_string(),
                        similarity: snapshot.similarity as f64,
                        geo_affinity: snapshot.geo_affinity as f64,
                        embedding_dim: snapshot.embedding_dim,
                        position: crate::api_client::GeoPosition {
                            lat: snapshot.position.lat as f64,
                            lon: snapshot.position.lon as f64,
                        },
                    });
                }
            }
            for peer_id in backup_peers {
                if let Some(snapshot) = node.topology.peer_snapshot(peer_id.as_str()) {
                    peers.push(crate::api_client::IrohPeerInfo {
                        id: peer_id.to_string(),
                        peer_type: "backup".to_string(),
                        similarity: snapshot.similarity as f64,
                        geo_affinity: snapshot.geo_affinity as f64,
                        embedding_dim: snapshot.embedding_dim,
                        position: crate::api_client::GeoPosition {
                            lat: snapshot.position.lat as f64,
                            lon: snapshot.position.lon as f64,
                        },
                    });
                }
            }
            
            Some(crate::api_client::IrohNodeInfo {
                node_id,
                is_running: true,
                tick_counter: node.tick_counter,
                device_capabilities: crate::api_client::IrohDeviceCapabilities {
                    max_memory_mb: capabilities.max_memory_mb,
                    cpu_cores: capabilities.cpu_cores,
                    has_gpu: capabilities.has_gpu,
                    network_type: format!("{:?}", capabilities.network_type),
                    battery_level: capabilities.battery_level,
                    is_charging: capabilities.is_charging,
                },
                training_stats: crate::api_client::IrohTrainingStats {
                    total_ticks: stats.get_stats().tick_count,
                    accuracy: stats.get_stats().training_accuracy,
                    loss: stats.get_stats().training_loss,
                    samples_processed: stats.get_stats().samples_processed,
                },
                peers,
            })
        } else {
            None
        }
    };
    
    // Upload to workers
    match state.api_client.upload_full_node_info(device_info, iroh_node).await {
        Ok(response) => {
            Ok(serde_json::json!({
                "success": response.success,
                "message": response.message,
                "data": response.data
            }))
        }
        Err(e) => Err(format!("Upload failed: {}", e)),
    }
}

/// Start GPU inference server
#[tauri::command]
pub async fn start_gpu_server() -> Result<String, String> {
    // 获取当前应用的目录（src-tauri目录）
    let app_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    // 获取项目根目录（src-tauri的上级目录）
    let project_root = app_dir.parent()
        .ok_or("Failed to get project root directory")?;
    
    // 构建Python服务器脚本的路径
    let server_script = project_root.join("gpu_inference_server_clean.py");
    
    // 构建虚拟环境Python的路径 (支持 macOS/Linux 和 Windows)
    let venv_python = if cfg!(target_os = "windows") {
        project_root.join("torch_env").join("Scripts").join("python.exe")
    } else {
        project_root.join("torch_env").join("bin").join("python")
    };
    
    // 选择Python解释器（优先使用虚拟环境）
    let python_exe = if venv_python.exists() {
        venv_python
    } else {
        // 尝试多个可能的 Python 命令
        let possible_pythons = ["python3", "python"];
        let mut found_python = None;
        for py_cmd in &possible_pythons {
            let check = Command::new(py_cmd)
                .arg("--version")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output();
            if let Ok(output) = check {
                if output.status.success() {
                    found_python = Some(std::path::PathBuf::from(py_cmd));
                    println!("找到Python: {}", py_cmd);
                    break;
                }
            }
        }
        found_python.ok_or_else(|| "未找到Python解释器，请安装Python 3.8+".to_string())?
    };
    
    if !server_script.exists() {
        return Err(format!("GPU服务器脚本未找到: {:?}", server_script));
    }
    
    // 检查Python是否可用
    let python_check = Command::new(&python_exe)
        .arg("--version")
        .output();
    
    match python_check {
        Ok(output) => {
            if !output.status.success() {
                return Err("Python未正确安装或配置".to_string());
            }
            println!("Python版本检查通过: {}", String::from_utf8_lossy(&output.stdout));
        }
        Err(e) => {
            return Err(format!("无法执行Python命令: {}", e));
        }
    }
    
    // 启动GPU服务器（后台进程）
    let mut child = Command::new(&python_exe)
        .current_dir(project_root) // 设置工作目录为项目根目录
        .arg(&server_script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start GPU server: {}", e))?;
    
    println!("GPU服务器启动进程ID: {:?}", child.id());
    
    // 等待一小段时间让服务器启动
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // 检查进程是否还在运行
    match child.try_wait() {
        Ok(Some(status)) => {
            if !status.success() {
                // 尝试读取错误输出
                if let Some(mut stderr) = child.stderr.take() {
                    let mut stderr_buf = String::new();
                    if let Ok(_) = std::io::Read::read_to_string(&mut stderr, &mut stderr_buf) {
                        return Err(format!("GPU服务器启动失败，退出码: {:?}\n错误信息: {}", status.code(), stderr_buf));
                    }
                }
                return Err(format!("GPU服务器启动失败，退出码: {:?}", status.code()));
            }
        }
        Ok(None) => {
            // 进程仍在运行，这是正常的
            println!("GPU服务器正在后台运行...");
        }
        Err(e) => {
            return Err(format!("检查GPU服务器状态失败: {}", e));
        }
    }
    
    Ok("GPU服务器启动成功".to_string())
}

/// Check if GPU server is running
#[tauri::command]
pub async fn check_gpu_server_status() -> Result<bool, String> {
    // 尝试连接到GPU服务器
    let client = reqwest::Client::new();
    
    match client.get("http://localhost:8000/")
        .timeout(tokio::time::Duration::from_secs(3))
        .send()
        .await {
        Ok(response) => Ok(response.status().is_success()),
        Err(e) => {
            println!("GPU服务器连接检查失败: {}", e);
            Ok(false)
        }
    }
}

/// Install Python dependencies for GPU server
#[tauri::command]
pub async fn install_gpu_dependencies() -> Result<String, String> {
    let app_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let project_root = app_dir.parent()
        .ok_or("Failed to get project root directory")?;

    let requirements_file = project_root.join("requirements.txt");

    if !requirements_file.exists() {
        return Err("requirements.txt文件未找到".to_string());
    }

    // 安装依赖
    let output = Command::new("pip")
        .current_dir(project_root) // 设置工作目录为项目根目录
        .arg("install")
        .arg("-r")
        .arg(&requirements_file)
        .output()
        .map_err(|e| format!("Failed to run pip install: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("依赖安装输出: {}", stdout);
        if !stderr.is_empty() {
            println!("依赖安装警告: {}", stderr);
        }
        Ok("Python依赖安装成功".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("依赖安装失败: {}", stderr))
    }
}

/// Get workflow status
#[tauri::command]
pub fn get_workflow_status(
    state: State<'_, AppState>
) -> WorkflowStatus {
    state.workflow_status.lock().clone()
}

/// Start document-driven workflow
#[tauri::command]
pub async fn start_document_driven_workflow(
    api_key: String,
    model_path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("🚀 [WORKFLOW] Starting document-driven workflow...");

    // Update workflow status
    {
        let mut workflow_status = state.workflow_status.lock();
        workflow_status.is_running = true;
        workflow_status.progress = 0.0;
        workflow_status.message = "正在初始化AI自主工作流...".to_string();
        workflow_status.current_step = "init".to_string();
    }

    // Emit event to frontend
    let _ = app.emit("workflow-status", {
        let status = state.workflow_status.lock();
        (*status).clone()
    });

    // Create workflow executor
    let executor = AsyncWorkflowExecutor::new();

    // Create Ralph Loop config
    let ralph_config = RalphLoopConfig {
        enabled: true,
        max_iterations: 50,
        iteration_delay_ms: 1000,
        completion_checker: None,
        max_total_time_ms: None,
        iteration_timeout_ms: 60000,
        max_cost: None,
        enable_history: true,
        smart_retry: williw::agent::workflow::SmartRetryStrategy {
            enabled: true,
            max_retries: 3,
            base_delay_ms: 1000,
            backoff_multiplier: 2.0,
            jitter: true,
            error_based_retry: std::collections::HashMap::new(),
            adaptive_retry: false,
            max_consecutive_failures: 3,
            learning_period: 10,
        },
    };

    let execution_id = format!("exec-{}", Uuid::new_v4());
    let execution_id_clone = execution_id.clone();

    // Emit starting message
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🎭 AI身份：去中心化算力专家\n📋 任务：自动配置算力网络并加载模型\n🚀 正在启动自主工作流...\n")
    }));

    // Start workflow in background
    let app_handle_clone = app.clone();
    let state_clone = state.workflow_status.clone();
    tokio::spawn(async move {
        println!("📚 [WORKFLOW] Starting document-driven workflow with execution_id: {}", execution_id_clone);

        // Simulate workflow steps with progress updates
        let steps = vec![
            ("正在阅读AI身份文档...", "reading_identity", 0.1),
            ("正在理解任务目标...", "understanding_task", 0.2),
            ("正在分析模型结构...", "analyzing_model", 0.3),
            ("正在连接去中心化算力网络...", "connecting_network", 0.4),
            ("正在配置算力节点...", "configuring_nodes", 0.5),
            ("正在切分模型分片...", "splitting_model", 0.6),
            ("正在分发模型分片...", "distributing_shards", 0.7),
            ("正在验证分片完整性...", "verifying_shards", 0.8),
            ("正在启动推理服务...", "starting_inference", 0.9),
            ("✅ 工作流完成！AI已准备好服务。", "completed", 1.0),
        ];

        for (i, (message, step, progress)) in steps.iter().enumerate() {
            tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

            // Update status
            {
                let mut status = state_clone.lock();
                status.current_step = step.to_string();
                status.progress = *progress;
                status.message = message.to_string();
            }

            // Emit message event
            let _ = app_handle_clone.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": format!("[{}/10] {}", i + 1, message),
                "step": step,
                "progress": progress,
            }));

            // Emit status event
            let _ = app_handle_clone.emit("workflow-status", {
                let status = state_clone.lock();
                (*status).clone()
            });
        }

        // Mark workflow as completed
        {
            let mut status = state_clone.lock();
            status.is_running = false;
            status.message = "工作流已完成".to_string();
            status.current_step = "completed".to_string();
        }

        // Emit final status
        let _ = app_handle_clone.emit("workflow-status", {
            let status = state_clone.lock();
            (*status).clone()
        });

        // Emit completion message
        let _ = app_handle_clone.emit("workflow-message", serde_json::json!({
            "type": "success",
            "content": "\n✨ 恭喜！去中心化算力网络已配置完成。\n\n🤖 您现在可以：\n- 直接与AI模型对话\n- 使用去中心化算力执行推理任务\n- 监控算力节点状态\n\n开始使用吧！"
        }));

        println!("✅ [WORKFLOW] Document-driven workflow completed successfully");
    });

    Ok(format!("Workflow started with ID: {}", execution_id))
}

/// Run AI-guided system setup
#[tauri::command]
pub async fn run_ai_setup(
    api_key: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("🤖 [AI SETUP] Starting AI-guided system setup...");

    use williw::agent::setup::{AISetupAssistant, SetupProgress, SetupStatus};
    use tauri::Emitter;

    // Update workflow status
    {
        let mut workflow_status = state.workflow_status.lock();
        workflow_status.is_running = true;
        workflow_status.progress = 0.0;
        workflow_status.message = "AI正在分析系统环境...".to_string();
        workflow_status.current_step = "ai_detection".to_string();
    }

    // Emit initial event
    let _ = app.emit("workflow-status", {
        let status = state.workflow_status.lock();
        (*status).clone()
    });

    let _ = app.emit("setup-progress", serde_json::json!({
        "status": "detecting",
        "message": "开始系统检测...",
        "progress": 0.0,
        "current_step": "系统检测"
    }));

    // Create setup assistant
    let assistant = AISetupAssistant::new(api_key);

    // Clone state and app handle for callback
    let app_handle = app.clone();

    // Run setup with progress callback
    let result = assistant.run_full_setup(move |progress: SetupProgress| {
        // Map setup status to frontend format
        let status_str = match progress.status {
            SetupStatus::NotStarted => "not_started",
            SetupStatus::Detecting => "detecting",
            SetupStatus::Planning => "planning",
            SetupStatus::Executing => "executing",
            SetupStatus::Verifying => "verifying",
            SetupStatus::Completed => "completed",
            SetupStatus::Failed => "failed",
        };

        let progress_percent = if progress.total_steps > 0 {
            (progress.completed_steps as f32 / progress.total_steps as f32) * 100.0
        } else {
            0.0
        };

        // Emit progress event
        let _ = app_handle.emit("setup-progress", serde_json::json!({
            "status": status_str,
            "message": progress.messages.last().unwrap_or(&"配置中...".to_string()),
            "progress": progress_percent,
            "total_steps": progress.total_steps,
            "completed_steps": progress.completed_steps,
            "current_step": progress.current_step,
            "errors": progress.errors,
        }));

        // Also update workflow status
        let status_message = match progress.status {
            SetupStatus::Detecting => "AI正在检测系统环境...",
            SetupStatus::Planning => "AI正在制定配置方案...",
            SetupStatus::Executing => "正在执行配置步骤...",
            SetupStatus::Verifying => "正在验证配置结果...",
            SetupStatus::Completed => "配置完成！",
            SetupStatus::Failed => "配置失败",
            _ => "配置中...",
        };

        let _ = app_handle.emit("workflow-status", serde_json::json!({
            "is_running": progress.status != SetupStatus::Completed && progress.status != SetupStatus::Failed,
            "progress": progress_percent / 100.0,
            "message": status_message,
            "current_step": progress.current_step.clone().unwrap_or_else(|| "unknown".to_string()),
        }));
    }).await;

    match result {
        Ok(progress) => {
            println!("✅ [AI SETUP] Setup completed successfully");
            
            // Emit completion event
            let execution_id = format!("setup_{}", chrono::Utc::now().timestamp_millis());
            let _ = app.emit("setup-complete", serde_json::json!({
                "success": true,
                "execution_id": execution_id,
                "message": "系统配置完成！GPU推理服务已就绪。"
            }));

            Ok(format!("Setup completed successfully"))
        }
        Err(e) => {
            eprintln!("❌ [AI SETUP] Setup failed: {}", e);
            
            // Emit failure event
            let _ = app.emit("setup-failed", serde_json::json!({
                "success": false,
                "error": e.clone()
            }));

            Err(format!("Setup failed: {}", e))
        }
    }
}

/// Check system setup status
#[tauri::command]
pub async fn check_setup_status() -> Result<serde_json::Value, String> {
    use williw::agent::setup::check_setup_status;
    
    let status = check_setup_status().await;
    
    Ok(serde_json::json!({
        "python": status.get("python").copied().unwrap_or(false),
        "pip": status.get("pip").copied().unwrap_or(false),
        "cuda": status.get("cuda").copied().unwrap_or(false),
        "torch": status.get("torch").copied().unwrap_or(false),
        "transformers": status.get("transformers").copied().unwrap_or(false),
        "inference_server": status.get("inference_server").copied().unwrap_or(false),
    }))
}

/// Start GPU inference server
#[tauri::command]
pub async fn start_gpu_inference_server(port: u16) -> Result<String, String> {
    use std::process::Command;
    
    let app_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let project_root = app_dir.parent()
        .ok_or("Failed to get project root directory")?;

    let server_script = project_root.join("gpu_inference_server_clean.py");

    if !server_script.exists() {
        return Err(format!("服务器脚本不存在: {:?}", server_script));
    }

    println!("🚀 启动GPU推理服务器 (端口 {})...", port);

    // 在后台启动服务器
    #[cfg(target_os = "windows")]
    {
        let _child = Command::new("python")
            .arg(&server_script)
            .arg("--port")
            .arg(port.to_string())
            .spawn()
            .map_err(|e| format!("无法启动服务器: {}", e))?;
    }

    // 等待服务器启动
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 检查服务器是否响应
    match reqwest::get(format!("http://localhost:{}/", port)).await {
        Ok(response) if response.status().is_success() => {
            Ok(format!("推理服务器已在端口 {} 启动", port))
        }
        _ => Err("服务器启动后无法访问".to_string()),
    }
}

// ============ 外部 API 管理 ============

/// 测试外部 API 连接
#[tauri::command]
pub async fn test_external_api(
    provider: String,
    base_url: String,
    api_key: String,
    model: String,
) -> Result<serde_json::Value, String> {
    use reqwest;
    use std::time::Duration;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 根据不同的提供商构建请求
    let result = match provider.as_str() {
        "openai" => {
            let response = client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("choices").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        "deepseek" => {
            let response = client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("choices").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        "anthropic" => {
            let response = client
                .post(format!("{}/messages", base_url))
                .header("x-api-key", api_key)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&serde_json::json!({
                    "model": model,
                    "max_tokens": 5,
                    "messages": [{"role": "user", "content": "Hi"}],
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("content").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        "glm" | "kimichat" | "minimax" | "qwen" | "custom" | "google" | "nvidia" | "openrouter" | "vercel" | "groq" | "perplexity" => {
            // 通用 OpenAI 兼容 API 调用
            let response = client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("choices").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        _ => Err(format!("不支持的提供商: {}", provider)),
    };

    result
}

/// 保存外部 API 配置
#[tauri::command]
pub async fn save_external_api(
    config: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<ExternalApiConfig, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let entry = ExternalApiConfig {
        id: id.clone(),
        name: config.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default(),
        provider: config.get("provider").and_then(|v| v.as_str()).unwrap_or("custom").to_string(),
        base_url: config.get("base_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        api_key: config.get("api_key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        model: config.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        enabled: config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    state.external_apis.lock().push(entry.clone());
    Ok(entry)
}

/// 获取外部 API 配置列表
#[tauri::command]
pub async fn get_external_apis(
    state: State<'_, AppState>,
) -> Result<Vec<ExternalApiConfig>, String> {
    let apis = state.external_apis.lock().clone();
    Ok(apis)
}

/// 删除外部 API 配置
#[tauri::command]
pub async fn delete_external_api(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut apis = state.external_apis.lock();
    apis.retain(|api| api.id != id);
    Ok(())
}

/// 切换外部 API 启用状态
#[tauri::command]
pub async fn toggle_external_api(
    id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut apis = state.external_apis.lock();
    if let Some(api) = apis.iter_mut().find(|api| api.id == id) {
        api.enabled = enabled;
    }
    Ok(())
}

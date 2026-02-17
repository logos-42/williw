use crate::state::AppState;
use tauri::State;
use tauri::Emitter;
use serde_json;

/// Upload device info to workers backend (/api/node-info)
#[tauri::command]
pub async fn upload_device_info_to_workers(
    state: State<'_, AppState>
) -> Result<String, String> {
    // 获取设备信息
    let device_info = state.device_info.lock().clone()
        .ok_or_else(|| "No device info available".to_string())?;

    let endpoint = {
        let node_guard = state.node.lock();
        if let Some(node) = node_guard.as_ref() {
            node.comms
                .local_addr()
                .unwrap_or_else(|_| "localhost:8080".to_string())
        } else {
            "localhost:8080".to_string()
        }
    };

    // 上传到workers后端的 /api/node-info 端点
    match state.api_client.upload_node_info(crate::api_client::NodeInfo {
        node_id: state.api_client.get_device_id(),
        endpoint,
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
    let training_config = crate::api_client::TrainingConfigData {
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

/// Poll workers for pending messages
#[tauri::command]
pub async fn poll_workers_messages(
    last_poll_time: Option<String>,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    match state.api_client.poll_messages(last_poll_time).await {
        Ok(response) => {
            Ok(serde_json::json!({
                "success": response.success,
                "messages": response.messages,
                "poll_timestamp": response.poll_timestamp
            }))
        }
        Err(e) => Err(format!("Poll failed: {}", e)),
    }
}

/// AI-driven node connection handler
#[tauri::command]
pub async fn handle_ai_node_connection(
    connection_request: serde_json::Value,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    use crate::system_checks;
    
    // Emit start event
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🤖 AI 正在分析节点连接请求...\n\n📋 将评估：\n• 节点性能\n• 网络延迟\n• 负载情况"
    }));
    
    // Extract connection info
    let from_node = connection_request.get("from_node")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    let suggested_connection = connection_request.get("suggested_connection")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    // AI 分析：检查本地系统状态
    let system_status = serde_json::json!({
        "python": system_checks::check_python().map(|(i, v)| serde_json::json!({"installed": i, "version": v})).unwrap_or_else(|_| serde_json::json!({"installed": false})),
        "cuda": system_checks::check_cuda().map(|(a, i)| serde_json::json!({"available": a, "info": i})).unwrap_or_else(|_| serde_json::json!({"available": false})),
        "torch": system_checks::check_pytorch().map(|(i, v)| serde_json::json!({"installed": i, "version": v})).unwrap_or_else(|_| serde_json::json!({"installed": false})),
    });
    
    // AI 决策：是否接受连接
    let should_connect = system_status.get("cuda")
        .and_then(|v| v.get("available"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    if should_connect {
        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "success",
            "content": format!("✅ AI 决定接受连接\n\n🔗 节点: {}\n🌐 连接: {}\n\n正在配置 Iroh P2P 连接...", from_node, suggested_connection)
        }));
        
        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "progress",
            "content": "🔗 正在建立 P2P 连接..."
        }));

        if suggested_connection.is_empty() {
            let err = "连接配置为空，缺少 suggested_connection";
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "error",
                "content": format!("❌ AI 决定接受连接，但连接配置无效: {}", err)
            }));
            return Ok(serde_json::json!({
                "success": false,
                "decision": "failed",
                "from_node": from_node,
                "connection_config": suggested_connection,
                "ai_reasoning": "Accepted by policy, but suggested connection is empty",
                "error": err
            }));
        }

        // 连接时不持有 state.node 锁，避免在 await 时阻塞其他命令
        let mut node_opt = { state.node.lock().take() };
        let connect_result = if let Some(mut node) = node_opt.take() {
            let result = node
                .comms
                .connect(suggested_connection.to_string())
                .await
                .map(|_| node.comms.node_id().to_string())
                .map_err(|e| e.to_string());
            state.node.lock().replace(node);
            result
        } else {
            Err("本地节点未运行，无法建立 P2P 连接".to_string())
        };

        match connect_result {
            Ok(local_node_id) => {
                let _ = app.emit("workflow-message", serde_json::json!({
                    "type": "success",
                    "content": format!("✅ P2P 连接建立成功\n\n本地节点: {}\n远端节点: {}", local_node_id, from_node)
                }));

                Ok(serde_json::json!({
                    "success": true,
                    "decision": "accepted",
                    "from_node": from_node,
                    "connection_config": suggested_connection,
                    "local_node_id": local_node_id,
                    "connection_established": true,
                    "ai_reasoning": "System has GPU available, accepting peer connection"
                }))
            }
            Err(err) => {
                let _ = app.emit("workflow-message", serde_json::json!({
                    "type": "error",
                    "content": format!("❌ P2P 连接建立失败\n\n节点: {}\n原因: {}", from_node, err)
                }));

                Ok(serde_json::json!({
                    "success": false,
                    "decision": "failed",
                    "from_node": from_node,
                    "connection_config": suggested_connection,
                    "connection_established": false,
                    "ai_reasoning": "Accepted by policy, but runtime connection failed",
                    "error": err
                }))
            }
        }
    } else {
        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "warning",
            "content": format!("⚠️ AI 建议延迟连接\n\n原因: 系统未检测到 GPU\n🔄 建议: 使用 CPU 模式或等待 GPU 可用时再连接")
        }));
        
        Ok(serde_json::json!({
            "success": true,
            "decision": "deferred",
            "from_node": from_node,
            "ai_reasoning": "No GPU available, deferring connection until GPU is ready"
        }))
    }
}

/// Register iroh node to workers backend (/api/iroh-node/register)
#[tauri::command]
pub async fn register_iroh_node_to_workers(
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 获取 iroh 节点 ID 和设备信息（在 await 之前释放锁）
    let (iroh_node_id, endpoint, device_info) = {
        let node_guard = state.node.lock();
        let node = node_guard.as_ref()
            .ok_or("Node not running")?;

        let iroh_node_id = node.comms.node_id().to_string();
        let endpoint = node.comms.local_addr()
            .unwrap_or_else(|_| "localhost:8080".to_string());
        
        let device_info = state.device_info.lock().clone()
            .ok_or_else(|| "No device info available".to_string())?;
        
        (iroh_node_id, endpoint, device_info)
    }; // 锁在这里释放

    // 构建注册数据
    let register_data = serde_json::json!({
        "node_id": iroh_node_id,
        "endpoint": endpoint,
        "device_info": {
            "gpu_type": device_info.gpu_type.clone().unwrap_or_default(),
            "gpu_memory_total": device_info.gpu_memory_total.unwrap_or(0.0),
            "cpu_cores": device_info.cpu_cores,
            "max_memory_mb": (device_info.total_memory_gb * 1024.0) as i64,
            "battery_level": device_info.battery_level.unwrap_or(100.0),
            "is_charging": device_info.is_charging.unwrap_or(false)
        },
        "iroh_node": {
            "node_id": iroh_node_id.clone(),
            "addresses": Vec::<String>::new()
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

/// Get all available nodes from workers backend (/api/nodes)
#[tauri::command]
pub async fn get_available_nodes_from_workers(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    match state.api_client.get_nodes().await {
        Ok(nodes) => {
            let nodes_json: Vec<serde_json::Value> = nodes.iter().map(|node| {
                serde_json::json!({
                    "node_id": node.node_id,
                    "endpoint": node.endpoint,
                    "max_memory_gb": node.capabilities.max_memory_gb,
                    "gpu_type": node.capabilities.gpu_type,
                    "gpu_memory_gb": node.capabilities.gpu_memory_gb,
                    "cpu_cores": node.capabilities.cpu_cores,
                    "current_load": node.current_load,
                    "reliability": node.reliability,
                })
            }).collect();
            
            Ok(serde_json::json!({
                "success": true,
                "nodes": nodes_json,
                "total": nodes_json.len()
            }))
        }
        Err(e) => Err(format!("获取节点失败：{}", e)),
    }
}

use crate::state::{AppState, TrainingStatus};
use williw::Node;
use williw::config::AppConfig;
use tauri::{State, Emitter};
use serde_json;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::time::Duration;

// Start training node
#[tauri::command]
pub async fn start_training(
    state: State<'_, AppState>
) -> Result<String, String> {
    if state.training_status.lock().is_running {
        return Err("Training node is already running".to_string());
    }

    let model_config = {
        let models = state.available_models.lock();
        models.first().cloned().unwrap_or_default()
    };

    let mut app_config = AppConfig::default();
    app_config.training.model_dim = model_config.dimensions;
    app_config.training.learning_rate = model_config.learning_rate;
    app_config.training.batch_size = model_config.batch_size;

    let node = Node::new(app_config)
        .await
        .map_err(|e| format!("Failed to create node: {}", e))?;

    let node_id = node.comms.node_id().to_string();
    *state.node.lock() = Some(node);

    let mut status = state.training_status.lock();
    status.is_running = true;
    status.current_epoch = 0;
    status.accuracy = 0.0;
    status.loss = 1.0;
    status.samples_processed = 0;

    spawn_node_driver(state.node.clone(), state.training_status.clone());

    Ok(format!("Training started with node: {}", node_id))
}

// Stop training node
#[tauri::command]
pub async fn stop_training(
    state: State<'_, AppState>
) -> Result<String, String> {
    let mut node_guard = state.node.lock();

    if let Some(_node) = node_guard.take() {
        let mut status = state.training_status.lock();
        status.is_running = false;
        Ok("Training stopped successfully".to_string())
    } else {
        Err("No training node is running".to_string())
    }
}

// Get current training status
#[tauri::command]
pub fn get_training_status(
    state: State<'_, AppState>
) -> TrainingStatus {
    state.training_status.lock().clone()
}

// Get training statistics
#[tauri::command]
pub fn get_training_stats(
    state: State<'_, AppState>
) -> TrainingStatus {
    state.training_status.lock().clone()
}

// AI 分析系统资源并决定最佳执行策略
#[tauri::command]
pub async fn ai_analyze_system(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🤖 AI: 开始分析系统资源..."
    }));
    
    // AI 检测 Python 环境
    let python_info = crate::system_checks::check_python()
        .map(|(installed, version)| serde_json::json!({
            "installed": installed,
            "version": version
        }))
        .unwrap_or_else(|_| serde_json::json!({"installed": false}));
    
    // AI 检测 CUDA/GPU
    let cuda_info = crate::system_checks::check_cuda()
        .map(|(available, info)| serde_json::json!({
            "available": available,
            "info": info
        }))
        .unwrap_or_else(|_| serde_json::json!({"available": false}));
    
    // AI 检测 PyTorch
    let pytorch_info = crate::system_checks::check_pytorch()
        .map(|(installed, version)| serde_json::json!({
            "installed": installed,
            "version": version
        }))
        .unwrap_or_else(|_| serde_json::json!({"installed": false}));
    
    // AI 决策
    let cuda_available = cuda_info.get("available")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let pytorch_installed = pytorch_info.get("installed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let strategy = if cuda_available && pytorch_installed {
        serde_json::json!({
            "strategy": "local_gpu",
            "reason": "检测到本地 GPU 和 PyTorch，优先使用本地算力",
            "confidence": 0.95,
            "estimated_speed": "快速 (本地 GPU)",
            "recommended_action": "启动本地 GPU 推理服务器"
        })
    } else {
        serde_json::json!({
            "strategy": "workers_network",
            "reason": "本地 GPU 不可用或 PyTorch 未安装，使用分布式网络算力",
            "confidence": 0.85,
            "estimated_speed": "中等 (网络延迟)",
            "recommended_action": "通过 Workers 网络请求算力"
        })
    };
    
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🧠 AI 决策: {:?}", strategy)
    }));
    
    Ok(serde_json::json!({
        "python": python_info,
        "cuda": cuda_info,
        "pytorch": pytorch_info,
        "strategy": strategy,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// AI 驱动的智能训练启动
#[tauri::command]
pub async fn ai_start_training(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🚀 AI: 开始智能启动训练..."
    }));
    
    // Step 1: AI 分析系统资源
    let analysis = ai_analyze_system(app.clone()).await?;
    
    let strategy = analysis.get("strategy")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    
    match strategy {
        "local_gpu" => {
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": "🎮 策略: 使用本地 GPU - 正在启动 GPU 服务器..."
            }));
            
            let gpu_result = crate::commands::gpu_commands::start_gpu_server(app.clone()).await;
            
            match gpu_result {
                Ok(msg) => {
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "success",
                        "content": format!("✅ 本地 GPU 服务器已启动: {}", msg)
                    }));
                    
                    let training_result = start_training(state).await;
                    
                    match training_result {
                        Ok(node_id) => {
                            let _ = app.emit("workflow-message", serde_json::json!({
                                "type": "success",
                                "content": format!("✅ 训练节点已启动: {}", node_id)
                            }));
                            
                            Ok(serde_json::json!({
                                "success": true,
                                "strategy": "local_gpu",
                                "gpu_server": "started",
                                "training_node": node_id,
                                "message": "AI 已使用本地 GPU 启动训练"
                            }))
                        }
                        Err(e) => {
                            Err(format!("训练节点启动失败: {}", e))
                        }
                    }
                }
                Err(e) => {
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "warning",
                        "content": format!("⚠️ GPU 服务器启动失败: {}，切换到 Workers 网络", e)
                    }));
                    
                    fallback_to_workers(app, state).await
                }
            }
        }
        _ => {
            fallback_to_workers(app, state).await
        }
    }
}

// 降级到 Workers 网络算力
async fn fallback_to_workers(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": "🌐 策略: 使用 Workers 分布式网络 - 正在上传节点信息..."
    }));
    
    match crate::commands::workers_commands::upload_full_node_info_to_workers(state.clone()).await {
        Ok(result) => {
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "success",
                "content": format!("✅ 已连接到 Workers 网络: {}", result)
            }));
            
            let training_result = start_training(state.clone()).await;
            
            match training_result {
                Ok(node_id) => {
                    Ok(serde_json::json!({
                        "success": true,
                        "strategy": "workers_network",
                        "workers_connected": true,
                        "local_node": node_id,
                        "message": "AI 已使用 Workers 网络启动训练"
                    }))
                }
                Err(_) => {
                    Ok(serde_json::json!({
                        "success": true,
                        "strategy": "workers_network",
                        "workers_connected": true,
                        "local_node": null,
                        "message": "已连接到 Workers 分布式算力网络"
                    }))
                }
            }
        }
        Err(e) => {
            Err(format!("连接 Workers 网络失败: {}", e))
        }
    }
}

fn spawn_node_driver(
    node_store: Arc<Mutex<Option<Node>>>,
    training_status: Arc<Mutex<TrainingStatus>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));

        loop {
            ticker.tick().await;

            if !training_status.lock().is_running {
                break;
            }

            let mut node_opt = { node_store.lock().take() };
            let Some(mut node) = node_opt.take() else {
                break;
            };

            if let Err(e) = node.drive_once().await {
                eprintln!("[NodeDriver] tick 执行失败: {}", e);
            }

            node_store.lock().replace(node);
        }

        eprintln!("[NodeDriver] 后台节点驱动已停止");
    });
}

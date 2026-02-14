//! 任务处理器模块
//!
//! 处理从 Workers 网络接收到的任务消息

use crate::api_client::WorkersMessage;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use anyhow::Result;

/// 任务类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    /// 推理任务
    Inference,
    /// 训练任务
    Training,
    /// 模型下载
    ModelDownload,
    /// 模型切分
    ModelSplit,
    /// 节点连接
    NodeConnect,
    /// 未知
    Unknown,
}

impl TaskType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "inference" => TaskType::Inference,
            "training" => TaskType::Training,
            "model_download" | "model-download" => TaskType::ModelDownload,
            "model_split" | "model-split" => TaskType::ModelSplit,
            "node_connect" | "node-connect" => TaskType::NodeConnect,
            _ => TaskType::Unknown,
        }
    }
}

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// 任务处理器
pub struct TaskHandler;

impl TaskHandler {
    /// 处理接收到的消息
    pub async fn handle_messages(
        messages: Vec<WorkersMessage>,
        app_handle: &tauri::AppHandle,
        state: &AppState,
    ) -> Result<Vec<TaskResult>> {
        let mut results = Vec::new();

        for message in messages {
            let result = Self::process_message(message, app_handle, state).await;
            results.push(result);
            
            // 上报任务结果
            if let Err(e) = report_task_result(&state.api_client, results.last().unwrap().clone()).await {
                log::error!("[TaskHandler] 上报任务结果失败: {}", e);
            }
        }

        Ok(results)
    }

    /// 处理单条消息
    async fn process_message(
        message: WorkersMessage,
        app_handle: &tauri::AppHandle,
        state: &AppState,
    ) -> TaskResult {
        let start_time = std::time::Instant::now();
        let task_id = message.id.clone();
        let task_type = TaskType::from_str(&message.message_type);

        // Emit message received event
        let _ = app_handle.emit("task-received", serde_json::json!({
            "task_id": &task_id,
            "task_type": &message.message_type,
            "from_node": &message.from_node,
        }));

        let result = match task_type {
            TaskType::Inference => {
                Self::handle_inference(message, state).await
            }
            TaskType::Training => {
                Self::handle_training(message, state).await
            }
            TaskType::ModelDownload => {
                Self::handle_model_download(message, state).await
            }
            TaskType::ModelSplit => {
                Self::handle_model_split(message, state).await
            }
            TaskType::NodeConnect => {
                Self::handle_node_connect(message, state).await
            }
            TaskType::Unknown => {
                Ok(serde_json::json!({
                    "message": "Unknown task type received",
                    "task_type": message.message_type,
                }))
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let _ = app_handle.emit("task-completed", serde_json::json!({
                    "task_id": &task_id,
                    "success": true,
                }));

                TaskResult {
                    task_id,
                    success: true,
                    result: Some(output),
                    error: None,
                    execution_time_ms: execution_time,
                }
            }
            Err(e) => {
                let _ = app_handle.emit("task-failed", serde_json::json!({
                    "task_id": &task_id,
                    "error": e.to_string(),
                }));

                TaskResult {
                    task_id,
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time,
                }
            }
        }
    }

    /// 处理推理任务
    async fn handle_inference(
        message: WorkersMessage,
        state: &AppState,
    ) -> Result<serde_json::Value> {
        // 从消息内容中提取推理参数
        let content = message.content;
        
        let model_id = content.get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        let input_data = content.get("input_data")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        log::info!("[TaskHandler] 处理推理任务: model_id={}", model_id);

        // 获取本地节点信息（如果可用）
        let node_info = {
            let node_guard = state.node.lock();
            node_guard.as_ref().map(|n| n.comms.node_id().to_string())
        };

        // 模拟推理处理
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 返回推理结果
        Ok(serde_json::json!({
            "task_type": "inference",
            "task_id": message.id,
            "model_id": model_id,
            "input_data": input_data,
            "output": {
                "result": "inference_completed",
                "node_id": node_info.unwrap_or_else(|| "unknown".to_string())
            },
            "status": "completed",
            "execution_time_ms": 100
        }))
    }

    /// 处理训练任务
    async fn handle_training(
        message: WorkersMessage,
        state: &AppState,
    ) -> Result<serde_json::Value> {
        let content = message.content;
        
        let model_id = content.get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        let epochs = content.get("epochs")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as u32;

        log::info!("[TaskHandler] 处理训练任务: model_id={}, epochs={}", model_id, epochs);

        // 获取本地节点信息（如果可用）
        let node_info = {
            let node_guard = state.node.lock();
            node_guard.as_ref().map(|n| {
                let node_id = n.comms.node_id().to_string();
                let tick = n.tick_counter;
                (node_id, tick)
            })
        };

        // 模拟训练处理（每个 epoch 约 50ms）
        let training_time = (epochs as u64) * 50;
        tokio::time::sleep(tokio::time::Duration::from_millis(training_time)).await;

        // 模拟训练结果
        let final_loss = 1.0 / (1.0 + epochs as f64 * 0.1);
        let final_accuracy = 1.0 - (final_loss * 0.5);

        Ok(serde_json::json!({
            "task_type": "training",
            "task_id": message.id,
            "model_id": model_id,
            "epochs": epochs,
            "status": "completed",
            "final_loss": final_loss,
            "final_accuracy": final_accuracy,
            "node_info": node_info.map(|(id, tick)| serde_json::json!({
                "node_id": id,
                "tick_counter": tick
            })).unwrap_or(serde_json::Value::Null),
            "execution_time_ms": training_time
        }))
    }

    /// 处理模型下载任务
    async fn handle_model_download(
        message: WorkersMessage,
        state: &AppState,
    ) -> Result<serde_json::Value> {
        let content = message.content;
        
        let model_id = content.get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        let source_url = content.get("source_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        log::info!("[TaskHandler] 处理模型下载任务: model_id={}, source={}", model_id, source_url);

        // 获取模型配置
        let model_config = {
            let models = state.available_models.lock();
            models.iter().find(|m| m.id == model_id).cloned()
        };

        let local_path = if !source_url.is_empty() {
            // 如果提供了 URL，尝试下载模型
            let model_dir = std::path::Path::new("./models");
            let _ = std::fs::create_dir_all(model_dir);
            let model_path = model_dir.join(format!("{}.bin", model_id));

            if source_url.starts_with("http://") || source_url.starts_with("https://") {
                // 模拟下载（实际实现需要真实的 HTTP 客户端）
                log::info!("[TaskHandler] 模拟下载模型 from: {}", source_url);
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }

            model_path.to_string_lossy().to_string()
        } else {
            // 使用本地模型路径
            format!("./models/{}", model_id)
        };

        Ok(serde_json::json!({
            "task_type": "model_download",
            "task_id": message.id,
            "model_id": model_id,
            "model_config": model_config,
            "local_path": local_path,
            "status": "completed",
            "execution_time_ms": 200
        }))
    }

    /// 处理模型切分任务
    async fn handle_model_split(
        message: WorkersMessage,
        state: &AppState,
    ) -> Result<serde_json::Value> {
        let content = message.content;
        
        let model_id = content.get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        let num_splits = content.get("num_splits")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        // 获取模型维度
        let model_dim = {
            let models = state.available_models.lock();
            models.iter()
                .find(|m| m.id == model_id)
                .map(|m| m.dimensions)
                .unwrap_or(512)
        };

        log::info!("[TaskHandler] 处理模型切分任务: model_id={}, splits={}, dim={}", 
            model_id, num_splits, model_dim);

        // 模拟模型切分处理
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // 生成切分计划
        let mut splits = Vec::new();
        let layers_per_split = 100 / num_splits;
        let dim_per_split = model_dim / num_splits;

        for i in 0..num_splits {
            splits.push(serde_json::json!({
                "split_id": i,
                "layer_start": i * layers_per_split,
                "layer_end": (i + 1) * layers_per_split,
                "dim_start": i * dim_per_split,
                "dim_end": (i + 1) * dim_per_split,
                "parameter_count": (model_dim * dim_per_split),
                "memory_requirement_mb": (model_dim * dim_per_split * 4) / (1024 * 1024), // 假设 float32
            }));
        }

        Ok(serde_json::json!({
            "task_type": "model_split",
            "task_id": message.id,
            "model_id": model_id,
            "num_splits": num_splits,
            "model_dim": model_dim,
            "splits": splits,
            "status": "completed",
            "execution_time_ms": 150
        }))
    }

    /// 处理节点连接任务
    async fn handle_node_connect(
        message: WorkersMessage,
        state: &AppState,
    ) -> Result<serde_json::Value> {
        let content = message.content;
        
        let target_node = content.get("target_node")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        log::info!("[TaskHandler] 处理节点连接任务: target={}", target_node);

        // 尝试连接到目标节点
        let mut node_opt = { state.node.lock().take() };
        
        if let Some(mut node) = node_opt.take() {
            let connect_result = node.comms.connect(target_node.to_string()).await;
            state.node.lock().replace(node);

            match connect_result {
                Ok(_) => {
                    Ok(serde_json::json!({
                        "task_type": "node_connect",
                        "target_node": target_node,
                        "status": "connected",
                        "message": "Successfully connected to target node",
                    }))
                }
                Err(e) => {
                    Ok(serde_json::json!({
                        "task_type": "node_connect",
                        "target_node": target_node,
                        "status": "failed",
                        "error": e.to_string(),
                    }))
                }
            }
        } else {
            Ok(serde_json::json!({
                "task_type": "node_connect",
                "target_node": target_node,
                "status": "failed",
                "error": "Local node not running",
            }))
        }
    }
}

/// 向 Workers 上报任务结果
pub async fn report_task_result(
    api_client: &crate::api_client::WorkersApiClient,
    task_result: TaskResult,
) -> Result<()> {
    api_client.report_task_result(
        task_result.task_id.clone(),
        task_result.success,
        task_result.result.clone(),
        task_result.error.clone(),
        task_result.execution_time_ms,
    ).await?;

    log::info!("[TaskHandler] 上报任务结果: task_id={}, success={}", 
        task_result.task_id, task_result.success);

    Ok(())
}

use crate::state::{AppState, TrainingStatus};
use williw::Node;
use williw::config::AppConfig;
use tauri::State;

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

/// Get training statistics
#[tauri::command]
pub fn get_training_stats(
    state: State<'_, AppState>
) -> TrainingStatus {
    state.training_status.lock().clone()
}
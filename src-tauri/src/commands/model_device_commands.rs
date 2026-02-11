use crate::state::{AppState, ModelConfig, DeviceInfo, AppSettings};
use tauri::State;

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
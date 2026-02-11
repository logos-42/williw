use crate::state::{AppState, ApiKeyEntry};
use tauri::State;
use uuid::Uuid;
use chrono::Utc;

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
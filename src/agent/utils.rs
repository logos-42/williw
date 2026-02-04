use std::path::PathBuf;
#[cfg(feature = "tauri")]
use tauri::{AppHandle, Manager};

/// Get application data directory
#[cfg(feature = "tauri")]
pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {}", e))
}

/// Get application data directory (fallback when tauri is not available)
#[cfg(not(feature = "tauri"))]
pub fn app_data_dir(_app: &()) -> Result<PathBuf, String> {
    // Return a default path when tauri is not available
    std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))
}

/// Normalize base URL by removing trailing slashes
pub fn normalize_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

/// Default IPFS API URL
pub fn default_ipfs_api_url() -> String {
    "http://127.0.0.1:5001".to_string()
}

/// Default IPFS Gateway URL
pub fn default_ipfs_gateway_url() -> String {
    "http://127.0.0.1:8080".to_string()
}


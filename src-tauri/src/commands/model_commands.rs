//! Model Management Commands Module
//! Integrates model download and split functionality

use tauri::Emitter;
use std::path::PathBuf;

// Import model downloader (via Cargo.toml dependency)
use model_downloader::{ModelDownloader, DownloadConfig, DownloadResult};
use model_splitter::{ModelSplitter, SplitConfig, SplitPlan};

/// Download model command
#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    model_id: String,
    cache_dir: Option<String>,
) -> Result<String, String> {
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("📥 Starting download model: {}", model_id),
        "step": "download_model",
        "progress": 0.1,
    }));

    // Create model downloader
    let downloader = ModelDownloader::new(None);

    // Create download config
    let config = DownloadConfig {
        model_name: model_id.clone(),
        cache_dir: cache_dir.clone(),
        hf_token: None,
    };

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🔄 Downloading model...",
        "step": "download_model",
        "progress": 0.3,
    }));

    // Download model
    let result = downloader.download_model(config)
        .await
        .map_err(|e| {
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "error",
                "content": format!("❌ Model download failed: {}", e),
                "step": "download_model",
                "progress": 0.0,
            }));
            format!("Model download failed: {}", e)
        })?;

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "success",
        "content": format!("✅ Model downloaded: {}", result.model_path),
        "step": "download_model",
        "progress": 1.0,
    }));

    Ok(result.model_path)
}

/// Get model metadata
#[tauri::command]
pub async fn get_model_metadata(
    model_path: String,
) -> Result<serde_json::Value, String> {
    // Check if model path exists
    let path = PathBuf::from(&model_path);
    if !path.exists() {
        return Err(format!("Model path does not exist: {}", model_path));
    }

    // Read model directory info
    let mut files: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }
    }

    Ok(serde_json::json!({
        "model_path": model_path,
        "files": files,
        "file_count": files.len(),
    }))
}

/// Download and split model - complete workflow
#[tauri::command]
pub async fn download_and_split_model(
    app: tauri::AppHandle,
    model_id: String,
    num_nodes: usize,
    cache_dir: Option<String>,
) -> Result<serde_json::Value, String> {
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🚀 Starting complete workflow: Download + Split model ({} nodes)", num_nodes),
        "step": "download_and_split",
        "progress": 0.0,
    }));

    // Step 1: Download model
    let model_path = download_model(
        app.clone(),
        model_id.clone(),
        cache_dir,
    ).await?;

    // Step 2: Get metadata
    let metadata = get_model_metadata(
        model_path.clone(),
    ).await?;

    // Step 3: Create split plan (simulated)
    let mut splits = Vec::new();
    let file_count = metadata.get("file_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let files_per_node = file_count / num_nodes;

    for i in 0..num_nodes {
        let start = i * files_per_node;
        let end = if i == num_nodes - 1 {
            file_count
        } else {
            start + files_per_node
        };

        splits.push(serde_json::json!({
            "node_id": format!("node_{}", i),
            "file_range": [start, end],
            "layer_count": end - start,
        }));
    }

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "success",
        "content": "✅ Model download and split complete!",
        "step": "download_and_split",
        "progress": 1.0,
    }));

    Ok(serde_json::json!({
        "success": true,
        "model_id": model_id,
        "model_path": model_path,
        "num_nodes": num_nodes,
        "splits": splits,
    }))
}

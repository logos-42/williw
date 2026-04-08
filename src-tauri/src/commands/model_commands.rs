//! Model Management Commands Module
//! Integrates model download and split functionality

use tauri::Emitter;
use std::path::PathBuf;
use std::time::Instant;

// Import model downloader (via Cargo.toml dependency)
use model_downloader::{ModelDownloader, DownloadConfig, DownloadResult};

// Import progress event
use crate::events::ModelDownloadProgress;

/// Download model with real-time progress, chunked download, and resume support
#[tauri::command]
pub async fn download_model_with_progress(
    app: tauri::AppHandle,
    model_id: String,
    cache_dir: Option<String>,
) -> Result<String, String> {
    use reqwest::Client;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("📥 开始下载模型：{}", model_id),
        "step": "download_model",
        "progress": 0.1,
    }));

    // Determine cache directory
    let cache_dir = cache_dir.unwrap_or_else(|| {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("williw")
            .join("models")
            .to_string_lossy()
            .to_string()
    });

    // Create cache directory if not exists
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("创建缓存目录失败：{}", e))?;

    // For Hugging Face models, construct download URL
    // Format: https://huggingface.co/{model_id}/resolve/main/{file}
    // For simplicity, we'll download a single file (e.g., pytorch_model.bin)
    // In production, you'd need to parse the model config to get all files
    
    let model_file_name = "pytorch_model.bin";
    let model_path = PathBuf::from(&cache_dir).join(&model_id.replace("/", "_")).join(model_file_name);
    
    // Check for partial download (resume support)
    let mut start_pos = 0u64;
    if model_path.exists() {
        let metadata = std::fs::metadata(&model_path).map_err(|e| e.to_string())?;
        start_pos = metadata.len();
        if start_pos > 0 {
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "info",
                "content": format!("🔄 发现未完成的下载，将从 {} 字节处继续", start_pos),
                "step": "download_model",
                "progress": 0.1,
            }));
        }
    }

    // Create parent directories
    if let Some(parent) = model_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{}", e))?;
    }

    // Build download URL (using Hugging Face as example)
    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        model_id, model_file_name
    );

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🌐 下载源：{}", download_url),
        "step": "download_model",
        "progress": 0.2,
    }));

    // Create HTTP client
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(3600)) // 1 hour timeout for large models
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    // Build request with Range header for resume support
    let mut request_builder = client.get(&download_url);
    if start_pos > 0 {
        request_builder = request_builder.header("Range", format!("bytes={}-", start_pos));
    }

    // Send request
    let mut response = request_builder
        .send()
        .await
        .map_err(|e| format!("请求失败：{}", e))?;

    // Check response status
    if !response.status().is_success() && response.status() != 206 {
        return Err(format!("下载失败：HTTP {}", response.status()));
    }

    // Get total size (from Content-Range header or Content-Length)
    let total_bytes = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(|len| len + start_pos);

    // Open file for writing
    let mut file = if start_pos > 0 {
        OpenOptions::new()
            .write(true)
            .append(true)
            .open(&model_path)
            .map_err(|e| format!("打开文件失败：{}", e))?
    } else {
        File::create(&model_path)
            .map_err(|e| format!("创建文件失败：{}", e))?
    };

    let mut downloaded_bytes = start_pos;
    let start_time = Instant::now();
    let mut last_progress_emit = Instant::now();
    let progress_emit_interval = std::time::Duration::from_millis(500); // Emit progress every 500ms

    // Stream download in chunks
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("下载数据块失败：{}", e))?
    {
        file.write_all(&chunk).map_err(|e| format!("写入文件失败：{}", e))?;
        downloaded_bytes += chunk.len() as u64;

        // Emit progress periodically
        if last_progress_emit.elapsed() >= progress_emit_interval {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (downloaded_bytes - start_pos) as f64 / elapsed
            } else {
                0.0
            };

            let progress_event = ModelDownloadProgress {
                model_id: model_id.clone(),
                downloaded_bytes,
                total_bytes,
                speed_bytes_per_sec: speed,
            };

            let _ = app.emit("model-download-progress", progress_event);

            // Also emit workflow-message for UI
            let progress_ratio = total_bytes
                .map(|total| downloaded_bytes as f64 / total as f64)
                .unwrap_or(0.5);

            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": format!(
                    "📥 下载中：{:.1}MB / {:.1}MB ({:.1}%), 速度：{:.2} MB/s",
                    downloaded_bytes as f64 / 1024.0 / 1024.0,
                    total_bytes.unwrap_or(0) as f64 / 1024.0 / 1024.0,
                    progress_ratio * 100.0,
                    speed / 1024.0 / 1024.0
                ),
                "step": "download_model",
                "progress": progress_ratio * 0.8, // Scale to 80% for download phase
            }));

            last_progress_emit = Instant::now();
        }
    }

    file.flush().map_err(|e| format!("刷新文件失败：{}", e))?;

    let final_elapsed = start_time.elapsed().as_secs_f64();
    let final_speed = if final_elapsed > 0.0 {
        (downloaded_bytes - start_pos) as f64 / final_elapsed
    } else {
        0.0
    };

    // Emit final progress
    let _ = app.emit("model-download-progress", ModelDownloadProgress {
        model_id: model_id.clone(),
        downloaded_bytes,
        total_bytes,
        speed_bytes_per_sec: final_speed,
    });

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "success",
        "content": format!(
            "✅ 模型下载完成：{}\n总大小：{:.1}MB, 平均速度：{:.2} MB/s",
            model_path.display(),
            downloaded_bytes as f64 / 1024.0 / 1024.0,
            final_speed / 1024.0 / 1024.0
        ),
        "step": "download_model",
        "progress": 1.0,
    }));

    Ok(model_path.to_string_lossy().to_string())
}

/// Download model command (legacy, without progress)
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
    let result = downloader.download_model(config, |_| {}).await
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

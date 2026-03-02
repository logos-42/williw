/**
 * Rust 模块 1: 下载模型
 * 从 Hugging Face 下载模型文件
 * 支持：分块下载、断点续传、实时进度
 */
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub model_name: String,
    pub cache_dir: Option<String>,
    pub hf_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub model_path: String,
    pub files_downloaded: Vec<String>,
    pub total_size_mb: f64,
    pub skipped_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub speed_mbps: f64,
}

pub struct ModelDownloader {
    client: Client,
    hf_token: Option<String>,
}

impl ModelDownloader {
    pub fn new(hf_token: Option<String>) -> Self {
        let client = Client::builder()
            .user_agent("model-downloader/0.1.0")
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            hf_token,
        }
    }

    /// 下载模型文件（带进度回调）
    pub async fn download_model<F>(&self, config: DownloadConfig, mut progress_callback: F) -> Result<DownloadResult>
    where
        F: FnMut(DownloadProgress),
    {
        let model_name = config.model_name;
        let cache_dir = config.cache_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./models_cache").join(model_name.replace("/", "_")));
        
        fs::create_dir_all(&cache_dir)
            .await
            .context("Failed to create cache directory")?;

        println!("下载模型: {} 到 {}", model_name, cache_dir.display());

        // 构建 Hugging Face API URL
        let api_base = "https://huggingface.co/api/models";
        let model_url = format!("{}/{}", api_base, model_name);

        // 获取模型文件列表
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(token) = &self.hf_token {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }

        let response = self.client
            .get(&model_url)
            .headers(headers.clone())
            .send()
            .await
            .context("Failed to fetch model info")?;

        let model_info: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse model info")?;

        // 提取需要下载的文件
        let files = model_info["siblings"]
            .as_array()
            .context("No files found in model info")?;

        let mut files_to_download = Vec::new();
        for file in files {
            let file_name = file["rfilename"]
                .as_str()
                .context("Invalid file name")?;
            
            // 优先下载关键文件
            if file_name.ends_with(".safetensors") 
                || file_name.ends_with(".bin")
                || file_name == "config.json"
                || file_name == "tokenizer.json"
                || file_name == "tokenizer_config.json" {
                files_to_download.push(file_name.to_string());
            }
        }

        // 下载文件
        let mut downloaded_files = Vec::new();
        let mut skipped_files = Vec::new();
        let mut total_size = 0u64;

        for file_name in &files_to_download {
            let file_url = format!(
                "https://huggingface.co/{}/resolve/main/{}",
                model_name, file_name
            );

            let file_path = cache_dir.join(file_name);
            
            // 先获取文件大小
            let head_response = self.client
                .head(&file_url)
                .headers(headers.clone())
                .send()
                .await
                .ok()
                .and_then(|r| r.headers().get("content-length").cloned())
                .and_then(|v| v.to_str().ok().and_then(|s| s.parse().ok()))
                .unwrap_or(0);

            // 如果文件已存在，检查大小是否匹配
            if file_path.exists() {
                let existing_size = fs::metadata(&file_path).await.map(|m| m.len()).unwrap_or(0);
                if existing_size == head_response && head_response > 0 {
                    // 文件完整，跳过
                    total_size += existing_size;
                    skipped_files.push(file_name.clone());
                    println!("  ✓ 文件已完整，跳过: {} ({} MB)", file_name, existing_size / (1024 * 1024));
                    continue;
                } else if existing_size > 0 {
                    // 文件不完整，需要断点续传
                    println!("  ↻ 断点续传: {} (已有 {} MB)", file_name, existing_size / (1024 * 1024));
                }
            }

            println!("  下载: {}", file_name);

            // 分块下载（使用 Range header）
            let mut downloaded: u64 = 0;
            let chunk_size: u64 = 1024 * 1024; // 1MB chunks
            let start_time = std::time::Instant::now();

            // 确保父目录存在
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // 打开文件（追加模式）
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .await
                .context(format!("Failed to open file {}", file_path.display()))?;

            // 获取已下载的大小
            let current_size = file.metadata().await.map(|m| m.len()).unwrap_or(0);
            downloaded = current_size;

            // 分块下载
            loop {
                let start = downloaded;
                let end = std::cmp::min(downloaded + chunk_size - 1, head_response.saturating_sub(1));
                
                if start >= head_response {
                    break;
                }

                let mut req = self.client.get(&file_url);
                req = req.header("Range", format!("bytes={}-{}", start, end));
                if let Some(token) = &self.hf_token {
                    req = req.header("Authorization", format!("Bearer {}", token));
                }

                let response = match req.send().await {
                    Ok(r) => r,
                    Err(e) => {
                        println!("  ⚠️ 下载chunk失败，重试中: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        continue;
                    }
                };

                let bytes = match response.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        println!("  ⚠️ 读取chunk失败，重试中: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                if let Err(e) = file.write_all(&bytes).await {
                    println!("  ⚠️ 写入失败，重试中: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }

                downloaded += bytes.len() as u64;
                total_size += bytes.len() as u64;

                // 计算速度
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 { (downloaded as f64 / (1024.0 * 1024.0)) / elapsed } else { 0.0 };

                // 报告进度
                progress_callback(DownloadProgress {
                    file_name: file_name.clone(),
                    downloaded_bytes: downloaded,
                    total_bytes: head_response,
                    percentage: if head_response > 0 { (downloaded as f64 / head_response as f64) * 100.0 } else { 0.0 },
                    speed_mbps: speed,
                });

                // 如果下载完成或到达文件末尾
                if downloaded >= head_response || bytes.len() < (chunk_size as usize) {
                    break;
                }
            }

            downloaded_files.push(file_name.clone());
            println!("  ✓ 完成: {} ({} MB)", file_name, downloaded / (1024 * 1024));
        }

        Ok(DownloadResult {
            model_path: cache_dir.to_string_lossy().to_string(),
            files_downloaded: downloaded_files,
            total_size_mb: total_size as f64 / (1024.0 * 1024.0),
            skipped_files,
        })
    }

    /// 简化版下载（无进度回调，兼容旧接口）
    pub async fn download_model_simple(&self, config: DownloadConfig) -> Result<DownloadResult> {
        self.download_model(config, |_| {}).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download_model() {
        let downloader = ModelDownloader::new(None);
        let config = DownloadConfig {
            model_name: "gpt2".to_string(),
            cache_dir: Some("./test_cache".to_string()),
            hf_token: None,
        };

        let result = downloader.download_model_simple(config).await;
        assert!(result.is_ok());
    }
}

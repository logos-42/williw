/**
 * Rust 模块 1: 下载模型
 * 从 Hugging Face 下载模型文件
 */
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
}

pub struct ModelDownloader {
    client: Client,
    hf_token: Option<String>,
}

impl ModelDownloader {
    pub fn new(hf_token: Option<String>) -> Self {
        let client = Client::builder()
            .user_agent("model-downloader/0.1.0")
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            hf_token,
        }
    }

    /// 下载模型文件
    pub async fn download_model(&self, config: DownloadConfig) -> Result<DownloadResult> {
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

        // 提取需要下载的文件（优先下载 safetensors 和 config.json）
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
        let mut total_size = 0u64;

        for file_name in &files_to_download {
            let file_url = format!(
                "https://huggingface.co/{}/resolve/main/{}",
                model_name, file_name
            );

            let file_path = cache_dir.join(file_name);
            
            // 如果文件已存在，跳过
            if file_path.exists() {
                let metadata = fs::metadata(&file_path).await?;
                total_size += metadata.len();
                downloaded_files.push(file_name.clone());
                println!("  文件已存在，跳过: {}", file_name);
                continue;
            }

            println!("  下载: {}", file_name);

            let response = self.client
                .get(&file_url)
                .headers(headers.clone())
                .send()
                .await
                .context(format!("Failed to download {}", file_name))?;

            let content = response
                .bytes()
                .await
                .context(format!("Failed to read content of {}", file_name))?;

            // 确保父目录存在
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // 写入文件
            let mut file = fs::File::create(&file_path)
                .await
                .context(format!("Failed to create file {}", file_path.display()))?;
            
            file.write_all(&content)
                .await
                .context(format!("Failed to write file {}", file_path.display()))?;

            total_size += content.len() as u64;
            downloaded_files.push(file_name.clone());
        }

        Ok(DownloadResult {
            model_path: cache_dir.to_string_lossy().to_string(),
            files_downloaded: downloaded_files,
            total_size_mb: total_size as f64 / (1024.0 * 1024.0),
        })
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

        let result = downloader.download_model(config).await;
        assert!(result.is_ok());
    }
}

/**
 * Rust 模块 3: 上传元数据到 Hugging Face
 * 将元数据 JSON 文件上传到 Hugging Face 公共仓库
 */
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    pub metadata_file: String,
    pub repo_id: String,
    pub hf_token: String,
    pub commit_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub repo: String,
    pub filename: String,
    pub url: String,
    pub commit_url: String,
}

pub struct MetadataUploader {
    client: Client,
}

impl MetadataUploader {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("metadata-uploader/0.1.0")
            .build()
            .expect("Failed to create HTTP client");
        
        Self { client }
    }

    /// 上传元数据到 Hugging Face
    pub async fn upload_metadata(&self, config: UploadConfig) -> Result<UploadResult> {
        let metadata_path = Path::new(&config.metadata_file);
        
        if !metadata_path.exists() {
            anyhow::bail!("元数据文件不存在: {}", config.metadata_file);
        }

        // 读取文件内容
        let file_content = fs::read(metadata_path)
            .await
            .context("Failed to read metadata file")?;

        // 提取文件名
        let filename = metadata_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid filename")?
            .to_string();

        println!("上传元数据到 Hugging Face: {}/{}", config.repo_id, filename);

        // 构建 Hugging Face API URL
        let upload_url = format!(
            "https://huggingface.co/api/models/{}/upload",
            config.repo_id
        );

        // 构建 multipart form
        let form = reqwest::multipart::Form::new()
            .text("path", filename.clone())
            .text("commitMessage", config.commit_message.unwrap_or_else(|| {
                format!("Upload metadata: {}", filename)
            }))
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_content)
                    .file_name(filename.clone())
                    .mime_str("application/json")?,
            );

        // 构建请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", config.hf_token).parse().unwrap(),
        );

        // 发送请求
        let response = self.client
            .post(&upload_url)
            .headers(headers)
            .multipart(form)
            .send()
            .await
            .context("Failed to upload metadata")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Upload failed with status {}: {}", status, body);
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse upload response")?;

        let commit_url = result["commitUrl"]
            .as_str()
            .context("No commitUrl in response")?
            .to_string();

        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            config.repo_id, filename
        );

        println!("✓ 上传成功: {}", url);

        Ok(UploadResult {
            repo: config.repo_id,
            filename,
            url,
            commit_url,
        })
    }

    /// 检查元数据是否已存在
    pub async fn check_metadata_exists(
        &self,
        repo_id: &str,
        filename: &str,
        hf_token: Option<&str>,
    ) -> Result<bool> {
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo_id, filename
        );

        let mut request = self.client.head(&url);

        if let Some(token) = hf_token {
            request = request.header(
                "Authorization",
                format!("Bearer {}", token),
            );
        }

        let response = request.send().await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_metadata_exists() {
        let uploader = MetadataUploader::new();
        let exists = uploader
            .check_metadata_exists("gpt2", "config.json", None)
            .await;
        assert!(exists.is_ok());
    }
}

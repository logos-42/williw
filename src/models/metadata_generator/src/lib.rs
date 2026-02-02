/**
 * Rust 模块 2: 提取 state_dict 生成元数据
 * 调用 Python 代码提取 state_dict 并生成元数据 JSON
 */
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerMetadata {
    pub name: String,
    pub shape: Vec<usize>,
    pub num_params: usize,
    pub compute_required: f64,
    pub layer_type: String,
    pub dtype: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_name: String,
    pub model_type: String,
    pub batch_size: usize,
    pub sequence_length: usize,
    pub layers: Vec<LayerMetadata>,
    pub total_compute: f64,
    pub total_layers: usize,
    pub generated_at: f64,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub model_name: String,
    pub model_path: String,
    pub batch_size: usize,
    pub sequence_length: usize,
    pub node_id: Option<String>,
}

pub struct MetadataGenerator;

impl Default for MetadataGenerator {
    fn default() -> Self {
        Self
    }
}

impl MetadataGenerator {
    pub fn new() -> Self {
        Self
    }
    /// 生成元数据（调用 Python 脚本）
    /// 
    /// 注意：这个函数需要调用 Python 代码来提取 state_dict
    /// 实际实现中，可以使用 pyo3 直接调用 Python，或者
    /// 通过命令行调用 Python 脚本
    pub async fn generate_metadata(
        &self,
        config: MetadataConfig,
    ) -> Result<ModelMetadata> {
        // 方案1: 通过命令行调用 Python 脚本
        let python_script = include_str!("../scripts/generate_metadata.py");
        
        // 将 Python 脚本保存到临时文件
        let script_path = std::env::temp_dir().join("generate_metadata.py");
        tokio::fs::write(&script_path, python_script).await?;
        
        // 构建 Python 命令
        let output = tokio::process::Command::new("python3")
            .arg(&script_path)
            .arg("--model-name")
            .arg(&config.model_name)
            .arg("--model-path")
            .arg(&config.model_path)
            .arg("--batch-size")
            .arg(config.batch_size.to_string())
            .arg("--sequence-length")
            .arg(config.sequence_length.to_string())
            .output()
            .await
            .context("Failed to execute Python script")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Python script failed: {}", stderr);
        }

        // 解析 JSON 输出
        let stdout = String::from_utf8_lossy(&output.stdout);
        let metadata: ModelMetadata = serde_json::from_str(&stdout)
            .context("Failed to parse metadata JSON")?;

        Ok(metadata)
    }

    /// 保存元数据到文件
    pub async fn save_metadata(
        &self,
        metadata: &ModelMetadata,
        output_path: &str,
    ) -> Result<()> {
        let json = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize metadata")?;
        
        tokio::fs::write(output_path, json)
            .await
            .context("Failed to write metadata file")?;
        
        Ok(())
    }

    /// 估算单层算力需求（Rust 实现，用于验证）
    pub fn estimate_layer_compute(
        &self,
        layer_name: &str,
        num_params: usize,
        layer_type: &str,
        batch_size: usize,
        sequence_length: usize,
        model_type: &str,
    ) -> f64 {
        let operation_costs: HashMap<&str, f64> = [
            ("conv2d", 2.0),
            ("linear", 2.0),
            ("attention", 4.0),
            ("layernorm", 1.0),
            ("embedding", 0.5),
            ("activation", 0.1),
            ("pooling", 0.2),
        ]
        .iter()
        .cloned()
        .collect();

        let cost_per_param = operation_costs
            .get(layer_type)
            .copied()
            .unwrap_or(2.0);

        let mut layer_compute = num_params as f64 * cost_per_param * batch_size as f64;

        if model_type == "transformer" {
            layer_compute *= sequence_length as f64 / 512.0;
        }

        layer_compute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_layer_compute() {
        let generator = MetadataGenerator;
        let compute = generator.estimate_layer_compute(
            "transformer.h.0.attn.q_proj.weight",
            16777216,
            "attention",
            1,
            512,
            "transformer",
        );
        assert!(compute > 0.0);
    }
}

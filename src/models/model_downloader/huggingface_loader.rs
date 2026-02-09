use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn, error};

/// Hugging Face 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HFModelConfig {
    pub model_id: String,
    pub revision: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub token: Option<String>,
}

/// 模型层信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLayer {
    pub name: String,
    pub layer_type: String,
    pub shape: Vec<usize>,
    pub parameters: Vec<f32>, // 改为 Vec<f32> 以支持序列化
}

/// 拆分后的模型部分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPartition {
    pub part_id: usize,
    pub layers: Vec<ModelLayer>,
    pub total_params: usize,
}

/// Llama 3.2 1B 模型加载器
pub struct LlamaModelLoader {
    config: HFModelConfig,
    cache_dir: PathBuf,
}

impl LlamaModelLoader {
    /// 创建新的模型加载器
    pub fn new(config: HFModelConfig) -> Result<Self> {
        let cache_dir = config.cache_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("./models/huggingface"));
        
        // 确保缓存目录存在
        fs::create_dir_all(&cache_dir)?;
        
        Ok(Self {
            config,
            cache_dir,
        })
    }

    /// 下载 Llama 3.2 1B 模型
    pub async fn download_model(&self) -> Result<PathBuf> {
        let model_path = self.cache_dir.join(&self.config.model_id);
        
        if model_path.exists() {
            info!("模型已存在于: {:?}", model_path);
            return Ok(model_path);
        }

        info!("开始下载 Llama 3.2 1B 模型...");
        
        // 使用 huggingface-hub 下载模型
        let output = Command::new("python")
            .arg("-c")
            .arg(&format!(
                r#"
import os
from huggingface_hub import snapshot_download
import json

try:
    model_path = snapshot_download(
        repo_id="{}",
        revision="{}",
        cache_dir="{}",
        token="{}",
        resume_download=True
    )
    print(f"DOWNLOAD_SUCCESS:{model_path}")
except Exception as e:
    print(f"DOWNLOAD_ERROR:{str(e)}")
    exit(1)
"#,
                self.config.model_id,
                self.config.revision.as_deref().unwrap_or("main"),
                self.cache_dir.to_string_lossy(),
                self.config.token.as_deref().unwrap_or("")
            ))
            .output()
            .await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("下载模型失败: {}", error_msg));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        if let Some(success_line) = output_str.lines().find(|line| line.starts_with("DOWNLOAD_SUCCESS:")) {
            let path = success_line.strip_prefix("DOWNLOAD_SUCCESS:").unwrap();
            info!("模型下载成功: {}", path);
            return Ok(PathBuf::from(path));
        } else {
            return Err(anyhow!("下载完成但未找到成功标记"));
        }
    }

    /// 加载模型参数
    pub async fn load_model_parameters(&self, model_path: &Path) -> Result<Vec<ModelLayer>> {
        info!("加载模型参数从: {:?}", model_path);
        
        // 查找模型权重文件
        let py_file = model_path.join("pytorch_model.bin");
        let safetensors_file = model_path.join("model.safetensors");
        
        let layers = if safetensors_file.exists() {
            self.load_safetensors(&safetensors_file).await?
        } else if py_file.exists() {
            self.load_pytorch_model(&py_file).await?
        } else {
            return Err(anyhow!("未找到模型权重文件"));
        };

        info!("成功加载 {} 个层", layers.len());
        Ok(layers)
    }

    /// 从 safetensors 文件加载参数
    async fn load_safetensors(&self, file_path: &Path) -> Result<Vec<ModelLayer>> {
        let output = Command::new("python")
            .arg("-c")
            .arg(&format!(
                r#"
import torch
import json
from safetensors import safe_open
import numpy as np

try:
    layers = []
    with safe_open("{}", framework="pt") as f:
        for key in f.keys():
            tensor = f.get_tensor(key)
            # 将张量转换为扁平的一维数组
            flat_params = tensor.flatten().numpy().astype(np.float32)
            
            layer_info = {{
                "name": key,
                "layer_type": str(tensor.dtype),
                "shape": list(tensor.shape),
                "parameters": flat_params.tolist()
            }}
            layers.append(layer_info)
    
    # 输出为 JSON 格式
    print(json.dumps({{"layers": layers}}))
except Exception as e:
    print(f"LOAD_ERROR:{{str(e)}}")
    exit(1)
"#,
                file_path.to_string_lossy()
            ))
            .output()
            .await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("加载 safetensors 失败: {}", error_msg));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // 解析 JSON 输出
        let json_data: serde_json::Value = serde_json::from_str(&output_str)?;
        let layers_json = json_data["layers"].as_array()
            .ok_or_else(|| anyhow!("无效的层数据格式"))?;

        let mut layers = Vec::new();
        for layer_json in layers_json {
            let layer: ModelLayer = serde_json::from_value(layer_json.clone())?;
            layers.push(layer);
        }

        Ok(layers)
    }

    /// 从 PyTorch 模型文件加载参数
    async fn load_pytorch_model(&self, file_path: &Path) -> Result<Vec<ModelLayer>> {
        let output = Command::new("python")
            .arg("-c")
            .arg(&format!(
                r#"
import torch
import json
import numpy as np

try:
    state_dict = torch.load("{}", map_location="cpu")
    layers = []
    
    for key, tensor in state_dict.items():
        if isinstance(tensor, torch.Tensor):
            # 将张量转换为扁平的一维数组
            flat_params = tensor.flatten().numpy().astype(np.float32)
            
            layer_info = {{
                "name": key,
                "layer_type": str(tensor.dtype),
                "shape": list(tensor.shape),
                "parameters": flat_params.tolist()
            }}
            layers.append(layer_info)
    
    # 输出为 JSON 格式
    print(json.dumps({{"layers": layers}}))
except Exception as e:
    print(f"LOAD_ERROR:{{str(e)}}")
    exit(1)
"#,
                file_path.to_string_lossy()
            ))
            .output()
            .await?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("加载 PyTorch 模型失败: {}", error_msg));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // 解析 JSON 输出
        let json_data: serde_json::Value = serde_json::from_str(&output_str)?;
        let layers_json = json_data["layers"].as_array()
            .ok_or_else(|| anyhow!("无效的层数据格式"))?;

        let mut layers = Vec::new();
        for layer_json in layers_json {
            let layer: ModelLayer = serde_json::from_value(layer_json.clone())?;
            layers.push(layer);
        }

        Ok(layers)
    }

    /// 按层拆分模型为两部分
    pub fn split_model_layers(&self, layers: Vec<ModelLayer>) -> Result<Vec<ModelPartition>> {
        if layers.is_empty() {
            return Err(anyhow!("没有可拆分的层"));
        }

        info!("开始拆分模型为两部分...");
        
        // 计算总参数数量
        let total_params: usize = layers.iter()
            .map(|layer| layer.parameters.len())
            .sum();

        info!("模型总参数数量: {}", total_params);

        // 按层数量大致平分
        let mid_point = layers.len() / 2;
        
        let part1_layers: Vec<ModelLayer> = layers.iter()
            .take(mid_point)
            .cloned()
            .collect();

        let part2_layers: Vec<ModelLayer> = layers.iter()
            .skip(mid_point)
            .cloned()
            .collect();

        let part1_params: usize = part1_layers.iter()
            .map(|layer| layer.parameters.len())
            .sum();

        let part2_params: usize = part2_layers.iter()
            .map(|layer| layer.parameters.len())
            .sum();

        let partitions = vec![
            ModelPartition {
                part_id: 0,
                layers: part1_layers,
                total_params: part1_params,
            },
            ModelPartition {
                part_id: 1,
                layers: part2_layers,
                total_params: part2_params,
            },
        ];

        info!("模型拆分完成:");
        info!("  Part 0: {} 层, {} 参数", partitions[0].layers.len(), partitions[0].total_params);
        info!("  Part 1: {} 层, {} 参数", partitions[1].layers.len(), partitions[1].total_params);

        Ok(partitions)
    }

    /// 保存拆分后的模型部分
    pub async fn save_partitions(&self, partitions: &[ModelPartition]) -> Result<Vec<PathBuf>> {
        let mut saved_paths = Vec::new();
        
        for partition in partitions {
            let file_path = self.cache_dir
                .join(&self.config.model_id)
                .join(format!("partition_{}.json", partition.part_id));
            
            // 确保目录存在
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // 保存为 JSON 文件
            let json_data = serde_json::to_string_pretty(partition)?;
            fs::write(&file_path, json_data)?;
            
            info!("保存模型分区 {} 到: {:?}", partition.part_id, file_path);
            saved_paths.push(file_path);
        }

        Ok(saved_paths)
    }

    /// 完整的加载和拆分流程
    pub async fn load_and_split(&self) -> Result<Vec<ModelPartition>> {
        // 1. 下载模型
        let model_path = self.download_model().await?;
        
        // 2. 加载参数
        let layers = self.load_model_parameters(&model_path).await?;
        
        // 3. 拆分模型
        let partitions = self.split_model_layers(layers)?;
        
        // 4. 保存分区
        self.save_partitions(&partitions).await?;
        
        Ok(partitions)
    }
}

/// 创建 Llama 3.2 1B 模型加载器的便捷函数
pub fn create_llama_32_1b_loader(cache_dir: Option<PathBuf>) -> Result<LlamaModelLoader> {
    let config = HFModelConfig {
        model_id: "meta-llama/Llama-3.2-1B".to_string(),
        revision: Some("main".to_string()),
        cache_dir,
        token: None, // 如果需要访问私有模型，可以提供 HF token
    };
    
    LlamaModelLoader::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_loader() {
        let temp_dir = TempDir::new().unwrap();
        let loader = create_llama_32_1b_loader(Some(temp_dir.path().to_path_buf()));
        assert!(loader.is_ok());
    }
}

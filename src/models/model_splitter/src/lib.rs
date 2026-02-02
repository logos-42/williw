/**
 * Rust 模块 4: 按算力切分模型
 * 根据 Worker 的分配方案切分模型
 */
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPlan {
    pub node_id: String,
    pub layer_names: Vec<String>,
    pub total_compute: f64,
    pub compute_utilization: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitConfig {
    pub model_name: String,
    pub model_path: String,
    pub split_plan: HashMap<String, SplitPlan>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitResult {
    pub node_id: String,
    pub shard_path: String,
    pub layer_names: Vec<String>,
    pub total_params: usize,
    pub shard_size_mb: f64,
}

pub struct ModelSplitter;

impl Default for ModelSplitter {
    fn default() -> Self {
        Self
    }
}

impl ModelSplitter {
    pub fn new() -> Self {
        Self
    }
    /// 根据方案切分模型（调用 Python 脚本）
    pub async fn split_model(
        &self,
        config: SplitConfig,
        node_id: &str,
    ) -> Result<SplitResult> {
        // 获取本节点的切分方案
        let my_plan = config.split_plan
            .get(node_id)
            .context(format!("节点 {} 没有分配到任何层", node_id))?;

        if my_plan.layer_names.is_empty() {
            anyhow::bail!("节点 {} 的层列表为空", node_id);
        }

        // 方案1: 通过命令行调用 Python 脚本
        let python_script = include_str!("../scripts/split_model.py");
        
        // 将 Python 脚本保存到临时文件
        let script_path = std::env::temp_dir().join("split_model.py");
        tokio::fs::write(&script_path, python_script).await?;
        
        // 将切分方案保存为临时 JSON 文件
        let plan_json = serde_json::to_string(my_plan)?;
        let plan_path = std::env::temp_dir().join(format!("split_plan_{}.json", node_id));
        tokio::fs::write(&plan_path, plan_json).await?;
        
        // 构建输出目录
        let output_dir = config.output_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./model_shards").join(node_id));
        tokio::fs::create_dir_all(&output_dir).await?;
        
        // 构建 Python 命令 - 优先使用 Python 3.12
        let python_cmd = std::env::var("PYTHON_PATH")
            .unwrap_or_else(|_| "C:\\Users\\Mechrevo\\AppData\\Local\\Programs\\Python\\Python312\\python.exe".to_string());

        let output = tokio::process::Command::new(&python_cmd)
            .arg(&script_path)
            .arg("--model-name")
            .arg(&config.model_name)
            .arg("--model-path")
            .arg(&config.model_path)
            .arg("--plan-file")
            .arg(plan_path.to_str().unwrap())
            .arg("--output-dir")
            .arg(output_dir.to_str().unwrap())
            .arg("--node-id")
            .arg(node_id)
            .output()
            .await
            .with_context(|| format!("Failed to execute Python script: {}", python_cmd))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Python script failed: {}", stderr);
        }

        // 解析 JSON 输出
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: SplitResult = serde_json::from_str(&stdout)
            .context("Failed to parse split result JSON")?;

        Ok(result)
    }

    /// 验证切分方案（检查所有层是否都被分配）
    pub fn validate_split_plan(
        &self,
        all_layer_names: &[String],
        split_plan: &HashMap<String, SplitPlan>,
    ) -> Result<()> {
        let mut assigned_layers = std::collections::HashSet::new();
        
        for plan in split_plan.values() {
            for layer_name in &plan.layer_names {
                if assigned_layers.contains(layer_name) {
                    anyhow::bail!("层 {} 被重复分配", layer_name);
                }
                assigned_layers.insert(layer_name.clone());
            }
        }

        // 检查是否有层未被分配
        let all_layers_set: std::collections::HashSet<_> = 
            all_layer_names.iter().cloned().collect();
        
        let missing_layers: Vec<_> = all_layers_set
            .difference(&assigned_layers)
            .cloned()
            .collect();

        if !missing_layers.is_empty() {
            anyhow::bail!("以下层未被分配: {:?}", missing_layers);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_split_plan() {
        let splitter = ModelSplitter;
        
        let all_layers = vec![
            "layer1".to_string(),
            "layer2".to_string(),
            "layer3".to_string(),
        ];
        
        let mut split_plan = HashMap::new();
        split_plan.insert(
            "node1".to_string(),
            SplitPlan {
                node_id: "node1".to_string(),
                layer_names: vec!["layer1".to_string(), "layer2".to_string()],
                total_compute: 100.0,
                compute_utilization: 0.5,
            },
        );
        split_plan.insert(
            "node2".to_string(),
            SplitPlan {
                node_id: "node2".to_string(),
                layer_names: vec!["layer3".to_string()],
                total_compute: 50.0,
                compute_utilization: 0.3,
            },
        );
        
        let result = splitter.validate_split_plan(&all_layers, &split_plan);
        assert!(result.is_ok());
    }
}

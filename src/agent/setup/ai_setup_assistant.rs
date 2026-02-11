//! AI 配置助手
//!
//! 使用 DeepSeek API 指导整个系统配置过程

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tokio::time::{sleep, Duration};

/// 配置助手
pub struct AISetupAssistant {
    api_key: String,
    api_base: String,
    model: String,
    client: reqwest::Client,
}

/// 系统检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDetectionResult {
    pub os_type: String,
    pub python_version: Option<String>,
    pub cuda_available: bool,
    pub cuda_version: Option<String>,
    pub gpu_devices: Vec<GPUDevice>,
    pub python_packages: HashMap<String, String>,
    pub memory_gb: f64,
    pub disk_free_gb: f64,
    pub cpu_cores: u32,
}

/// GPU 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUDevice {
    pub index: u32,
    pub name: String,
    pub memory_mb: u64,
    pub compute_capability: Option<String>,
}

/// 配置步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: Option<String>,
    pub estimated_time_secs: u32,
    pub critical: bool,
    pub verification_command: Option<String>,
}

/// AI 配置建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISetupAdvice {
    pub summary: String,
    pub steps: Vec<SetupStep>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
    pub estimated_total_time_mins: u32,
}

/// 配置进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupProgress {
    pub total_steps: usize,
    pub completed_steps: usize,
    pub current_step: Option<String>,
    pub status: SetupStatus,
    pub messages: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SetupStatus {
    NotStarted,
    Detecting,
    Planning,
    Executing,
    Verifying,
    Completed,
    Failed,
}

impl AISetupAssistant {
    /// 创建新的配置助手
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_base: "https://api.deepseek.com/v1".to_string(),
            model: "deepseek-chat".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// 使用自定义模型
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// 检测系统环境
    pub async fn detect_system(&self) -> Result<SystemDetectionResult, String> {
        let mut result = SystemDetectionResult {
            os_type: std::env::consts::OS.to_string(),
            python_version: None,
            cuda_available: false,
            cuda_version: None,
            gpu_devices: Vec::new(),
            python_packages: HashMap::new(),
            memory_gb: 0.0,
            disk_free_gb: 0.0,
            cpu_cores: 0,
        };

        // 检测 Python 版本
        if let Ok(output) = Command::new("python").args(["--version"]).output() {
            if let Ok(version) = String::from_utf8(output.stdout) {
                result.python_version = Some(version.trim().to_string());
            }
        }

        // 检测 CUDA
        if let Ok(output) = Command::new("nvcc").args(["--version"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("release") {
                result.cuda_available = true;
                // 提取版本号
                for line in output_str.lines() {
                    if line.contains("release") {
                        if let Some(idx) = line.find("release") {
                            let version_part = &line[idx + 8..];
                            if let Some(end_idx) = version_part.find(",") {
                                result.cuda_version = Some(version_part[..end_idx].trim().to_string());
                            }
                        }
                    }
                }
            }
        }

        // 检测 NVIDIA GPU
        if result.cuda_available {
            if let Ok(output) = Command::new("nvidia-smi")
                .args(["--query-gpu=index,name,memory.total,compute_cap", "--format=csv,noheader"])
                .output()
            {
                let gpu_info = String::from_utf8_lossy(&output.stdout);
                for line in gpu_info.lines() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        if let Ok(index) = parts[0].parse::<u32>() {
                            let memory_str = parts[2].replace("MiB", "").replace("MB", "");
                            let memory_mb = memory_str.parse::<u64>().unwrap_or(0);
                            
                            result.gpu_devices.push(GPUDevice {
                                index,
                                name: parts[1].to_string(),
                                memory_mb,
                                compute_capability: parts.get(3).map(|s| s.to_string()),
                            });
                        }
                    }
                }
            }
        }

        // 检测已安装的 Python 包
        let packages = vec!["torch", "numpy", "transformers", "safetensors", "flask", "fastapi"];
        for pkg in packages {
            if let Ok(output) = Command::new("python")
                .args(["-c", &format!("import {}; print({}.__version__)", pkg, pkg)])
                .output()
            {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    result.python_packages.insert(pkg.to_string(), version.trim().to_string());
                }
            }
        }

        // 检测系统资源
        #[cfg(target_os = "windows")]
        {
            // CPU 核心数
            if let Ok(output) = Command::new("wmic")
                .args(["cpu", "get", "NumberOfLogicalProcessors", "/value"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.starts_with("NumberOfLogicalProcessors=") {
                        if let Ok(cores) = line[26..].trim().parse::<u32>() {
                            result.cpu_cores = cores;
                        }
                    }
                }
            }

            // 内存信息
            if let Ok(output) = Command::new("wmic")
                .args(["computersystem", "get", "TotalPhysicalMemory", "/value"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.starts_with("TotalPhysicalMemory=") {
                        if let Ok(bytes) = line[20..].trim().parse::<u64>() {
                            result.memory_gb = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
                        }
                    }
                }
            }

            // 磁盘空间
            if let Ok(output) = Command::new("wmic")
                .args(["logicaldisk", "where", "DeviceID='C:'", "get", "FreeSpace", "/value"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.starts_with("FreeSpace=") {
                        if let Ok(bytes) = line[10..].trim().parse::<u64>() {
                            result.disk_free_gb = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// 获取 AI 配置建议
    pub async fn get_setup_advice(&self, detection: &SystemDetectionResult) -> Result<AISetupAdvice, String> {
        let system_prompt = r#"你是一位专业的 AI 系统配置专家。基于系统检测结果，提供详细的配置步骤和建议。

你需要：
1. 分析系统当前状态
2. 识别缺失的组件
3. 提供具体的安装命令
4. 估算每个步骤的时间
5. 标记关键步骤（失败会导致整个配置失败）

输出必须是合法的 JSON 格式，包含以下字段：
{
  "summary": "配置摘要",
  "steps": [
    {
      "id": "step_id",
      "name": "步骤名称",
      "description": "详细描述",
      "command": "可选的安装命令",
      "estimated_time_secs": 估计秒数,
      "critical": true/false,
      "verification_command": "验证命令"
    }
  ],
  "recommendations": ["建议1", "建议2"],
  "warnings": ["警告1", "警告2"],
  "estimated_total_time_mins": 总估计分钟数
}"#;

        let user_prompt = format!(
            "请分析以下系统检测结果并提供配置建议：\n\n{}",
            serde_json::to_string_pretty(detection).map_err(|e| e.to_string())?
        );

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.3,
            "max_tokens": 4000
        });

        let response = self.client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("API 请求失败: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API 错误: {}", error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("响应格式错误")?;

        // 提取 JSON 部分
        let json_str = if content.contains("```json") {
            content.split("```json").nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(content)
        } else if content.contains("```") {
            content.split("```").nth(1)
                .unwrap_or(content)
        } else {
            content
        };

        let advice: AISetupAdvice = serde_json::from_str(json_str.trim())
            .map_err(|e| format!("解析 AI 建议失败: {}\n内容: {}", e, json_str))?;

        Ok(advice)
    }

    /// 执行配置步骤
    pub async fn execute_step(&self, step: &SetupStep) -> Result<String, String> {
        println!("🔄 执行步骤: {}", step.name);
        
        if let Some(command) = &step.command {
            println!("   命令: {}", command);
            
            let output = Command::new("cmd")
                .args(["/C", command])
                .output()
                .map_err(|e| format!("执行命令失败: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !output.status.success() {
                return Err(format!("命令执行失败: {}", stderr));
            }

            // 等待估计的时间
            sleep(Duration::from_secs(step.estimated_time_secs as u64)).await;

            // 验证
            if let Some(verify_cmd) = &step.verification_command {
                let verify_output = Command::new("cmd")
                    .args(["/C", verify_cmd])
                    .output();
                
                if let Ok(output) = verify_output {
                    if !output.status.success() {
                        return Err("验证失败".to_string());
                    }
                }
            }

            Ok(stdout.to_string())
        } else {
            // 没有命令的步骤（纯信息）
            sleep(Duration::from_secs(1)).await;
            Ok("步骤完成".to_string())
        }
    }

    /// 运行完整配置流程
    pub async fn run_full_setup<F>(
        &self,
        progress_callback: F,
    ) -> Result<SetupProgress, String>
    where
        F: Fn(SetupProgress) + Send + Sync + 'static,
    {
        let mut progress = SetupProgress {
            total_steps: 0,
            completed_steps: 0,
            current_step: None,
            status: SetupStatus::Detecting,
            messages: vec!["开始系统检测...".to_string()],
            errors: Vec::new(),
        };

        progress_callback(progress.clone());

        // 1. 系统检测
        println!("🔍 步骤 1/4: 检测系统环境...");
        let detection = match self.detect_system().await {
            Ok(d) => {
                progress.messages.push(format!(
                    "检测完成: {} GPU(s), {} GB 内存",
                    d.gpu_devices.len(),
                    d.memory_gb as i32
                ));
                d
            }
            Err(e) => {
                progress.errors.push(format!("系统检测失败: {}", e));
                progress.status = SetupStatus::Failed;
                progress_callback(progress.clone());
                return Err(e);
            }
        };

        progress.status = SetupStatus::Planning;
        progress_callback(progress.clone());

        // 2. 获取 AI 配置建议
        println!("🤖 步骤 2/4: 获取 AI 配置建议...");
        let advice = match self.get_setup_advice(&detection).await {
            Ok(a) => {
                progress.messages.push(format!("AI 建议: {}", a.summary));
                progress.total_steps = a.steps.len();
                a
            }
            Err(e) => {
                progress.errors.push(format!("获取 AI 建议失败: {}", e));
                progress.status = SetupStatus::Failed;
                progress_callback(progress.clone());
                return Err(e);
            }
        };

        progress.status = SetupStatus::Executing;
        progress_callback(progress.clone());

        // 3. 执行配置步骤
        println!("⚙️  步骤 3/4: 执行配置 ({} 个步骤)...", advice.steps.len());
        for (i, step) in advice.steps.iter().enumerate() {
            progress.current_step = Some(step.name.clone());
            progress.messages.push(format!("[{}] {}", i + 1, step.name));
            progress_callback(progress.clone());

            match self.execute_step(step).await {
                Ok(result) => {
                    progress.completed_steps += 1;
                    progress.messages.push(format!("  ✅ 完成: {}", result.lines().next().unwrap_or("OK")));
                }
                Err(e) => {
                    progress.errors.push(format!("步骤 '{}' 失败: {}", step.name, e));
                    if step.critical {
                        progress.status = SetupStatus::Failed;
                        progress_callback(progress.clone());
                        return Err(format!("关键步骤失败: {}", e));
                    } else {
                        progress.messages.push(format!("  ⚠️  跳过: {}", e));
                    }
                }
            }
            progress_callback(progress.clone());
        }

        // 4. 验证配置
        println!("✅ 步骤 4/4: 验证配置...");
        progress.status = SetupStatus::Verifying;
        progress.current_step = Some("验证配置".to_string());
        progress_callback(progress.clone());

        let final_detection = self.detect_system().await?;
        
        // 检查关键组件是否就绪
        let has_torch = final_detection.python_packages.contains_key("torch");
        let has_transformers = final_detection.python_packages.contains_key("transformers");
        let has_flask = final_detection.python_packages.contains_key("flask");

        if has_torch && has_transformers && has_flask {
            progress.status = SetupStatus::Completed;
            progress.messages.push("🎉 配置完成！所有关键组件已就绪".to_string());
            progress.current_step = None;
        } else {
            progress.status = SetupStatus::Failed;
            progress.errors.push("配置验证失败：缺少必要组件".to_string());
        }

        progress_callback(progress.clone());
        Ok(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_system() {
        let assistant = AISetupAssistant::new("test_key".to_string());
        let result = assistant.detect_system().await;
        assert!(result.is_ok());
        
        let detection = result.unwrap();
        println!("检测到的系统信息: {:?}", detection);
        assert!(!detection.os_type.is_empty());
    }
}

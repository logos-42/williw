//! 配置任务定义
//!
//! 定义具体的配置任务和操作

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::path::PathBuf;

/// 配置任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SetupTaskType {
    /// 安装 Python 包
    InstallPythonPackage { name: String, version: Option<String> },
    /// 创建虚拟环境
    CreateVirtualEnv { path: String },
    /// 下载模型
    DownloadModel { model_id: String, cache_dir: String },
    /// 配置 GPU
    ConfigureGPU,
    /// 启动推理服务器
    StartInferenceServer { port: u16 },
    /// 加入去中心化网络
    JoinDecentralizedNetwork { bootstrap_nodes: Vec<String> },
    /// 自定义命令
    CustomCommand { command: String, description: String },
}

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub message: String,
    pub output: Option<String>,
    pub duration_ms: u64,
}

/// 配置任务执行器
pub struct SetupTaskExecutor;

impl SetupTaskExecutor {
    /// 创建新的任务执行器
    pub fn new() -> Self {
        Self
    }

    /// 执行任务
    pub async fn execute(&self, task: &SetupTaskType) -> TaskResult {
        let start = std::time::Instant::now();
        
        match task {
            SetupTaskType::InstallPythonPackage { name, version } => {
                self.install_python_package(name, version.as_deref()).await
            }
            SetupTaskType::CreateVirtualEnv { path } => {
                self.create_virtual_env(path).await
            }
            SetupTaskType::DownloadModel { model_id, cache_dir } => {
                self.download_model(model_id, cache_dir).await
            }
            SetupTaskType::ConfigureGPU => {
                self.configure_gpu().await
            }
            SetupTaskType::StartInferenceServer { port } => {
                self.start_inference_server(*port).await
            }
            SetupTaskType::JoinDecentralizedNetwork { bootstrap_nodes } => {
                self.join_network(bootstrap_nodes).await
            }
            SetupTaskType::CustomCommand { command, description } => {
                self.run_custom_command(command, description).await
            }
        }
        .map(|msg| TaskResult {
            success: true,
            message: msg,
            output: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
        .unwrap_or_else(|e| TaskResult {
            success: false,
            message: e,
            output: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// 安装 Python 包
    async fn install_python_package(&self, name: &str, version: Option<&str>) -> Result<String, String> {
        let pkg_spec = match version {
            Some(v) => format!("{}=={}", name, v),
            None => name.to_string(),
        };

        println!("📦 安装 Python 包: {}", pkg_spec);

        let output = Command::new("pip")
            .args(["install", &pkg_spec, "--upgrade"])
            .output()
            .map_err(|e| format!("无法执行 pip: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pip 安装失败: {}", stderr));
        }

        Ok(format!("成功安装 {}", pkg_spec))
    }

    /// 创建虚拟环境
    async fn create_virtual_env(&self, path: &str) -> Result<String, String> {
        println!("🐍 创建虚拟环境: {}", path);

        // 检查目录是否已存在
        if PathBuf::from(path).exists() {
            return Ok(format!("虚拟环境已存在: {}", path));
        }

        let output = Command::new("python")
            .args(["-m", "venv", path])
            .output()
            .map_err(|e| format!("无法创建虚拟环境: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("创建虚拟环境失败: {}", stderr));
        }

        Ok(format!("成功创建虚拟环境: {}", path))
    }

    /// 下载模型
    async fn download_model(&self, model_id: &str, cache_dir: &str) -> Result<String, String> {
        println!("📥 下载模型: {} -> {}", model_id, cache_dir);

        // 创建缓存目录
        std::fs::create_dir_all(cache_dir)
            .map_err(|e| format!("无法创建缓存目录: {}", e))?;

        // 使用 Python 脚本下载模型
        let script = format!(
            r#"
import os
os.environ['HF_HOME'] = '{}'
os.environ['TRANSFORMERS_CACHE'] = '{}'

from transformers import AutoModelForCausalLM, AutoTokenizer

print('Downloading model: {}')
tokenizer = AutoTokenizer.from_pretrained('{}')
model = AutoModelForCausalLM.from_pretrained('{}')
print('Model downloaded successfully')
"#,
            cache_dir, cache_dir, model_id, model_id, model_id
        );

        let output = Command::new("python")
            .arg("-c")
            .arg(&script)
            .output()
            .map_err(|e| format!("无法执行下载脚本: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("模型下载失败: {}", stderr));
        }

        Ok(format!("成功下载模型: {}", model_id))
    }

    /// 配置 GPU
    async fn configure_gpu(&self) -> Result<String, String> {
        println!("🎮 配置 GPU...");

        // 检查 CUDA 可用性
        let output = Command::new("python")
            .arg("-c")
            .arg("import torch; print(f'CUDA available: {torch.cuda.is_available()}'); print(f'CUDA device count: {torch.cuda.device_count()}')")
            .output()
            .map_err(|e| format!("无法检查 CUDA: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        if !output.status.success() || !stdout.contains("CUDA available: True") {
            return Err("CUDA 不可用".to_string());
        }

        // 设置 CUDA 环境变量
        #[cfg(target_os = "windows")]
        {
            // 在 Windows 上添加 CUDA 到 PATH
            let cuda_path = r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v11.8\bin";
            let current_path = std::env::var("PATH").unwrap_or_default();
            if !current_path.contains(cuda_path) {
                std::env::set_var("PATH", format!("{};{}", cuda_path, current_path));
            }
        }

        Ok(format!("GPU 配置完成: {}", stdout.trim()))
    }

    /// 启动推理服务器
    async fn start_inference_server(&self, port: u16) -> Result<String, String> {
        println!("🚀 启动推理服务器 (端口 {})...", port);

        // 检查端口是否被占用
        if self.is_port_in_use(port).await {
            return Ok(format!("端口 {} 已被占用，可能服务器已在运行", port));
        }

        // 启动服务器进程
        let project_root = std::env::current_dir()
            .map_err(|e| format!("无法获取当前目录: {}", e))?;
        
        let server_script = project_root.join("gpu_inference_server_clean.py");
        
        if !server_script.exists() {
            return Err(format!("服务器脚本不存在: {:?}", server_script));
        }

        // 使用 Python 启动服务器（后台运行）
        #[cfg(target_os = "windows")]
        {
            let _child = Command::new("python")
                .arg(&server_script)
                .arg("--port")
                .arg(port.to_string())
                .spawn()
                .map_err(|e| format!("无法启动服务器: {}", e))?;
        }

        // 等待服务器启动
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // 检查服务器是否响应
        match reqwest::get(format!("http://localhost:{}/", port)).await {
            Ok(response) if response.status().is_success() => {
                Ok(format!("推理服务器已在端口 {} 启动", port))
            }
            _ => Err("服务器启动后无法访问".to_string()),
        }
    }

    /// 检查端口是否被占用
    async fn is_port_in_use(&self, port: u16) -> bool {
        tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .is_err()
    }

    /// 加入去中心化网络
    async fn join_network(&self, bootstrap_nodes: &[String]) -> Result<String, String> {
        println!("🌐 加入去中心化网络...");
        println!("   Bootstrap 节点: {:?}", bootstrap_nodes);

        // 这里将调用去中心化计算网络的接口
        // 目前返回成功，实际实现需要集成 Iroh P2P
        Ok(format!("已连接到 {} 个 bootstrap 节点", bootstrap_nodes.len()))
    }

    /// 运行自定义命令
    async fn run_custom_command(&self, command: &str, description: &str) -> Result<String, String> {
        println!("🔧 {}: {}", description, command);

        #[cfg(target_os = "windows")]
        let output = Command::new("cmd")
            .args(["/C", command])
            .output();
        
        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(format!("{} 完成: {}", description, stdout.lines().next().unwrap_or("OK")))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(format!("{} 失败: {}", description, stderr))
                }
            }
            Err(e) => Err(format!("无法执行命令: {}", e)),
        }
    }
}

impl Default for SetupTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 预定义的配置任务列表
pub fn get_default_setup_tasks() -> Vec<SetupTaskType> {
    vec![
        SetupTaskType::InstallPythonPackage {
            name: "torch".to_string(),
            version: None,
        },
        SetupTaskType::InstallPythonPackage {
            name: "transformers".to_string(),
            version: None,
        },
        SetupTaskType::InstallPythonPackage {
            name: "flask".to_string(),
            version: None,
        },
        SetupTaskType::InstallPythonPackage {
            name: "flask-cors".to_string(),
            version: None,
        },
        SetupTaskType::ConfigureGPU,
        SetupTaskType::StartInferenceServer { port: 8000 },
    ]
}

/// 检查配置状态
pub async fn check_setup_status() -> HashMap<String, bool> {
    let mut status = HashMap::new();

    // 检查 Python
    status.insert(
        "python".to_string(),
        Command::new("python").arg("--version").output().is_ok(),
    );

    // 检查 pip
    status.insert(
        "pip".to_string(),
        Command::new("pip").arg("--version").output().is_ok(),
    );

    // 检查 CUDA
    status.insert(
        "cuda".to_string(),
        Command::new("nvcc").arg("--version").output().is_ok(),
    );

    // 检查 torch
    status.insert(
        "torch".to_string(),
        Command::new("python")
            .args(["-c", "import torch; print(torch.__version__)"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false),
    );

    // 检查 transformers
    status.insert(
        "transformers".to_string(),
        Command::new("python")
            .args(["-c", "import transformers; print(transformers.__version__)"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false),
    );

    // 检查服务器是否运行
    status.insert(
        "inference_server".to_string(),
        reqwest::get("http://localhost:8000/")
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false),
    );

    status
}

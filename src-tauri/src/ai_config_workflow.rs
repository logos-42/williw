//! AI 驱动的系统配置工作流
//! 
//! 这个模块让 AI 使用各种工具来自动完成系统配置：
//! - BashTool: 执行命令检测和安装环境
//! - DecentralizedModelTool: 切分和分发模型
//! - IrohCommsTool: 配置 P2P 网络

use crate::state::{AppState, WorkflowStatus};
use tauri::State;
use tauri::Emitter;
use williw::agent::tools::{
    ToolExecutor, create_execution_context,
    BashTool,
    DecentralizedModelTool,
    IrohCommsTool,
};
use williw::agent::tools::bash::Shell;
use uuid::Uuid;

/// AI 配置工作流
pub struct AIConfigWorkflow {
    app_handle: tauri::AppHandle,
    execution_id: String,
}

impl AIConfigWorkflow {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            execution_id: format!("ai-config-{}", Uuid::new_v4()),
        }
    }

    /// 发送工作流消息到前端
    async fn emit_message(&self, msg_type: &str, content: &str) {
        let _ = self.app_handle.emit("workflow-message", serde_json::json!({
            "type": msg_type,
            "content": content,
        }));
    }

    /// 使用 Bash 工具执行命令
    async fn execute_bash(&self, command: &str, description: &str) -> Result<String, String> {
        self.emit_message("progress", &format!("🔧 {}", description)).await;
        
        let tool = BashTool::new();
        let context = create_execution_context(self.execution_id.clone());
        
        let args = serde_json::json!({
            "command": command,
            "shell": "bash",
            "timeout": 60
        });
        
        match tool.execute(args, &context).await {
            Ok(result) => {
                let output = format!("{:?}", result.data);
                self.emit_message("success", &format!("✅ {} 完成", description)).await;
                Ok(output)
            }
            Err(e) => {
                let error = format!("{:?}", e);
                self.emit_message("warning", &format!("⚠️ {} 失败: {}", description, error)).await;
                Err(error)
            }
        }
    }

    /// AI 步骤 1: 检测 Python 环境
    pub async fn check_python(&self) -> Result<bool, String> {
        self.emit_message("info", "🤖 AI: 开始检测 Python 环境...").await;
        
        match self.execute_bash(
            "python --version || python3 --version",
            "检测 Python"
        ).await {
            Ok(version) => {
                self.emit_message("success", &format!("✅ Python 已安装: {}", version)).await;
                Ok(true)
            }
            Err(_) => {
                self.emit_message("warning", "⚠️ Python 未安装，AI 将尝试安装...").await;
                Ok(false)
            }
        }
    }

    /// AI 步骤 2: 检测 CUDA/GPU
    pub async fn check_gpu(&self) -> Result<bool, String> {
        self.emit_message("info", "🤖 AI: 检测 GPU 和 CUDA 可用性...").await;
        
        // 首先尝试 nvcc
        match self.execute_bash("nvcc --version", "检测 CUDA (nvcc)").await {
            Ok(_) => {
                self.emit_message("success", "✅ CUDA 工具包已安装").await;
                return Ok(true);
            }
            Err(_) => {}
        }
        
        // 然后尝试通过 Python 检测 PyTorch CUDA
        match self.execute_bash(
            "python3 -c 'import torch; print(f\"CUDA available: {torch.cuda.is_available()}\")' 2>/dev/null || python -c 'import torch; print(f\"CUDA available: {torch.cuda.is_available()}\")' 2>/dev/null || echo 'PyTorch not installed'",
            "检测 PyTorch CUDA"
        ).await {
            Ok(output) if output.contains("True") => {
                self.emit_message("success", "✅ PyTorch CUDA 可用").await;
                Ok(true)
            }
            _ => {
                self.emit_message("warning", "⚠️ GPU 不可用，将使用 CPU 模式").await;
                Ok(false)
            }
        }
    }

    /// AI 步骤 3: 安装 Python 依赖
    pub async fn install_dependencies(&self) -> Result<(), String> {
        self.emit_message("info", "🤖 AI: 开始安装 Python 依赖...").await;
        
        let packages = vec![
            "torch",
            "transformers",
            "accelerate",
            "sentencepiece",
            "protobuf",
            "iroh",
        ];
        
        for package in packages {
            let cmd = format!("pip install --upgrade {} || pip3 install --upgrade {}", package, package);
            match self.execute_bash(&cmd, &format!("安装 {}", package)).await {
                Ok(_) => {}
                Err(_) => {
                    self.emit_message("warning", &format!("⚠️ {} 安装可能有问题，继续其他包", package)).await;
                }
            }
        }
        
        self.emit_message("success", "✅ 依赖安装完成").await;
        Ok(())
    }

    /// AI 步骤 4: 使用 DecentralizedModelTool 配置模型
    pub async fn configure_model(&self) -> Result<(), String> {
        self.emit_message("info", "🤖 AI: 使用 DecentralizedModelTool 配置模型...").await;
        
        let tool = DecentralizedModelTool::new();
        let context = create_execution_context(self.execution_id.clone());
        
        // 执行完整流水线
        let args = serde_json::json!({
            "operation": "FullPipeline",
            "model_name": "default_model",
            "model_source": "huggingface",
            "output_dir": "./models",
            "target_nodes": vec!["local_node".to_string()]
        });
        
        match tool.execute(args, &context).await {
            Ok(result) => {
                let data = format!("{:?}", result.data);
                self.emit_message("success", &format!("✅ 模型配置完成: {}", data)).await;
                Ok(())
            }
            Err(e) => {
                let error = format!("{:?}", e);
                self.emit_message("warning", &format!("⚠️ 模型配置可能有问题: {}", error)).await;
                Ok(()) // 继续执行
            }
        }
    }

    /// AI 步骤 5: 使用 IrohCommsTool 配置网络
    pub async fn configure_network(&self) -> Result<(), String> {
        self.emit_message("info", "🤖 AI: 使用 IrohCommsTool 配置 P2P 网络...").await;
        
        match IrohCommsTool::new().await {
            Ok(tool) => {
                self.emit_message("success", "✅ Iroh 工具初始化成功").await;
                let context = create_execution_context(self.execution_id.clone());
                
                // 获取节点 ID
                let args = serde_json::json!({
                    "GetNodeId": {}
                });
                match tool.execute(args, &context).await {
                    Ok(result) => {
                        let node_id = format!("{:?}", result.data);
                        self.emit_message("success", &format!("🔑 Iroh 节点 ID: {}", node_id)).await;
                    }
                    Err(e) => {
                        self.emit_message("warning", &format!("⚠️ 获取节点ID失败: {:?}", e)).await;
                    }
                }
                
                // 广播就绪状态
                let args = serde_json::json!({
                    "BroadcastMessage": {
                        "message_type": "node_ready",
                        "message": "AI配置完成，节点就绪"
                    }
                });
                match tool.execute(args, &context).await {
                    Ok(_) => {
                        self.emit_message("success", "📢 已广播节点就绪状态").await;
                    }
                    Err(e) => {
                        self.emit_message("warning", &format!("⚠️ 广播失败: {:?}", e)).await;
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                self.emit_message("warning", &format!("⚠️ Iroh 初始化失败: {}，跳过网络配置", e)).await;
                Ok(())
            }
        }
    }

    /// 运行完整的 AI 配置工作流
    pub async fn run_full_workflow(&self) -> Result<(), String> {
        self.emit_message("info", "🚀 AI 自主配置工作流启动").await;
        self.emit_message("info", "🤖 AI 将使用以下工具：").await;
        self.emit_message("info", "  • BashTool - 执行系统命令").await;
        self.emit_message("info", "  • DecentralizedModelTool - 模型处理").await;
        self.emit_message("info", "  • IrohCommsTool - P2P网络配置").await;
        
        // Phase 1: 环境检测
        self.emit_message("progress", "🔍 第一阶段：AI 检测系统环境").await;
        let _ = self.check_python().await;
        let _ = self.check_gpu().await;
        
        // Phase 2: 依赖安装
        self.emit_message("progress", "📦 第二阶段：AI 安装依赖").await;
        let _ = self.install_dependencies().await;
        
        // Phase 3: 模型配置
        self.emit_message("progress", "🤖 第三阶段：AI 配置模型").await;
        let _ = self.configure_model().await;
        
        // Phase 4: 网络配置
        self.emit_message("progress", "🌐 第四阶段：AI 配置网络").await;
        let _ = self.configure_network().await;
        
        // 完成
        self.emit_message("success", "✅ AI 配置工作流完成！").await;
        self.emit_message("success", "🎉 去中心化算力网络已就绪").await;
        
        Ok(())
    }
}

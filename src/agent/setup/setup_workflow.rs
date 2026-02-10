//! 配置工作流
//!
//! 定义和执 AI 驱动的配置工作流

use super::{ai_setup_assistant::*, setup_tasks::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

/// 配置工作流
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupWorkflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<SetupWorkflowStep>,
    pub metadata: HashMap<String, String>,
}

/// 配置工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupWorkflowStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task: SetupTaskType,
    pub depends_on: Vec<String>,
    pub retry_count: u32,
    pub is_critical: bool,
}

/// 配置工作流执行器
pub struct SetupWorkflowExecutor {
    assistant: AISetupAssistant,
    task_executor: SetupTaskExecutor,
    workflows: Arc<RwLock<HashMap<String, SetupWorkflow>>>,
    active_executions: Arc<RwLock<HashMap<String, SetupExecution>>>,
    event_sender: mpsc::UnboundedSender<SetupEvent>,
}

/// 配置执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupExecution {
    pub id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub current_step: Option<String>,
    pub completed_steps: Vec<String>,
    pub failed_steps: Vec<(String, String)>,
    pub results: HashMap<String, TaskResult>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub progress_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// 配置事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SetupEvent {
    ExecutionStarted { execution_id: String, workflow_id: String },
    StepStarted { execution_id: String, step_id: String, step_name: String },
    StepCompleted { execution_id: String, step_id: String, result: TaskResult },
    StepFailed { execution_id: String, step_id: String, error: String },
    ProgressUpdated { execution_id: String, percent: f32 },
    ExecutionCompleted { execution_id: String, success: bool },
}

impl SetupWorkflowExecutor {
    /// 创建新的工作流执行器
    pub fn new(api_key: String) -> (Self, mpsc::UnboundedReceiver<SetupEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let executor = Self {
            assistant: AISetupAssistant::new(api_key),
            task_executor: SetupTaskExecutor::new(),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        };

        (executor, event_receiver)
    }

    /// 注册工作流
    pub async fn register_workflow(&self, workflow: SetupWorkflow) {
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow.id.clone(), workflow);
    }

    /// 创建默认配置工作流
    pub fn create_default_workflow() -> SetupWorkflow {
        SetupWorkflow {
            id: "default_setup".to_string(),
            name: "默认系统配置".to_string(),
            description: "自动配置 GPU 推理环境和去中心化网络".to_string(),
            steps: vec![
                SetupWorkflowStep {
                    id: "detect_system".to_string(),
                    name: "系统检测".to_string(),
                    description: "检测系统环境和可用资源".to_string(),
                    task: SetupTaskType::CustomCommand {
                        command: "python --version && nvidia-smi --query-gpu=name,memory.total --format=csv,noheader".to_string(),
                        description: "检测 Python 和 GPU".to_string(),
                    },
                    depends_on: vec![],
                    retry_count: 0,
                    is_critical: true,
                },
                SetupWorkflowStep {
                    id: "install_torch".to_string(),
                    name: "安装 PyTorch".to_string(),
                    description: "安装 PyTorch GPU 版本".to_string(),
                    task: SetupTaskType::InstallPythonPackage {
                        name: "torch".to_string(),
                        version: None,
                    },
                    depends_on: vec!["detect_system".to_string()],
                    retry_count: 2,
                    is_critical: true,
                },
                SetupWorkflowStep {
                    id: "install_transformers".to_string(),
                    name: "安装 Transformers".to_string(),
                    description: "安装 Hugging Face Transformers 库".to_string(),
                    task: SetupTaskType::InstallPythonPackage {
                        name: "transformers".to_string(),
                        version: None,
                    },
                    depends_on: vec!["install_torch".to_string()],
                    retry_count: 2,
                    is_critical: true,
                },
                SetupWorkflowStep {
                    id: "install_flask".to_string(),
                    name: "安装 Flask".to_string(),
                    description: "安装 Flask Web 框架".to_string(),
                    task: SetupTaskType::InstallPythonPackage {
                        name: "flask".to_string(),
                        version: None,
                    },
                    depends_on: vec![],
                    retry_count: 2,
                    is_critical: true,
                },
                SetupWorkflowStep {
                    id: "configure_gpu".to_string(),
                    name: "配置 GPU".to_string(),
                    description: "验证和配置 CUDA GPU".to_string(),
                    task: SetupTaskType::ConfigureGPU,
                    depends_on: vec!["install_torch".to_string()],
                    retry_count: 1,
                    is_critical: false,
                },
                SetupWorkflowStep {
                    id: "start_server".to_string(),
                    name: "启动推理服务器".to_string(),
                    description: "启动本地 GPU 推理服务".to_string(),
                    task: SetupTaskType::StartInferenceServer { port: 8000 },
                    depends_on: vec![
                        "install_torch".to_string(),
                        "install_transformers".to_string(),
                        "install_flask".to_string(),
                    ],
                    retry_count: 1,
                    is_critical: true,
                },
            ],
            metadata: HashMap::new(),
        }
    }

    /// 执行工作流
    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
    ) -> Result<SetupExecution, String> {
        // 获取工作流
        let workflow = {
            let workflows = self.workflows.read().await;
            workflows
                .get(workflow_id)
                .cloned()
                .ok_or_else(|| format!("工作流 '{}' 不存在", workflow_id))?
        };

        // 创建执行上下文
        let execution_id = format!("setup_{}", uuid::Uuid::new_v4());
        let execution = SetupExecution {
            id: execution_id.clone(),
            workflow_id: workflow_id.to_string(),
            status: ExecutionStatus::Running,
            current_step: None,
            completed_steps: Vec::new(),
            failed_steps: Vec::new(),
            results: HashMap::new(),
            started_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            progress_percent: 0.0,
        };

        // 存储执行状态
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id.clone(), execution.clone());
        }

        // 发送开始事件
        let _ = self.event_sender.send(SetupEvent::ExecutionStarted {
            execution_id: execution_id.clone(),
            workflow_id: workflow_id.to_string(),
        });

        // 执行步骤
        let total_steps = workflow.steps.len();
        let mut completed = 0;

        for step in &workflow.steps {
            // 检查依赖是否满足
            let deps_satisfied = step.depends_on.iter().all(|dep| {
                execution.completed_steps.contains(dep)
            });

            if !deps_satisfied {
                let error = format!("步骤 '{}' 的依赖未满足", step.name);
                let _ = self.event_sender.send(SetupEvent::StepFailed {
                    execution_id: execution_id.clone(),
                    step_id: step.id.clone(),
                    error: error.clone(),
                });
                
                if step.is_critical {
                    self.fail_execution(&execution_id, error).await;
                    return Err("关键步骤依赖未满足".to_string());
                }
                continue;
            }

            // 更新当前步骤
            {
                let mut executions = self.active_executions.write().await;
                if let Some(exec) = executions.get_mut(&execution_id) {
                    exec.current_step = Some(step.name.clone());
                }
            }

            // 发送步骤开始事件
            let _ = self.event_sender.send(SetupEvent::StepStarted {
                execution_id: execution_id.clone(),
                step_id: step.id.clone(),
                step_name: step.name.clone(),
            });

            println!("🔄 执行步骤: {}", step.name);

            // 执行任务（带重试）
            let mut result = None;
            for attempt in 0..=step.retry_count {
                if attempt > 0 {
                    println!("   第 {} 次重试...", attempt);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }

                result = Some(self.task_executor.execute(&step.task).await);
                
                if result.as_ref().unwrap().success {
                    break;
                }
            }

            let result = result.unwrap();

            // 更新执行状态
            {
                let mut executions = self.active_executions.write().await;
                if let Some(exec) = executions.get_mut(&execution_id) {
                    exec.results.insert(step.id.clone(), result.clone());
                    
                    if result.success {
                        exec.completed_steps.push(step.id.clone());
                        completed += 1;
                    } else {
                        exec.failed_steps.push((step.id.clone(), result.message.clone()));
                    }

                    exec.progress_percent = (completed as f32 / total_steps as f32) * 100.0;
                }
            }

            // 发送事件
            if result.success {
                println!("   ✅ 完成: {}", result.message);
                let _ = self.event_sender.send(SetupEvent::StepCompleted {
                    execution_id: execution_id.clone(),
                    step_id: step.id.clone(),
                    result: result.clone(),
                });
            } else {
                println!("   ❌ 失败: {}", result.message);
                let _ = self.event_sender.send(SetupEvent::StepFailed {
                    execution_id: execution_id.clone(),
                    step_id: step.id.clone(),
                    error: result.message.clone(),
                });

                if step.is_critical {
                    self.fail_execution(&execution_id, result.message).await;
                    return Err(format!("关键步骤 '{}' 失败", step.name));
                }
            }

            // 发送进度更新
            let progress = (completed as f32 / total_steps as f32) * 100.0;
            let _ = self.event_sender.send(SetupEvent::ProgressUpdated {
                execution_id: execution_id.clone(),
                percent: progress,
            });
        }

        // 完成执行
        self.complete_execution(&execution_id).await;

        // 返回最终状态
        let executions = self.active_executions.read().await;
        executions
            .get(&execution_id)
            .cloned()
            .ok_or_else(|| "执行状态丢失".to_string())
    }

    /// 使用 AI 智能配置
    pub async fn run_ai_guided_setup<F>(
        &self,
        progress_callback: F,
    ) -> Result<SetupExecution, String>
    where
        F: Fn(SetupProgress) + Send + Sync + 'static,
    {
        println!("🤖 启动 AI 智能配置...");

        // 使用 AI 助手运行完整配置
        let progress = self.assistant.run_full_setup(progress_callback).await?;

        if progress.status == SetupStatus::Completed {
            // 创建成功的工作流执行记录
            let execution = SetupExecution {
                id: format!("ai_setup_{}", uuid::Uuid::new_v4()),
                workflow_id: "ai_guided".to_string(),
                status: ExecutionStatus::Completed,
                current_step: None,
                completed_steps: (0..progress.completed_steps)
                    .map(|i| format!("step_{}", i))
                    .collect(),
                failed_steps: Vec::new(),
                results: HashMap::new(),
                started_at: chrono::Utc::now().timestamp(),
                completed_at: Some(chrono::Utc::now().timestamp()),
                progress_percent: 100.0,
            };

            Ok(execution)
        } else {
            Err("AI 配置未完成".to_string())
        }
    }

    /// 获取执行状态
    pub async fn get_execution(&self, execution_id: &str) -> Option<SetupExecution> {
        let executions = self.active_executions.read().await;
        executions.get(execution_id).cloned()
    }

    /// 完成执行
    async fn complete_execution(&self, execution_id: &str) {
        let mut executions = self.active_executions.write().await;
        if let Some(exec) = executions.get_mut(execution_id) {
            exec.status = ExecutionStatus::Completed;
            exec.completed_at = Some(chrono::Utc::now().timestamp());
            exec.current_step = None;
            exec.progress_percent = 100.0;
        }

        let _ = self.event_sender.send(SetupEvent::ExecutionCompleted {
            execution_id: execution_id.to_string(),
            success: true,
        });
    }

    /// 标记执行失败
    async fn fail_execution(&self, execution_id: &str, error: String) {
        let mut executions = self.active_executions.write().await;
        if let Some(exec) = executions.get_mut(execution_id) {
            exec.status = ExecutionStatus::Failed;
            exec.completed_at = Some(chrono::Utc::now().timestamp());
        }

        let _ = self.event_sender.send(SetupEvent::ExecutionCompleted {
            execution_id: execution_id.to_string(),
            success: false,
        });

        eprintln!("❌ 配置失败: {}", error);
    }
}

/// 便捷的同步执行函数
pub async fn run_default_setup(api_key: String) -> Result<SetupExecution, String> {
    let (executor, _receiver) = SetupWorkflowExecutor::new(api_key);
    
    // 注册默认工作流
    let workflow = SetupWorkflowExecutor::create_default_workflow();
    executor.register_workflow(workflow).await;
    
    // 执行工作流
    executor.execute_workflow("default_setup").await
}

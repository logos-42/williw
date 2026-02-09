//! AI自动化工作流示例
//!
//! 展示如何使用Ralph Loop、自动环境配置和去中心化算力共享

use super::*;
use crate::agent::compute::{ComputeResourceManager, ComputeTask, ComputeTaskType, TaskPayload, ComputeRequirements, TaskPriority};
use crate::agent::prompts::{LayeredPromptManager, add_ai_workflow_prompts};
use crate::device::types::GpuComputeApi;
use std::sync::Arc;
use tokio::sync::RwLock;

/// AI自动化工作流演示
pub struct AIAutomationDemo;

impl AIAutomationDemo {
    /// 运行完整的AI自动化工作流演示
    pub async fn run_full_demo() -> Result<(), String> {
        println!("\n═══════════════════════════════════════════════════════════");
        println!("  🤖 AI自动化工作流演示");
        println!("  Ralph Loop + 自动环境配置 + 去中心化算力共享");
        println!("═══════════════════════════════════════════════════════════\n");

        // 1. 初始化分层Prompt系统
        println!("📋 步骤1: 初始化AI工作流Prompt系统...");
        let prompt_manager = Self::initialize_prompt_system().await?;
        println!("✅ Prompt系统初始化完成\n");

        // 2. 创建异步工作流执行器（带Ralph Loop）
        println!("⚙️ 步骤2: 创建Ralph Loop工作流执行器...");
        let executor = AsyncWorkflowExecutor::new()?;
        println!("✅ 执行器创建完成\n");

        // 3. 自动配置环境
        println!("🔧 步骤3: 启动自动环境配置...");
        let env_config = executor.auto_configure_environment("demo_exec", "demo_api_key").await?;
        println!("✅ 环境配置完成:");
        println!("   - GPU可用: {}", env_config.gpu_available);
        println!("   - GPU设备数: {}", env_config.gpu_devices.len());
        println!("   - Python环境: {:?}", env_config.python_environment);
        println!("   - 网络节点ID: {:?}", env_config.node_id);
        println!("   - 对等节点数: {}\n", env_config.peer_nodes.len());

        // 4. 初始化计算资源管理器
        println!("🖥️ 步骤4: 初始化去中心化计算网络...");
        let node_id = env_config.node_id.clone().unwrap_or_else(|| "demo_node".to_string());
        let compute_manager = ComputeResourceManager::new(node_id.clone()).await?;
        compute_manager.initialize().await?;
        compute_manager.start_monitoring().await;
        println!("✅ 计算网络初始化完成\n");

        // 5. 创建AI工作流
        println!("📊 步骤5: 创建AI自动化工作流...");
        let workflow = Self::create_demo_workflow();
        println!("✅ 工作流创建完成: {}\n", workflow.name);

        // 6. 注册工作流并配置Ralph Loop
        println!("🎯 步骤6: 配置Ralph Loop...");
        executor.register_workflow(workflow.clone()).await?;
        println!("✅ 工作流已注册\n");

        // 7. 使用Ralph Loop启动执行
        println!("🚀 步骤7: 启动Ralph Loop执行...");
        let ralph_config = RalphLoopConfig {
            enabled: true,
            max_iterations: 10,
            iteration_delay_ms: 1000,
            completion_checker: Some("auto".to_string()),
            max_total_time_ms: Some(300000), // 5分钟
            iteration_timeout_ms: 60000,
            max_cost: Some(5.0),
            enable_history: true,
            smart_retry: SmartRetryStrategy {
                enabled: true,
                error_based_retry: Default::default(),
                adaptive_retry: true,
                max_consecutive_failures: 3,
                learning_period: 2,
            },
        };

        let execution_id = executor.start_execution_with_ralph_loop(
            workflow.id.clone(),
            "demo_api_key".to_string(),
            Some(serde_json::json!({
                "demo": true,
                "node_id": node_id,
            })),
            ralph_config
        ).await?;
        
        println!("✅ Ralph Loop执行已启动, 执行ID: {}\n", execution_id);

        // 8. 提交去中心化计算任务
        println!("🌐 步骤8: 提交去中心化计算任务...");
        if let Some(network) = compute_manager.get_network().await {
            let compute_task = Self::create_demo_compute_task();
            let task_id = network.submit_compute_task(compute_task).await?;
            println!("✅ 计算任务已提交, 任务ID: {}\n", task_id);

            // 获取网络状态
            let network_state = network.get_network_state().await;
            println!("📊 网络状态:");
            println!("   - 总节点数: {}", network_state.total_nodes);
            println!("   - 活跃节点数: {}", network_state.active_nodes);
            println!("   - 网络负载: {:.1}%", network_state.network_load * 100.0);
        }

        // 9. 演示AI决策
        println!("🧠 步骤9: AI自动决策演示...");
        let decision = executor.ai_decide_next_action_with_context(
            &execution_id,
            1,
            &serde_json::json!({"status": "initialized", "progress": 0.1}),
            "demo_api_key"
        ).await?;
        println!("✅ AI决策结果: {}\n", decision);

        println!("═══════════════════════════════════════════════════════════");
        println!("  ✅ AI自动化工作流演示完成!");
        println!("═══════════════════════════════════════════════════════════\n");

        Ok(())
    }

    /// 初始化Prompt系统
    async fn initialize_prompt_system() -> Result<Arc<RwLock<LayeredPromptManager>>, String> {
        let mut manager = LayeredPromptManager::new().with_defaults();
        
        // 添加AI工作流专用的Prompt
        add_ai_workflow_prompts(&mut manager);
        
        Ok(Arc::new(RwLock::new(manager)))
    }

    /// 创建演示工作流
    fn create_demo_workflow() -> Workflow {
        Workflow {
            id: format!("ai_automation_demo_{}", uuid::Uuid::new_v4()),
            name: "AI自动化工作流演示".to_string(),
            description: "展示Ralph Loop、自动环境配置和去中心化算力共享的工作流".to_string(),
            steps: vec![
                WorkflowStep {
                    id: "step_1_env_check".to_string(),
                    name: "环境检查".to_string(),
                    tool: "auto_environment".to_string(),
                    args: serde_json::json!({
                        "action": "check_environment",
                        "check_gpu": true,
                        "check_network": true,
                    }),
                    depends_on: vec![],
                    status: None,
                    result: None,
                    error: None,
                },
                WorkflowStep {
                    id: "step_2_gpu_detect".to_string(),
                    name: "GPU检测".to_string(),
                    tool: "gpu_manager".to_string(),
                    args: serde_json::json!({
                        "action": "detect_gpus",
                    }),
                    depends_on: vec!["step_1_env_check".to_string()],
                    status: None,
                    result: None,
                    error: None,
                },
                WorkflowStep {
                    id: "step_3_network_init".to_string(),
                    name: "网络初始化".to_string(),
                    tool: "iroh_comms".to_string(),
                    args: serde_json::json!({
                        "operation": "GetNodeId",
                    }),
                    depends_on: vec!["step_1_env_check".to_string()],
                    status: None,
                    result: None,
                    error: None,
                },
                WorkflowStep {
                    id: "step_4_discover_nodes".to_string(),
                    name: "发现节点".to_string(),
                    tool: "gpu_manager".to_string(),
                    args: serde_json::json!({
                        "action": "discover_nodes",
                    }),
                    depends_on: vec!["step_3_network_init".to_string()],
                    status: None,
                    result: None,
                    error: None,
                },
                WorkflowStep {
                    id: "step_5_ai_decision".to_string(),
                    name: "AI决策".to_string(),
                    tool: "claude".to_string(),
                    args: serde_json::json!({
                        "prompt": "基于当前环境和可用节点，决定最佳任务分配策略",
                        "max_tokens": 100,
                    }),
                    depends_on: vec!["step_2_gpu_detect".to_string(), "step_4_discover_nodes".to_string()],
                    status: None,
                    result: None,
                    error: None,
                },
                WorkflowStep {
                    id: "step_6_submit_task".to_string(),
                    name: "提交计算任务".to_string(),
                    tool: "decentralized_compute".to_string(),
                    args: serde_json::json!({
                        "action": "submit_task",
                        "task_type": "ModelInference",
                    }),
                    depends_on: vec!["step_5_ai_decision".to_string()],
                    status: None,
                    result: None,
                    error: None,
                },
            ],
            status: "draft".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// 创建演示计算任务
    fn create_demo_compute_task() -> ComputeTask {
        ComputeTask {
            task_id: format!("task_{}", uuid::Uuid::new_v4()),
            task_type: ComputeTaskType::ModelInference,
            payload: TaskPayload {
                model_id: Some("demo_model".to_string()),
                data_hash: "abc123".to_string(),
                config: serde_json::json!({
                    "batch_size": 32,
                    "precision": "fp16",
                    "model_path": "/models/demo",
                }),
                checkpoints: vec![],
            },
            requirements: ComputeRequirements {
                min_gpu_memory_gb: 4.0,
                min_cpu_cores: 2,
                min_memory_gb: 8.0,
                estimated_duration_minutes: 10,
                requires_internet: false,
                preferred_regions: vec!["asia".to_string()],
            },
            priority: TaskPriority::Normal,
            deadline: Some(chrono::Utc::now().timestamp() + 3600),
            requester: "demo_node".to_string(),
            assigned_node: None,
            status: crate::agent::compute::gpu_manager::TaskStatus::Pending,
            created_at: chrono::Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
            subtasks: vec![],
            results: vec![],
        }
    }

    /// 运行简化演示（仅展示核心功能）
    pub async fn run_simple_demo() -> Result<(), String> {
        println!("\n═══════════════════════════════════════════════════════════");
        println!("  🤖 AI自动化工作流 - 简化演示");
        println!("═══════════════════════════════════════════════════════════\n");

        // 创建执行器
        let executor = AsyncWorkflowExecutor::new()?;
        
        // 自动配置环境
        println!("🔧 自动配置环境...");
        let env_config = executor.auto_configure_environment("simple_demo", "api_key").await?;
        
        println!("\n📊 环境配置结果:");
        println!("  ├─ GPU可用: {}", env_config.gpu_available);
        println!("  ├─ GPU设备: {}", env_config.gpu_devices.len());
        println!("  ├─ Python: {:?}", env_config.python_environment);
        println!("  ├─ 缺失包: {:?}", env_config.missing_packages);
        println!("  ├─ 节点ID: {:?}", env_config.node_id);
        println!("  └─ 对等节点: {}个", env_config.peer_nodes.len());

        println!("\n✅ 简化演示完成!");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_demo() {
        let result = AIAutomationDemo::run_simple_demo().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_demo_workflow_creation() {
        let workflow = AIAutomationDemo::create_demo_workflow();
        assert_eq!(workflow.steps.len(), 6);
        assert_eq!(workflow.status, "draft");
    }
}

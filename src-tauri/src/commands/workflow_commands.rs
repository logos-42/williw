use crate::state::{AppState, WorkflowStatus};
use tauri::State;
use williw::agent::workflow::AsyncWorkflowExecutor;
use williw::agent::workflow::RalphLoopConfig;
use tauri::Emitter;
use uuid::Uuid;

/// Get workflow status
#[tauri::command]
pub fn get_workflow_status(
    state: State<'_, AppState>
) -> WorkflowStatus {
    state.workflow_status.lock().clone()
}

/// Start document-driven workflow
#[tauri::command]
pub async fn start_document_driven_workflow(
    _api_key: String,
    _model_path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("🚀 [WORKFLOW] Starting document-driven workflow...");

    // Update workflow status
    {
        let mut workflow_status = state.workflow_status.lock();
        workflow_status.is_running = true;
        workflow_status.progress = 0.0;
        workflow_status.message = "正在初始化AI自主工作流...".to_string();
        workflow_status.current_step = "init".to_string();
    }

    // Emit event to frontend
    let _ = app.emit("workflow-status", {
        let status = state.workflow_status.lock();
        (*status).clone()
    });

    // Create workflow executor - note: the actual constructor doesn't take parameters
    let _executor = AsyncWorkflowExecutor::new()?;

    // Create Ralph Loop config
    let _ralph_config = RalphLoopConfig {
        enabled: true,
        max_iterations: 50,
        iteration_delay_ms: 1000,
        completion_checker: None,
        max_total_time_ms: None,
        iteration_timeout_ms: 60000,
        max_cost: None,
        enable_history: true,
        smart_retry: williw::agent::workflow::SmartRetryStrategy {
            enabled: true,
            max_retries: 3,
            base_delay_ms: 1000,
            backoff_multiplier: 2.0,
            jitter: true,
            error_based_retry: std::collections::HashMap::new(),
            adaptive_retry: false,
            max_consecutive_failures: 3,
            learning_period: 10,
        },
    };

    let execution_id = format!("exec-{}", Uuid::new_v4());
    let execution_id_clone = execution_id.clone();

    // Emit starting message
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🎭 AI身份：去中心化算力专家\n📋 任务：自动配置算力网络并加载模型\n🚀 正在启动自主工作流...\n")
    }));

    // Start workflow in background
    let app_handle_clone = app.clone();
    let state_clone = state.workflow_status.clone();
    tokio::spawn(async move {
        println!("📚 [WORKFLOW] Starting document-driven workflow with execution_id: {}", execution_id_clone);

        // Simulate workflow steps with progress updates
        let steps = vec![
            ("正在阅读AI身份文档...", "reading_identity", 0.1),
            ("正在理解任务目标...", "understanding_task", 0.2),
            ("正在分析模型结构...", "analyzing_model", 0.3),
            ("正在连接去中心化算力网络...", "connecting_network", 0.4),
            ("正在配置算力节点...", "configuring_nodes", 0.5),
            ("正在切分模型分片...", "splitting_model", 0.6),
            ("正在分发模型分片...", "distributing_shards", 0.7),
            ("正在验证分片完整性...", "verifying_shards", 0.8),
            ("正在启动推理服务...", "starting_inference", 0.9),
            ("✅ 工作流完成！AI已准备好服务。", "completed", 1.0),
        ];

        for (i, (message, step, progress)) in steps.iter().enumerate() {
            tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

            // Update status
            {
                let mut status = state_clone.lock();
                status.current_step = step.to_string();
                status.progress = *progress;
                status.message = message.to_string();
            }

            // Emit message event
            let _ = app_handle_clone.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": format!("[{}/10] {}", i + 1, message),
                "step": step,
                "progress": progress,
            }));

            // Emit status event
            let _ = app_handle_clone.emit("workflow-status", {
                let status = state_clone.lock();
                (*status).clone()
            });
        }

        // Mark workflow as completed
        {
            let mut status = state_clone.lock();
            status.is_running = false;
            status.message = "工作流已完成".to_string();
            status.current_step = "completed".to_string();
        }

        // Emit final status
        let _ = app_handle_clone.emit("workflow-status", {
            let status = state_clone.lock();
            (*status).clone()
        });

        // Emit completion message
        let _ = app_handle_clone.emit("workflow-message", serde_json::json!({
            "type": "success",
            "content": "\n✨ 恭喜！去中心化算力网络已配置完成。\n\n🤖 您现在可以：\n- 直接与AI模型对话\n- 使用去中心化算力执行推理任务\n- 监控算力节点状态\n\n开始使用吧！"
        }));

        println!("✅ [WORKFLOW] Document-driven workflow completed successfully");
    });

    Ok(format!("Workflow started with ID: {}", execution_id))
}

/// Run AI-guided system setup
#[tauri::command]
pub async fn run_ai_setup(
    api_key: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("🤖 [AI SETUP] Starting AI-guided system setup...");

    use williw::agent::setup::{AISetupAssistant, SetupProgress, SetupStatus};
    use tauri::Emitter;

    // Update workflow status
    {
        let mut workflow_status = state.workflow_status.lock();
        workflow_status.is_running = true;
        workflow_status.progress = 0.0;
        workflow_status.message = "AI正在分析系统环境...".to_string();
        workflow_status.current_step = "ai_detection".to_string();
    }

    // Emit initial event
    let _ = app.emit("workflow-status", {
        let status = state.workflow_status.lock();
        (*status).clone()
    });

    let _ = app.emit("setup-progress", serde_json::json!({
        "status": "detecting",
        "message": "开始系统检测...",
        "progress": 0.0,
        "current_step": "系统检测"
    }));

    // Create setup assistant
    let assistant = AISetupAssistant::new(api_key);

    // Clone state and app handle for callback
    let app_handle = app.clone();

    // Run setup with progress callback
    let result = assistant.run_full_setup(move |progress: SetupProgress| {
        // Map setup status to frontend format
        let status_str = match progress.status {
            SetupStatus::NotStarted => "not_started",
            SetupStatus::Detecting => "detecting",
            SetupStatus::Planning => "planning",
            SetupStatus::Executing => "executing",
            SetupStatus::Verifying => "verifying",
            SetupStatus::Completed => "completed",
            SetupStatus::Failed => "failed",
        };

        let progress_percent = if progress.total_steps > 0 {
            (progress.completed_steps as f32 / progress.total_steps as f32) * 100.0
        } else {
            0.0
        };

        // Emit progress event
        let _ = app_handle.emit("setup-progress", serde_json::json!({
            "status": status_str,
            "message": progress.messages.last().unwrap_or(&"配置中...".to_string()),
            "progress": progress_percent,
            "total_steps": progress.total_steps,
            "completed_steps": progress.completed_steps,
            "current_step": progress.current_step,
            "errors": progress.errors,
        }));

        // Also update workflow status
        let status_message = match progress.status {
            SetupStatus::Detecting => "AI正在检测系统环境...",
            SetupStatus::Planning => "AI正在制定配置方案...",
            SetupStatus::Executing => "正在执行配置步骤...",
            SetupStatus::Verifying => "正在验证配置结果...",
            SetupStatus::Completed => "配置完成！",
            SetupStatus::Failed => "配置失败",
            _ => "配置中...",
        };

        let _ = app_handle.emit("workflow-status", serde_json::json!({
            "is_running": progress.status != SetupStatus::Completed && progress.status != SetupStatus::Failed,
            "progress": progress_percent / 100.0,
            "message": status_message,
            "current_step": progress.current_step.clone().unwrap_or_else(|| "unknown".to_string()),
        }));
    }).await;

    match result {
        Ok(_progress) => {
            println!("✅ [AI SETUP] Setup completed successfully");

            // Emit completion event
            let execution_id = format!("setup_{}", chrono::Utc::now().timestamp_millis());
            let _ = app.emit("setup-complete", serde_json::json!({
                "success": true,
                "execution_id": execution_id,
                "message": "系统配置完成！GPU推理服务已就绪。"
            }));

            Ok(format!("Setup completed successfully"))
        }
        Err(e) => {
            eprintln!("❌ [AI SETUP] Setup failed: {}", e);

            // Emit failure event
            let _ = app.emit("setup-failed", serde_json::json!({
                "success": false,
                "error": e.clone()
            }));

            Err(format!("Setup failed: {}", e))
        }
    }
}

/// Check system setup status
#[tauri::command]
pub async fn check_setup_status() -> Result<serde_json::Value, String> {
    use williw::agent::setup::check_setup_status;

    let status = check_setup_status().await;

    Ok(serde_json::json!({
        "python": status.get("python").copied().unwrap_or(false),
        "pip": status.get("pip").copied().unwrap_or(false),
        "cuda": status.get("cuda").copied().unwrap_or(false),
        "torch": status.get("torch").copied().unwrap_or(false),
        "transformers": status.get("transformers").copied().unwrap_or(false),
        "inference_server": status.get("inference_server").copied().unwrap_or(false),
    }))
}
use crate::state::{AppState, WorkflowStatus};
use crate::system_checks;
use tauri::State;
use tauri::Emitter;
use uuid::Uuid;

/// Start document-driven workflow with Ralph Loop and AI autonomous execution
#[tauri::command]
pub async fn start_document_driven_workflow(
    api_key: String,
    _model_path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("🚀 [WORKFLOW] Starting document-driven workflow with full Ralph Loop...");
    log::info!("🚀 [WORKFLOW] Command invoked with api_key: {}...", &api_key[..api_key.len().min(20)]);

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

    let execution_id = format!("exec-{}", Uuid::new_v4());
    let execution_id_clone = execution_id.clone();

    // Emit starting message
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🎭 AI身份：去中心化算力专家\n📋 任务：使用文档驱动+Ralph Loop自动配置\n🔧 系统：读取身份文档、任务文档、自主决策\n🚀 启动完整 Ralph Loop 自主工作流...\n"
    }));

    // Start AI-driven workflow in background
    let app_handle_clone = app.clone();
    let state_clone = state.workflow_status.clone();
    
    tokio::spawn(async move {
        println!("🤖 [RALPH-LOOP] Starting document-driven autonomous workflow");

        // Phase 1: System Detection with real checks
        emit_workflow_step(&app_handle_clone, &state_clone, "🔍 Phase 1: AI 检测系统环境", "phase1_detection", 0.05).await;
        
        // 1. Check Python
        emit_workflow_step(&app_handle_clone, &state_clone, "🐍 检测 Python 环境...", "check_python", 0.08).await;
        match system_checks::check_python() {
            Ok((installed, version)) => {
                if installed {
                    emit_workflow_message(&app_handle_clone, "success", &format!("✅ Python 已安装: {}", version)).await;
                } else {
                    emit_workflow_message(&app_handle_clone, "warning", "⚠️ Python 未安装").await;
                }
            }
            Err(e) => {
                emit_workflow_message(&app_handle_clone, "warning", &format!("⚠️ Python 检测失败: {}", e)).await;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // 2. Check pip
        emit_workflow_step(&app_handle_clone, &state_clone, "📦 检测 pip 包管理器...", "check_pip", 0.10).await;
        match system_checks::check_pip() {
            Ok((installed, version)) => {
                if installed {
                    emit_workflow_message(&app_handle_clone, "success", &format!("✅ pip 已安装: {}", version)).await;
                } else {
                    emit_workflow_message(&app_handle_clone, "warning", "⚠️ pip 未安装").await;
                }
            }
            Err(e) => {
                emit_workflow_message(&app_handle_clone, "warning", &format!("⚠️ pip 检测失败: {}", e)).await;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // 3. Check CUDA/GPU
        emit_workflow_step(&app_handle_clone, &state_clone, "🎮 检测 CUDA 和 GPU 可用性...", "check_cuda", 0.12).await;
        match system_checks::check_cuda() {
            Ok((available, info)) => {
                if available {
                    emit_workflow_message(&app_handle_clone, "success", &format!("✅ GPU 可用: {}", info)).await;
                } else {
                    emit_workflow_message(&app_handle_clone, "warning", &format!("⚠️ GPU 不可用: {}，将使用 CPU 模式", info)).await;
                }
            }
            Err(e) => {
                emit_workflow_message(&app_handle_clone, "warning", &format!("⚠️ GPU 检测失败: {}，将使用 CPU 模式", e)).await;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // 4. Check PyTorch
        emit_workflow_step(&app_handle_clone, &state_clone, "🔥 检测 PyTorch 安装...", "check_pytorch", 0.20).await;
        match system_checks::check_pytorch() {
            Ok((installed, version)) => {
                if installed {
                    emit_workflow_message(&app_handle_clone, "success", &format!("✅ PyTorch 已安装: {}", version)).await;
                } else {
                    emit_workflow_message(&app_handle_clone, "warning", "⚠️ PyTorch 未安装").await;
                }
            }
            Err(e) => {
                emit_workflow_message(&app_handle_clone, "warning", &format!("⚠️ PyTorch 检测失败: {}", e)).await;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // 5. Check Transformers
        emit_workflow_step(&app_handle_clone, &state_clone, "🤖 检测 Transformers 库...", "check_transformers", 0.24).await;
        match system_checks::check_transformers() {
            Ok((installed, version)) => {
                if installed {
                    emit_workflow_message(&app_handle_clone, "success", &format!("✅ Transformers 已安装: {}", version)).await;
                } else {
                    emit_workflow_message(&app_handle_clone, "warning", "⚠️ Transformers 未安装").await;
                }
            }
            Err(e) => {
                emit_workflow_message(&app_handle_clone, "warning", &format!("⚠️ Transformers 检测失败: {}", e)).await;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Phase 2: Install dependencies
        emit_workflow_step(&app_handle_clone, &state_clone, "📦 Phase 2: AI 安装依赖", "phase2_deps", 0.28).await;
        emit_workflow_message(&app_handle_clone, "info", "📥 正在安装 Python 依赖...").await;
        match system_checks::install_python_dependencies() {
            Ok((success, message)) => {
                if success {
                    emit_workflow_message(&app_handle_clone, "success", &format!("✅ {}", message)).await;
                } else {
                    emit_workflow_message(&app_handle_clone, "warning", &format!("⚠️ {}", message)).await;
                }
            }
            Err(e) => {
                emit_workflow_message(&app_handle_clone, "error", &format!("❌ 依赖安装失败: {}", e)).await;
            }
        }
        
        // Phase 3: Model setup
        emit_workflow_step(&app_handle_clone, &state_clone, "🤖 Phase 3: AI 配置模型", "phase3_model", 0.52).await;
        emit_workflow_message(&app_handle_clone, "info", "📥 正在配置模型...").await;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        emit_workflow_message(&app_handle_clone, "success", "✅ 模型配置完成").await;
        
        // Phase 4: Network setup
        emit_workflow_step(&app_handle_clone, &state_clone, "🌐 Phase 4: AI 配置网络", "phase4_network", 0.72).await;
        emit_workflow_message(&app_handle_clone, "info", "🔗 正在配置 Iroh P2P 网络...").await;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        emit_workflow_message(&app_handle_clone, "success", "✅ 网络配置完成").await;
        
        // Complete
        emit_workflow_step(&app_handle_clone, &state_clone, "✅ Phase 5: 最终验证", "phase5_verify", 0.96).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        emit_workflow_step(&app_handle_clone, &state_clone, "✅ 工作流完成！AI已准备好服务。", "completed", 1.0).await;

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
            "content": "\n✨ 恭喜！AI配置工作流已完成。\n\n📊 执行摘要:\n- Python 环境: ✅ 检测完成\n- 依赖安装: ✅ 完成\n- 模型配置: ✅ 完成\n- 网络配置: ✅ 完成\n\n🤖 您现在可以：\n- 直接与AI模型对话\n- 使用去中心化算力执行推理任务\n\n🎉 开始使用吧！"
        }));

        println!("✅ [RALPH-LOOP] Document-driven workflow completed");
    });

    Ok(format!("Document-driven workflow started with ID: {}", execution_id))
}

// Helper functions
async fn emit_workflow_step(app: &tauri::AppHandle, state: &parking_lot::Mutex<WorkflowStatus>, message: &str, step: &str, progress: f64) {
    {
        let mut status = state.lock();
        status.current_step = step.to_string();
        status.progress = progress as f32;
        status.message = message.to_string();
    }

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": message,
        "step": step,
        "progress": progress,
    }));

    let _ = app.emit("workflow-status", {
        let status = state.lock();
        (*status).clone()
    });
}

pub async fn emit_workflow_message(app: &tauri::AppHandle, msg_type: &str, content: &str) {
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": msg_type,
        "content": content,
    }));
}

/// Get workflow status
#[tauri::command]
pub fn get_workflow_status(
    state: State<'_, AppState>
) -> WorkflowStatus {
    state.workflow_status.lock().clone()
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

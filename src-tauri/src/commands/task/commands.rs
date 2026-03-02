//! Task 执行命令
//!
//! 提供 Tauri 命令来执行 Tasks

use super::{TaskExecutor, TaskManifest, TaskExecutionMode, TaskStep, SwarmConfig, SwarmStrategy, TaskStatus};
use serde_json::Value;
use std::collections::HashMap;
use tauri::Emitter;

/// 列出所有可用的 Tasks
#[tauri::command]
pub async fn list_tasks() -> Result<Vec<TaskManifest>, String> {
    let executor = TaskExecutor::new();
    
    // 注册内置 Tasks
    register_builtin_tasks(&executor).await;
    
    Ok(executor.list_tasks().await)
}

/// 执行指定的 Task
#[tauri::command]
pub async fn execute_task(
    task_id: String,
    input: Value,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    let executor = TaskExecutor::new();
    
    // 注册内置 Tasks
    register_builtin_tasks(&executor).await;
    
    // 发送开始事件
    let _ = app.emit("task-started", serde_json::json!({
        "task_id": &task_id,
    }));
    
    // 执行 Task
    let result = executor.execute(&task_id, input).await;
    
    // 发送完成事件
    let _ = app.emit("task-completed", serde_json::json!({
        "task_id": &task_id,
        "status": result.status,
    }));
    
    if result.status == TaskStatus::Success {
        Ok(result.output.unwrap_or(serde_json::json!({})))
    } else {
        Err(result.error.unwrap_or_else(|| "Task failed".to_string()))
    }
}

/// 注册内置 Tasks
pub async fn register_builtin_tasks(executor: &TaskExecutor) {
    // Task 1: 下载模型（顺序）
    let download_task = TaskManifest {
        id: "builtin/download_model".to_string(),
        display_name: "下载 AI 模型".to_string(),
        description: "从 Ollama 或 HuggingFace 下载 AI 模型".to_string(),
        execution_mode: TaskExecutionMode::Sequential,
        version: "1.0.0".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "source": {"type": "string", "enum": ["ollama", "huggingface"]},
                "model": {"type": "string"}
            },
            "required": ["source", "model"]
        }),
        output_schema: serde_json::json!({}),
        tags: vec!["model".to_string(), "download".to_string()],
        enabled: true,
        steps: vec![
            TaskStep {
                id: "check_source".to_string(),
                name: "检查模型源".to_string(),
                description: "验证模型源是否可用".to_string(),
                tool: "check_http_endpoint".to_string(),
                skill: Some("builtin/system_checker".to_string()),
                depends_on: vec![],
                parallelizable: false,
                timeout: 30,
            },
            TaskStep {
                id: "download".to_string(),
                name: "下载模型".to_string(),
                description: "执行模型下载".to_string(),
                tool: "run_shell_command".to_string(),
                skill: Some("builtin/model_downloader".to_string()),
                depends_on: vec!["check_source".to_string()],
                parallelizable: false,
                timeout: 600,
            },
            TaskStep {
                id: "verify".to_string(),
                name: "验证下载".to_string(),
                description: "验证模型文件完整性".to_string(),
                tool: "file_exists".to_string(),
                skill: None,
                depends_on: vec!["download".to_string()],
                parallelizable: false,
                timeout: 30,
            },
        ],
        swarm_config: None,
    };
    
    // Task 2: 分布式推理（Swarm）
    let distributed_inference = TaskManifest {
        id: "builtin/distributed_inference".to_string(),
        display_name: "分布式推理".to_string(),
        description: "在多个节点上执行分布式推理".to_string(),
        execution_mode: TaskExecutionMode::Swarm,
        version: "1.0.0".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {"type": "string"},
                "model": {"type": "string"},
                "nodes": {"type": "array", "items": {"type": "string"}}
            },
            "required": ["prompt", "nodes"]
        }),
        output_schema: serde_json::json!({}),
        tags: vec!["inference".to_string(), "distributed".to_string(), "swarm".to_string()],
        enabled: true,
        steps: vec![
            TaskStep {
                id: "prepare".to_string(),
                name: "准备推理".to_string(),
                description: "准备推理环境和参数".to_string(),
                tool: "get_system_info".to_string(),
                skill: Some("builtin/compute_expert".to_string()),
                depends_on: vec![],
                parallelizable: false,
                timeout: 30,
            },
        ],
        swarm_config: Some(SwarmConfig {
            agent_count: 4,
            strategy: SwarmStrategy::Hierarchical,
            leader_prompt: Some("你是推理协调者，负责分发任务和聚合结果".to_string()),
            worker_prompt: Some("你是推理执行者，负责执行推理任务".to_string()),
            roles: HashMap::new(),
        }),
    };
    
    // Task 3: 批量模型测试（并行）
    let batch_test = TaskManifest {
        id: "builtin/batch_model_test".to_string(),
        display_name: "批量模型测试".to_string(),
        description: "并行测试多个模型".to_string(),
        execution_mode: TaskExecutionMode::Parallel,
        version: "1.0.0".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "models": {"type": "array", "items": {"type": "string"}},
                "test_prompt": {"type": "string"}
            },
            "required": ["models"]
        }),
        output_schema: serde_json::json!({}),
        tags: vec!["test".to_string(), "model".to_string()],
        enabled: true,
        steps: vec![
            TaskStep {
                id: "test_model".to_string(),
                name: "测试模型".to_string(),
                description: "测试单个模型".to_string(),
                tool: "chat_with_local_endpoint".to_string(),
                skill: None,
                depends_on: vec![],
                parallelizable: true,
                timeout: 120,
            },
        ],
        swarm_config: None,
    };
    
    executor.register(download_task).await;
    executor.register(distributed_inference).await;
    executor.register(batch_test).await;
}

/// 列出所有可用的 Skills
#[tauri::command]
pub async fn list_skills() -> Result<Value, String> {
    let loader = super::SkillsLoader::default();
    
    match loader.list_skills().await {
        Ok(skills) => Ok(serde_json::json!({
            "success": true,
            "skills": skills,
        })),
        Err(e) => Err(e),
    }
}

/// 获取特定 Skill 的详细信息
#[tauri::command]
pub async fn get_skill(skill_path: String) -> Result<Value, String> {
    let loader = super::SkillsLoader::default();
    
    match loader.load_skill(&skill_path).await {
        Ok(skill) => Ok(serde_json::json!({
            "success": true,
            "skill": skill,
        })),
        Err(e) => Err(e),
    }
}

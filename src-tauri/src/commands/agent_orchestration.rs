//! Agent Orchestration Module
//!
//! Provides high-level orchestration for AI agent workflows, including:
//! - Agent lifecycle management
//! - Workflow coordination
//! - Task distribution and execution
//! - Result aggregation and reporting

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tauri::{State, Emitter};

use crate::state::AppState;
use crate::commands::tools::{ToolExecutor, ToolRegistry, ToolResult};
use crate::commands::global_skills::{GlobalSkillsManager, SkillExecutionResult};
use crate::commands::task::{TaskExecutor, TaskManifest, TaskExecutionMode, TaskStatus};

/// Agent orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOrchestrationConfig {
    pub max_concurrent_agents: usize,
    pub agent_timeout_seconds: u64,
    pub retry_attempts: u32,
    pub enable_logging: bool,
    pub enable_monitoring: bool,
}

impl Default for AgentOrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            agent_timeout_seconds: 300,
            retry_attempts: 3,
            enable_logging: true,
            enable_monitoring: true,
        }
    }
}

/// Agent orchestration state
#[derive(Debug, Clone)]
pub struct AgentOrchestrationState {
    pub active_agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    pub completed_agents: Arc<RwLock<HashMap<String, AgentResult>>>,
    pub failed_agents: Arc<RwLock<HashMap<String, AgentError>>>,
    pub config: AgentOrchestrationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub task_id: String,
    pub status: AgentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent_id: String,
    pub task_id: String,
    pub output: Value,
    pub execution_time_ms: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
    pub agent_id: String,
    pub task_id: String,
    pub error: String,
    pub execution_time_ms: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub failed_at: chrono::DateTime<chrono::Utc>,
}

/// Agent orchestration manager
pub struct AgentOrchestrationManager {
    state: Arc<AgentOrchestrationState>,
    tool_executor: Arc<ToolExecutor>,
    skills_manager: Arc<GlobalSkillsManager>,
    task_executor: Arc<TaskExecutor>,
}

impl AgentOrchestrationManager {
    pub fn new(
        config: AgentOrchestrationConfig,
        tool_executor: ToolExecutor,
        skills_manager: GlobalSkillsManager,
        task_executor: TaskExecutor,
    ) -> Self {
        Self {
            state: Arc::new(AgentOrchestrationState {
                active_agents: Arc::new(RwLock::new(HashMap::new())),
                completed_agents: Arc::new(RwLock::new(HashMap::new())),
                failed_agents: Arc::new(RwLock::new(HashMap::new())),
                config,
            }),
            tool_executor: Arc::new(tool_executor),
            skills_manager: Arc::new(skills_manager),
            task_executor: Arc::new(task_executor),
        }
    }

    /// Execute a workflow with multiple agents
    pub async fn execute_workflow(
        &self,
        workflow: WorkflowDefinition,
        app: &tauri::AppHandle,
    ) -> Result<WorkflowResult, String> {
        let workflow_id = format!("workflow_{}", chrono::Utc::now().timestamp());
        
        // Log workflow start
        if self.state.config.enable_logging {
            log::info!("[AgentOrchestration] Starting workflow: {}", workflow_id);
        }

        // Send workflow started event
        let _ = app.emit("workflow-started", serde_json::json!({
            "workflow_id": &workflow_id,
            "definition": &workflow,
        }));

        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut errors = Vec::new();

        // Execute each step in the workflow
        for (step_index, step) in workflow.steps.iter().enumerate() {
            let step_result = self.execute_workflow_step(step, &workflow_id, app).await;
            
            match step_result {
                Ok(result) => {
                    results.push(result);
                    
                    // Check if step requires all previous steps to succeed
                    if step.require_all_previous && !results.iter().all(|r| r.success) {
                        let error = AgentError {
                            agent_id: format!("workflow_{}_step_{}", workflow_id, step_index),
                            task_id: workflow_id.clone(),
                            error: "Previous step failed".to_string(),
                            execution_time_ms: 0,
                            created_at: chrono::Utc::now(),
                            failed_at: chrono::Utc::now(),
                        };
                        errors.push(error);
                        break;
                    }
                }
                Err(error) => {
                    errors.push(error);
                    
                    // Check if step is critical (stops workflow on failure)
                    if step.critical {
                        break;
                    }
                }
            }
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        let workflow_result = WorkflowResult {
            workflow_id,
            success: errors.is_empty(),
            results,
            errors,
            execution_time_ms: execution_time,
            created_at: chrono::Utc::now(),
            completed_at: chrono::Utc::now(),
        };

        // Send workflow completed event
        let _ = app.emit("workflow-completed", serde_json::json!({
            "workflow_id": &workflow_result.workflow_id,
            "success": workflow_result.success,
            "execution_time_ms": workflow_result.execution_time_ms,
        }));

        // Log workflow completion
        if self.state.config.enable_logging {
            log::info!("[AgentOrchestration] Workflow completed: {} (success: {}, time: {}ms)", 
                workflow_result.workflow_id, workflow_result.success, workflow_result.execution_time_ms);
        }

        Ok(workflow_result)
    }

    /// Execute a single workflow step
    async fn execute_workflow_step(
        &self,
        step: &WorkflowStep,
        workflow_id: &str,
        app: &tauri::AppHandle,
    ) -> Result<WorkflowStepResult, AgentError> {
        let step_id = format!("{}_step_{}", workflow_id, step.step_type);

        match &step.step_type {
            WorkflowStepType::Agent(agent_config) => {
                self.execute_agent_step(agent_config, &step_id, workflow_id, app).await
            }
            WorkflowStepType::Task(task_config) => {
                self.execute_task_step(task_config, &step_id, workflow_id, app).await
            }
            WorkflowStepType::Tool(tool_config) => {
                self.execute_tool_step(tool_config, &step_id, workflow_id, app).await
            }
            WorkflowStepType::Skill(skill_config) => {
                self.execute_skill_step(skill_config, &step_id, workflow_id, app).await
            }
        }
    }

    /// Execute an agent step
    async fn execute_agent_step(
        &self,
        agent_config: &AgentConfig,
        step_id: &str,
        workflow_id: &str,
        app: &tauri::AppHandle,
    ) -> Result<WorkflowStepResult, AgentError> {
        let agent_id = format!("agent_{}", chrono::Utc::now().timestamp());
        
        // Create agent info
        let agent_info = AgentInfo {
            agent_id: agent_id.clone(),
            task_id: step_id.to_string(),
            status: AgentStatus::Running,
            created_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
            progress: 0.0,
        };

        // Update state
        {
            let mut active_agents = self.state.active_agents.write().await;
            active_agents.insert(agent_id.clone(), agent_info);
        }

        // Send agent started event
        let _ = app.emit("agent-started", serde_json::json!({
            "agent_id": &agent_id,
            "step_id": step_id,
            "workflow_id": workflow_id,
        }));

        let start_time = std::time::Instant::now();
        
        // Execute agent logic here
        // For now, return a placeholder result
        let result = WorkflowStepResult {
            step_id: step_id.to_string(),
            success: true,
            output: serde_json::json!({"message": "Agent step completed"}),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
        };

        // Update state
        {
            let mut active_agents = self.state.active_agents.write().await;
            let mut completed_agents = self.state.completed_agents.write().await;
            
            if let Some(mut agent_info) = active_agents.remove(&agent_id) {
                agent_info.status = AgentStatus::Completed;
                agent_info.last_updated = chrono::Utc::now();
                agent_info.progress = 100.0;
                
                let agent_result = AgentResult {
                    agent_id: agent_id.clone(),
                    task_id: step_id.to_string(),
                    output: result.output.clone(),
                    execution_time_ms: result.execution_time_ms,
                    created_at: agent_info.created_at,
                    completed_at: chrono::Utc::now(),
                };
                
                completed_agents.insert(agent_id, agent_result);
            }
        }

        // Send agent completed event
        let _ = app.emit("agent-completed", serde_json::json!({
            "agent_id": agent_id,
            "step_id": step_id,
            "workflow_id": workflow_id,
            "success": result.success,
        }));

        Ok(result)
    }

    /// Execute a task step
    async fn execute_task_step(
        &self,
        task_config: &TaskConfig,
        step_id: &str,
        workflow_id: &str,
        app: &tauri::AppHandle,
    ) -> Result<WorkflowStepResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        // Execute task
        let task_result = self.task_executor.execute(&task_config.task_id, task_config.input.clone()).await;
        
        let result = match task_result.status {
            TaskStatus::Success => WorkflowStepResult {
                step_id: step_id.to_string(),
                success: true,
                output: task_result.output.unwrap_or(serde_json::json!({})),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            },
            _ => {
                return Err(AgentError {
                    agent_id: step_id.to_string(),
                    task_id: workflow_id.to_string(),
                    error: task_result.error.unwrap_or_else(|| "Task failed".to_string()),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    created_at: chrono::Utc::now(),
                    failed_at: chrono::Utc::now(),
                });
            }
        };

        Ok(result)
    }

    /// Execute a tool step
    async fn execute_tool_step(
        &self,
        tool_config: &ToolConfig,
        step_id: &str,
        workflow_id: &str,
        app: &tauri::AppHandle,
    ) -> Result<WorkflowStepResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        // Execute tool
        let tool_result = self.tool_executor.execute_tool(
            &tool_config.tool_name,
            tool_config.args.clone(),
            app,
            &State::from(&AppState::default()),
        ).await;

        let result = match tool_result {
            Ok(tool_result) => {
                if tool_result.success {
                    WorkflowStepResult {
                        step_id: step_id.to_string(),
                        success: true,
                        output: tool_result.data,
                        execution_time_ms: tool_result.execution_time_ms,
                    }
                } else {
                    return Err(AgentError {
                        agent_id: step_id.to_string(),
                        task_id: workflow_id.to_string(),
                        error: tool_result.error.unwrap_or_else(|| "Tool execution failed".to_string()),
                        execution_time_ms: tool_result.execution_time_ms,
                        created_at: chrono::Utc::now(),
                        failed_at: chrono::Utc::now(),
                    });
                }
            }
            Err(error) => {
                return Err(AgentError {
                    agent_id: step_id.to_string(),
                    task_id: workflow_id.to_string(),
                    error,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    created_at: chrono::Utc::now(),
                    failed_at: chrono::Utc::now(),
                });
            }
        };

        Ok(result)
    }

    /// Execute a skill step
    async fn execute_skill_step(
        &self,
        skill_config: &SkillConfig,
        step_id: &str,
        workflow_id: &str,
        app: &tauri::AppHandle,
    ) -> Result<WorkflowStepResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        // Execute skill
        let skill_result = self.skills_manager.execute_skill(
            &skill_config.skill_path,
            skill_config.input.clone(),
            app,
            &State::from(&AppState::default()),
        ).await;

        let result = match skill_result {
            Ok(skill_result) => {
                if skill_result.success {
                    WorkflowStepResult {
                        step_id: step_id.to_string(),
                        success: true,
                        output: skill_result.output,
                        execution_time_ms: skill_result.execution_time_ms,
                    }
                } else {
                    return Err(AgentError {
                        agent_id: step_id.to_string(),
                        task_id: workflow_id.to_string(),
                        error: skill_result.error.unwrap_or_else(|| "Skill execution failed".to_string()),
                        execution_time_ms: skill_result.execution_time_ms,
                        created_at: chrono::Utc::now(),
                        failed_at: chrono::Utc::now(),
                    });
                }
            }
            Err(error) => {
                return Err(AgentError {
                    agent_id: step_id.to_string(),
                    task_id: workflow_id.to_string(),
                    error,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    created_at: chrono::Utc::now(),
                    failed_at: chrono::Utc::now(),
                });
            }
        };

        Ok(result)
    }

    /// Get orchestration statistics
    pub async fn get_statistics(&self) -> OrchestrationStatistics {
        let active_count = self.state.active_agents.read().await.len();
        let completed_count = self.state.completed_agents.read().await.len();
        let failed_count = self.state.failed_agents.read().await.len();

        OrchestrationStatistics {
            active_agents: active_count,
            completed_agents: completed_count,
            failed_agents: failed_count,
            total_agents: active_count + completed_count + failed_count,
            config: self.state.config.clone(),
        }
    }

    /// Cancel a running agent
    pub async fn cancel_agent(&self, agent_id: &str) -> Result<(), String> {
        let mut active_agents = self.state.active_agents.write().await;
        
        if let Some(mut agent_info) = active_agents.get_mut(agent_id) {
            agent_info.status = AgentStatus::Cancelled;
            agent_info.last_updated = chrono::Utc::now();
            
            Ok(())
        } else {
            Err(format!("Agent {} not found or already completed", agent_id))
        }
    }
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub parallel_execution: bool,
    pub timeout_seconds: Option<u64>,
}

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_type: WorkflowStepType,
    pub description: String,
    pub critical: bool,
    pub require_all_previous: bool,
    pub timeout_seconds: Option<u64>,
}

/// Workflow step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStepType {
    Agent(AgentConfig),
    Task(TaskConfig),
    Tool(ToolConfig),
    Skill(SkillConfig),
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: String,
    pub prompt: String,
    pub model: Option<String>,
    pub parameters: Option<HashMap<String, Value>>,
}

/// Task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub task_id: String,
    pub input: Value,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub tool_name: String,
    pub args: Value,
}

/// Skill configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub skill_path: String,
    pub input: Value,
}

/// Workflow step result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepResult {
    pub step_id: String,
    pub success: bool,
    pub output: Value,
    pub execution_time_ms: u64,
}

/// Workflow result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub workflow_id: String,
    pub success: bool,
    pub results: Vec<WorkflowStepResult>,
    pub errors: Vec<AgentError>,
    pub execution_time_ms: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Orchestration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStatistics {
    pub active_agents: usize,
    pub completed_agents: usize,
    pub failed_agents: usize,
    pub total_agents: usize,
    pub config: AgentOrchestrationConfig,
}

/// Tauri commands for agent orchestration
#[tauri::command]
pub async fn execute_workflow(
    workflow: WorkflowDefinition,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<WorkflowResult, String> {
    let orchestration_manager = state.get_orchestrator().await;
    orchestration_manager.execute_workflow(workflow, &app).await
}

#[tauri::command]
pub async fn get_orchestration_statistics(
    state: State<'_, AppState>,
) -> Result<OrchestrationStatistics, String> {
    let orchestration_manager = state.get_orchestrator().await;
    Ok(orchestration_manager.get_statistics().await)
}

#[tauri::command]
pub async fn cancel_agent(
    agent_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let orchestration_manager = state.get_orchestrator().await;
    orchestration_manager.cancel_agent(&agent_id).await
}
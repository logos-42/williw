use serde::{Deserialize, Serialize};
use serde_json;
use crate::state::AppState;
use tauri::{State, Emitter};
use crate::commands::tools::definitions::{ToolDefinition, ToolRegistry};
use crate::commands::tools::implementations::ToolExecutor;

/// Tool execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionContext {
    pub execution_id: String,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ToolExecutionContext {
    pub fn new(execution_id: String) -> Self {
        Self {
            execution_id,
            agent_id: None,
            session_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_agent(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Tool execution result with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub execution_id: String,
    pub tool_name: String,
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub context: ToolExecutionContext,
}

/// Enhanced tool executor with context and logging
pub struct ContextualToolExecutor {
    registry: ToolRegistry,
}

impl ContextualToolExecutor {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    pub fn with_registry(mut self, registry: ToolRegistry) -> Self {
        self.registry = registry;
        self
    }

    /// Execute a tool with context
    pub async fn execute_with_context(
        &self,
        tool_name: &str,
        args: serde_json::Value,
        context: ToolExecutionContext,
        app: &tauri::AppHandle,
        state: &State<'_, AppState>,
    ) -> Result<ToolExecutionResult, String> {
        let start_time = std::time::Instant::now();
        
        // Log tool execution start
        log::info!("[ToolExecutor] Executing tool: {} (execution_id: {})", tool_name, context.execution_id);

        // Check if tool exists in registry
        if self.registry.get_tool(tool_name).is_none() {
            return Err(format!("Tool not found in registry: {}", tool_name));
        }

        // Execute the tool
        let result = ToolExecutor::execute_tool(tool_name, args, app, state).await?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        let execution_result = ToolExecutionResult {
            execution_id: context.execution_id.clone(),
            tool_name: tool_name.to_string(),
            success: result.success,
            data: result.data,
            error: result.error,
            execution_time_ms: execution_time,
            context,
        };

        // Log tool execution result
        if execution_result.success {
            log::info!("[ToolExecutor] Tool {} completed successfully in {}ms", tool_name, execution_time);
        } else {
            log::warn!("[ToolExecutor] Tool {} failed after {}ms: {:?}", tool_name, execution_time, execution_result.error);
        }

        Ok(execution_result)
    }

    /// Execute multiple tools in sequence
    pub async fn execute_batch(
        &self,
        tools: Vec<(String, serde_json::Value)>,
        context: ToolExecutionContext,
        app: &tauri::AppHandle,
        state: &State<'_, AppState>,
    ) -> Result<Vec<ToolExecutionResult>, String> {
        let mut results = Vec::new();

        for (tool_name, args) in tools {
            let result = self.execute_with_context(&tool_name, args, context.clone(), app, state).await?;
            results.push(result);

            // If a tool fails and is critical, we might want to stop execution
            // For now, we continue and let the caller decide
        }

        Ok(results)
    }

    /// Execute tools with error handling and retry logic
    pub async fn execute_with_retry(
        &self,
        tool_name: &str,
        args: serde_json::Value,
        context: ToolExecutionContext,
        app: &tauri::AppHandle,
        state: &State<'_, AppState>,
        max_retries: u32,
        retry_delay_ms: u64,
    ) -> Result<ToolExecutionResult, String> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.execute_with_context(tool_name, args.clone(), context.clone(), app, state).await {
                Ok(result) => {
                    if result.success {
                        return Ok(result);
                    } else {
                        last_error = result.error;
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }

            if attempt < max_retries {
                // Wait before retrying
                tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms)).await;
            }
        }

        Err(format!("Tool {} failed after {} attempts. Last error: {:?}", 
            tool_name, max_retries + 1, last_error))
    }

    /// Get tool definition from registry
    pub fn get_tool_definition(&self, tool_name: &str) -> Option<&ToolDefinition> {
        self.registry.get_tool(tool_name)
    }

    /// Get all available tools
    pub fn get_all_tools(&self) -> Vec<&ToolDefinition> {
        self.registry.get_all_tools().iter().collect()
    }

    /// Get tools by type
    pub fn get_tools_by_type(&self, tool_type: &crate::commands::tools::definitions::ToolType) -> Vec<&ToolDefinition> {
        self.registry.get_tools_by_type(tool_type)
    }
}

/// Tool execution manager for managing multiple tool executors
pub struct ToolExecutionManager {
    executors: Vec<ContextualToolExecutor>,
}

impl ToolExecutionManager {
    pub fn new() -> Self {
        Self {
            executors: Vec::new(),
        }
    }

    pub fn add_executor(&mut self, executor: ContextualToolExecutor) {
        self.executors.push(executor);
    }

    /// Execute a tool using the first available executor
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
        context: ToolExecutionContext,
        app: &tauri::AppHandle,
        state: &State<'_, AppState>,
    ) -> Result<ToolExecutionResult, String> {
        if self.executors.is_empty() {
            return Err("No tool executors available".to_string());
        }

        // Use the first executor for now
        // In the future, we could implement load balancing or executor selection logic
        self.executors[0].execute_with_context(tool_name, args, context, app, state).await
    }

    /// Get the default executor
    pub fn get_default_executor(&self) -> Option<&ContextualToolExecutor> {
        self.executors.first()
    }

    /// Get the number of available executors
    pub fn executor_count(&self) -> usize {
        self.executors.len()
    }
}

/// Create a default tool execution manager with all tools registered
pub fn create_default_tool_execution_manager() -> ToolExecutionManager {
    let mut manager = ToolExecutionManager::new();
    let executor = ContextualToolExecutor::new();
    
    // Register all tools
    let mut registry = ToolRegistry::new();
    registry.register_tools(crate::commands::tools::definitions::get_tool_definitions());
    
    let executor = executor.with_registry(registry);
    manager.add_executor(executor);
    
    manager
}
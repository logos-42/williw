//! 分层提示词工具
//!
//! 提供分层提示词工程功能，避免上下文腐烂，支持循环执行直到任务完成

use super::{ToolExecutor, ToolMetadata, ToolCategory, ToolPriority, ToolStatus, ExecutionContext, ToolResult, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent::prompts::{LayeredPromptManager, LayeredPromptExecutor};

/// 分层提示词工具
pub struct LayeredPromptTool {
    metadata: ToolMetadata,
    manager: Arc<RwLock<LayeredPromptManager>>,
}

impl LayeredPromptTool {
    /// 创建新的分层提示词工具
    pub fn new() -> Self {
        let manager = Arc::new(RwLock::new(LayeredPromptManager::new().with_defaults()));
        
        Self {
            metadata: ToolMetadata {
                id: "layered_prompt".to_string(),
                name: "Layered Prompt Tool".to_string(),
                description: "Advanced layered prompt engineering system to prevent context decay and enable iterative task completion".to_string(),
                category: ToolCategory::Automation,
                priority: ToolPriority::High,
                status: ToolStatus::Available,
                version: "1.0.0".to_string(),
                author: "Alou Team".to_string(),
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
                dependencies: vec!["serde".to_string(), "tokio".to_string()],
                platforms: vec!["windows".to_string(), "macos".to_string(), "linux".to_string()],
                permissions: vec!["ai_interaction".to_string()],
            },
            manager,
        }
    }
}

#[async_trait]
impl ToolExecutor for LayeredPromptTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<ToolResult, ToolError> {
        let prompt_op: LayeredPromptOperation = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid arguments: {}", e)))?;

        match prompt_op {
            LayeredPromptOperation::ExecuteUntilComplete { task_description, max_iterations, iteration_delay_ms } => {
                self.execute_until_complete(task_description, max_iterations, iteration_delay_ms, context).await
            },
            LayeredPromptOperation::AddContextEntry { entry_type, content, importance } => {
                self.add_context_entry(entry_type, content, importance).await
            },
            LayeredPromptOperation::GetContextSummary => {
                self.get_context_summary().await
            },
            LayeredPromptOperation::BuildPrompt { task_description } => {
                self.build_prompt(task_description).await
            },
        }
    }

    async fn validate_args(&self, args: &serde_json::Value) -> Result<(), ToolError> {
        if let Ok(_op) = serde_json::from_value::<LayeredPromptOperation>(args.clone()) {
            Ok(())
        } else {
            Err(ToolError::InvalidArguments("Invalid layered prompt operation arguments".to_string()))
        }
    }

    fn help(&self) -> String {
        r#"Layered Prompt Tool - Advanced prompt engineering system

Available operations:
- execute_until_complete: Execute layered prompts iteratively until task completion
- add_context_entry: Add a context entry to the global context
- get_context_summary: Get a summary of the current context
- build_prompt: Build a layered prompt for a given task

Execute Until Complete options:
- task_description: Description of the task to execute
- max_iterations: Maximum number of iterations (default: 10)
- iteration_delay_ms: Delay between iterations in milliseconds (default: 1000)

Add Context Entry options:
- entry_type: Type of context entry (input, output, tool_call, tool_result, error, status_update, decision, learning_summary)
- content: Content of the context entry
- importance: Importance level (0-10) of the context entry

Example usage:
{
  "operation": "execute_until_complete",
  "task_description": "Analyze the codebase and identify potential optimizations",
  "max_iterations": 5,
  "iteration_delay_ms": 2000
}

{
  "operation": "add_context_entry",
  "entry_type": "decision",
  "content": "Decided to focus on performance optimizations",
  "importance": 8
}"#.to_string()
    }
}

impl LayeredPromptTool {
    /// 执行分层提示词直到任务完成
    async fn execute_until_complete(
        &self,
        task_description: String,
        max_iterations: Option<usize>,
        iteration_delay_ms: Option<u64>,
        _context: &ExecutionContext,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();

        // 设置当前任务ID
        let task_id = format!("task_{}_{}", _context.session_id, chrono::Utc::now().timestamp());
        self.manager.write().await.set_current_task_id(task_id.clone()).await;

        // 创建执行器
        let executor = LayeredPromptExecutor::new(self.manager.clone());

        // 执行直到完成或达到最大迭代次数
        let max_iter = max_iterations.unwrap_or(10);
        let delay_ms = iteration_delay_ms.unwrap_or(1000);

        // 使用一个简单的完成条件（在实际实现中，这应该是更复杂的条件）
        let completion_result = executor.execute_until_complete(
            &task_description,
            |_result| false, // 在实际实现中，这里应该是基于_result的复杂判断
            max_iter,
            delay_ms,
        ).await;

        let execution_time = start_time.elapsed().as_millis() as u64;

        match completion_result {
            Ok(result) => {
                Ok(ToolResult {
                    success: true,
                    data: serde_json::json!({
                        "task_id": task_id,
                        "task_description": task_description,
                        "result": result,
                        "iterations_completed": max_iter, // 在实际实现中，这应该是实际完成的迭代数
                        "max_iterations": max_iter,
                        "iteration_delay_ms": delay_ms,
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    output: Some(format!("Layered prompt execution completed for task: {}", task_description)),
                    warnings: vec![],
                    context: None,
                })
            },
            Err(e) => {
                let error_msg = e.clone();
                Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({
                        "task_id": task_id,
                        "task_description": task_description,
                        "error": e,
                        "max_iterations": max_iter,
                        "iteration_delay_ms": delay_ms,
                    }),
                    error: Some(error_msg),
                    execution_time_ms: execution_time,
                    output: Some(format!("Layered prompt execution failed: {}", e)),
                    warnings: vec![],
                    context: None,
                })
            }
        }
    }

    /// 添加上下文条目
    async fn add_context_entry(
        &self,
        entry_type: String,
        content: String,
        importance: u8,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        // 转换字符串类型的entry_type为ContextType
        let context_type = match entry_type.as_str() {
            "input" => crate::agent::prompts::ContextType::Input,
            "output" => crate::agent::prompts::ContextType::Output,
            "tool_call" => crate::agent::prompts::ContextType::ToolCall,
            "tool_result" => crate::agent::prompts::ContextType::ToolResult,
            "error" => crate::agent::prompts::ContextType::Error,
            "status_update" => crate::agent::prompts::ContextType::StatusUpdate,
            "decision" => crate::agent::prompts::ContextType::Decision,
            "learning_summary" => crate::agent::prompts::ContextType::LearningSummary,
            _ => crate::agent::prompts::ContextType::StatusUpdate, // 默认类型
        };
        
        let context_entry = crate::agent::prompts::ContextEntry {
            id: format!("context_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            entry_type: context_type,
            content,
            importance: std::cmp::min(importance, 10), // 限制重要性在0-10范围内
            timestamp: chrono::Utc::now().timestamp(),
            task_id: self.manager.read().await.get_current_task_id().await,
        };
        
        self.manager.write().await.update_global_context(context_entry).await;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "added": true,
                "entry_type": entry_type,
                "importance": std::cmp::min(importance, 10),
            }),
            error: None,
            execution_time_ms: execution_time,
            output: Some("Context entry added successfully".to_string()),
            warnings: vec![],
            context: None,
        })
    }

    /// 获取上下文摘要
    async fn get_context_summary(&self) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        let context = self.manager.read().await.get_global_context().await;
        let summary = context.get_context_summary();
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "context_summary": summary,
                "current_task_id": context.current_task_id,
                "context_entries_count": context.context_window.len(),
            }),
            error: None,
            execution_time_ms: execution_time,
            output: Some(format!("Context summary retrieved ({} entries)", context.context_window.len())),
            warnings: vec![],
            context: None,
        })
    }

    /// 构建分层提示词
    async fn build_prompt(&self, task_description: String) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        let prompt = self.manager.read().await.build_layered_prompt(&task_description).await;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "task_description": task_description,
                "built_prompt": prompt,
                "prompt_length": prompt.len(),
            }),
            error: None,
            execution_time_ms: execution_time,
            output: Some(format!("Layered prompt built ({} chars)", prompt.len())),
            warnings: vec![],
            context: None,
        })
    }
}

/// 分层提示词操作枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum LayeredPromptOperation {
    /// 执行分层提示词直到任务完成
    ExecuteUntilComplete {
        /// 任务描述
        task_description: String,
        /// 最大迭代次数
        max_iterations: Option<usize>,
        /// 迭代间延迟（毫秒）
        iteration_delay_ms: Option<u64>,
    },
    /// 添加上下文条目
    AddContextEntry {
        /// 条目类型
        entry_type: String,
        /// 内容
        content: String,
        /// 重要性 (0-10)
        importance: u8,
    },
    /// 获取上下文摘要
    GetContextSummary,
    /// 构建分层提示词
    BuildPrompt {
        /// 任务描述
        task_description: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_layered_prompt_tool_creation() {
        let tool = LayeredPromptTool::new();
        assert_eq!(tool.metadata().name, "Layered Prompt Tool");
    }

    #[tokio::test]
    async fn test_layered_prompt_validation() {
        let tool = LayeredPromptTool::new();

        // 有效的参数
        let valid_args = serde_json::json!({
            "operation": "get_context_summary"
        });
        assert!(tool.validate_args(&valid_args).await.is_ok());

        // 无效的参数
        let invalid_args = serde_json::json!({
            "invalid": "args"
        });
        assert!(tool.validate_args(&invalid_args).await.is_err());
    }
}
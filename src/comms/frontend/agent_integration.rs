//! 智能体前端集成模块
//!
//! 提供智能体功能与前端对话框的集成，支持ask模式和agent模式

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use crate::agent::prompts::{LayeredPromptManager, LayeredPromptExecutor, ContextEntry, ContextType};
use crate::agent::tools::{ToolRegistry, ToolResult, ExecutionContext};
use crate::agent::workflow::AsyncWorkflowExecutor;

/// 智能体模式枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMode {
    /// Ask模式：简单问答模式
    Ask,
    /// Agent模式：自主执行任务模式
    Agent,
}

/// 消息类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// 用户输入
    UserInput,
    /// AI助手回复
    AssistantReply,
    /// 系统消息
    SystemMessage,
    /// 工具执行结果
    ToolResult,
    /// 错误消息
    ErrorMessage,
}

/// 对话消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息ID
    pub id: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 发送者
    pub sender: String,
    /// 内容
    pub content: String,
    /// 时间戳
    pub timestamp: i64,
    /// 相关工具结果（如果有）
    pub tool_result: Option<ToolResult>,
    /// 执行上下文
    pub execution_context: Option<serde_json::Value>,
}

/// 智能体会话配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionConfig {
    /// 会话ID
    pub session_id: String,
    /// 智能体模式
    pub mode: AgentMode,
    /// 最大上下文长度
    pub max_context_length: usize,
    /// 是否启用工具
    pub enable_tools: bool,
    /// 是否启用工作流
    pub enable_workflows: bool,
    /// 最大迭代次数（仅适用于Agent模式）
    pub max_iterations: usize,
    /// 迭代延迟（毫秒）
    pub iteration_delay_ms: u64,
}

/// 智能体会话
#[derive(Clone)]
pub struct AgentSession {
    /// 会话配置
    pub config: AgentSessionConfig,
    /// 消息历史
    pub messages: Arc<RwLock<Vec<ChatMessage>>>,
    /// 提示词管理器
    pub prompt_manager: Arc<RwLock<LayeredPromptManager>>,
    /// 工具注册表
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
    /// 工作流执行器
    pub workflow_executor: Arc<AsyncWorkflowExecutor>,
    /// 发送消息通道
    tx: mpsc::UnboundedSender<ChatMessage>,
    /// 接收消息通道
    rx: Arc<RwLock<mpsc::UnboundedReceiver<ChatMessage>>>,
}

impl AgentSession {
    /// 创建新的智能体会话
    pub async fn new(config: AgentSessionConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let prompt_manager = Arc::new(RwLock::new(LayeredPromptManager::new().with_defaults()));
        let tool_registry = Arc::new(RwLock::new(crate::agent::tools::initialize_tools().await?));
        let workflow_executor = Arc::new(AsyncWorkflowExecutor::new().map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))?);

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            messages: Arc::new(RwLock::new(Vec::new())),
            prompt_manager,
            tool_registry,
            workflow_executor,
            tx,
            rx: Arc::new(RwLock::new(rx)),
        })
    }

    /// 处理用户输入
    pub async fn handle_user_input(&mut self, input: String) -> Result<Vec<ChatMessage>, String> {
        let mut responses = Vec::new();
        
        // 创建用户消息
        let user_message = ChatMessage {
            id: format!("msg_{}_{}", self.config.session_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            message_type: MessageType::UserInput,
            sender: "user".to_string(),
            content: input.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            tool_result: None,
            execution_context: None,
        };
        
        self.messages.write().await.push(user_message.clone());
        
        // 根据模式处理输入
        match self.config.mode {
            AgentMode::Ask => {
                // Ask模式：简单问答
                let response = self.process_ask_mode(input).await?;
                self.messages.write().await.push(response.clone());
                responses.push(response);
            }
            AgentMode::Agent => {
                // Agent模式：自主执行任务
                let agent_responses = self.process_agent_mode(input).await?;
                for response in agent_responses {
                    self.messages.write().await.push(response.clone());
                    responses.push(response);
                }
            }
        }
        
        Ok(responses)
    }

    /// 处理Ask模式
    async fn process_ask_mode(&self, input: String) -> Result<ChatMessage, String> {
        // 构建分层提示词
        let prompt = self.prompt_manager.read().await.build_layered_prompt(&input).await;
        
        // 模拟AI响应（在实际实现中，这里会调用AI模型API）
        let ai_response = self.simulate_ai_response(&prompt).await;
        
        Ok(ChatMessage {
            id: format!("resp_{}_{}", self.config.session_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            message_type: MessageType::AssistantReply,
            sender: "assistant".to_string(),
            content: ai_response,
            timestamp: chrono::Utc::now().timestamp(),
            tool_result: None,
            execution_context: None,
        })
    }

    /// 处理Agent模式
    async fn process_agent_mode(&self, input: String) -> Result<Vec<ChatMessage>, String> {
        let mut responses = Vec::new();
        
        // 如果启用了工具，检查是否需要使用工具
        if self.config.enable_tools {
            // 尝试识别工具调用
            if let Some(tool_call) = self.identify_tool_call(&input).await {
                // 执行工具
                let tool_result = self.execute_tool(tool_call).await?;
                
                // 添加工具结果消息
                let tool_result_msg = ChatMessage {
                    id: format!("tool_{}_{}", self.config.session_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                    message_type: MessageType::ToolResult,
                    sender: "system".to_string(),
                    content: format!("Tool executed: {}", tool_result.output.as_ref().unwrap_or(&"Unknown".to_string())),
                    timestamp: chrono::Utc::now().timestamp(),
                    tool_result: Some(tool_result.clone()),
                    execution_context: None,
                };
                
                responses.push(tool_result_msg);
                
                // 基于工具结果生成AI响应
                let ai_response = self.generate_response_from_tool_result(&tool_result).await;
                
                let ai_msg = ChatMessage {
                    id: format!("resp_{}_{}", self.config.session_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                    message_type: MessageType::AssistantReply,
                    sender: "assistant".to_string(),
                    content: ai_response,
                    timestamp: chrono::Utc::now().timestamp(),
                    tool_result: Some(tool_result),
                    execution_context: None,
                };
                
                responses.push(ai_msg);
            } else {
                // 没有识别到工具调用，使用普通响应
                let prompt = self.prompt_manager.read().await.build_layered_prompt(&input).await;
                let ai_response = self.simulate_ai_response(&prompt).await;
                
                let ai_msg = ChatMessage {
                    id: format!("resp_{}_{}", self.config.session_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                    message_type: MessageType::AssistantReply,
                    sender: "assistant".to_string(),
                    content: ai_response,
                    timestamp: chrono::Utc::now().timestamp(),
                    tool_result: None,
                    execution_context: None,
                };
                
                responses.push(ai_msg);
            }
        } else {
            // 工具未启用，使用普通响应
            let prompt = self.prompt_manager.read().await.build_layered_prompt(&input).await;
            let ai_response = self.simulate_ai_response(&prompt).await;
            
            let ai_msg = ChatMessage {
                id: format!("resp_{}_{}", self.config.session_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                message_type: MessageType::AssistantReply,
                sender: "assistant".to_string(),
                content: ai_response,
                timestamp: chrono::Utc::now().timestamp(),
                tool_result: None,
                execution_context: None,
            };
            
            responses.push(ai_msg);
        }
        
        Ok(responses)
    }

    /// 识别工具调用
    async fn identify_tool_call(&self, input: &str) -> Option<serde_json::Value> {
        // 简单的工具识别逻辑（在实际实现中，这里会有更复杂的NLP处理）
        if input.to_lowercase().contains("file") && input.to_lowercase().contains("read") {
            // 识别为文件读取操作
            return Some(serde_json::json!({
                "tool_name": "filesystem",
                "operation": "read_file",
                "params": {
                    "path": input.split_whitespace().find(|w| w.ends_with(".txt") || w.ends_with(".rs") || w.ends_with(".json")).unwrap_or("")
                }
            }));
        } else if input.to_lowercase().contains("search") || input.to_lowercase().contains("find") {
            // 识别为搜索操作
            return Some(serde_json::json!({
                "tool_name": "search",
                "operation": "search_files",
                "params": {
                    "query": input
                }
            }));
        }
        
        None
    }

    /// 执行工具
    async fn execute_tool(&self, tool_call: serde_json::Value) -> Result<ToolResult, String> {
        let tool_name = tool_call.get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing tool name")?;
        
        let params = tool_call.get("params")
            .unwrap_or(&serde_json::Value::Object(serde_json::Map::new())).clone();
        
        let mut registry = self.tool_registry.write().await;
        let tool = registry.get_tool(tool_name).await
            .ok_or(format!("Tool '{}' not found", tool_name))?;
        
        let context = ExecutionContext {
            session_id: self.config.session_id.clone(),
            user_id: None,
            working_directory: None,
            environment: std::collections::HashMap::new(),
            timeout_seconds: Some(30),
            permissions: vec!["read".to_string(), "write".to_string()],
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        let result = tool.execute(params, &context).await
            .map_err(|e| e.to_string())?;
        
        Ok(result)
    }

    /// 基于工具结果生成响应
    async fn generate_response_from_tool_result(&self, tool_result: &ToolResult) -> String {
        if tool_result.success {
            format!(
                "工具执行成功！\n结果: {}\n执行时间: {}ms",
                tool_result.output.as_ref().unwrap_or(&"无输出".to_string()),
                tool_result.execution_time_ms
            )
        } else {
            format!(
                "工具执行失败！\n错误: {}\n警告: {:?}",
                tool_result.error.as_ref().unwrap_or(&"未知错误".to_string()),
                tool_result.warnings
            )
        }
    }

    /// 模拟AI响应（在实际实现中，这里会调用AI模型API）
    async fn simulate_ai_response(&self, prompt: &str) -> String {
        // 在实际实现中，这里会调用OpenAI API或其他LLM服务
        // 现在我们返回一个模拟响应
        format!("AI助手回复: 我收到了您的消息: \"{}\"。这是基于分层提示词系统的响应。", 
                prompt.lines().last().unwrap_or("消息"))
    }

    /// 获取最近的消息
    pub async fn get_recent_messages(&self, count: usize) -> Vec<ChatMessage> {
        let messages = self.messages.read().await;
        let start = if messages.len() > count {
            messages.len() - count
        } else {
            0
        };

        messages[start..].to_vec()
    }

    /// 清空会话历史
    pub async fn clear_history(&self) {
        self.messages.write().await.clear();
    }

    /// 更新全局上下文
    pub async fn update_context(&self, content: String, importance: u8) {
        let context_entry = ContextEntry {
            id: format!("ctx_{}_{}", self.config.session_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            entry_type: ContextType::Input,
            content,
            importance,
            timestamp: chrono::Utc::now().timestamp(),
            task_id: Some(self.config.session_id.clone()),
        };
        
        self.prompt_manager.write().await.update_global_context(context_entry).await;
    }
}

/// 智能体前端管理器
pub struct AgentFrontendManager {
    /// 活跃的会话
    sessions: Arc<RwLock<std::collections::HashMap<String, AgentSession>>>,
    /// 默认配置
    default_config: AgentSessionConfig,
}

impl AgentFrontendManager {
    /// 创建新的智能体前端管理器
    pub fn new(default_config: AgentSessionConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            default_config,
        }
    }

    /// 创建新会话
    pub async fn create_session(&self, session_id: String, mode: AgentMode) -> Result<(), String> {
        let config = AgentSessionConfig {
            session_id: session_id.clone(),
            mode,
            ..self.default_config.clone()
        };
        
        let session = AgentSession::new(config)
            .await
            .map_err(|e| e.to_string())?;
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);
        
        Ok(())
    }

    /// 获取会话的Arc引用
    pub async fn get_session_arc(&self, session_id: &str) -> Option<Arc<RwLock<AgentSession>>> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            Some(Arc::new(RwLock::new(session.clone())))
        } else {
            None
        }
    }

    /// 获取会话（返回引用而非克隆）
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<RwLock<AgentSession>>> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            Some(Arc::new(RwLock::new(session.clone())))
        } else {
            None
        }
    }

    /// 处理会话消息
    pub async fn handle_message(&self, session_id: &str, message: String) -> Result<Vec<ChatMessage>, String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id).ok_or("Session not found")?;
        
        session.handle_user_input(message).await
    }

    /// 删除会话
    pub async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).ok_or("Session not found".to_string())?;
        Ok(())
    }

    /// 获取所有活跃会话ID
    pub async fn get_active_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_session_creation() {
        let config = AgentSessionConfig {
            session_id: "test_session".to_string(),
            mode: AgentMode::Ask,
            max_context_length: 1000,
            enable_tools: true,
            enable_workflows: false,
            max_iterations: 5,
            iteration_delay_ms: 1000,
        };

        let session = AgentSession::new(config).await;
        assert!(session.is_ok());
    }

    #[tokio::test]
    async fn test_agent_frontend_manager() {
        let default_config = AgentSessionConfig {
            session_id: "".to_string(),
            mode: AgentMode::Ask,
            max_context_length: 1000,
            enable_tools: true,
            enable_workflows: false,
            max_iterations: 5,
            iteration_delay_ms: 1000,
        };

        let manager = AgentFrontendManager::new(default_config);
        
        // 创建会话
        let result = manager.create_session("test_session".to_string(), AgentMode::Agent).await;
        assert!(result.is_ok());
        
        // 检查会话是否存在
        let session = manager.get_session("test_session").await;
        assert!(session.is_some());
        
        // 处理消息
        let responses = manager.handle_message("test_session", "Hello, agent!".to_string()).await;
        assert!(responses.is_ok());
        assert!(!responses.unwrap().is_empty());
    }
}
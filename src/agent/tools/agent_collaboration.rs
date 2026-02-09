//! Agent 协作工具模块
//!
//! 实现多智能体协作和通信功能

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent::tools::{ToolExecutor, ToolMetadata, ToolResult, ToolError, ExecutionContext, ToolCategory, ToolStatus, ToolPriority};

/// 协作会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed(String),
}

/// 参与者角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantRole {
    Leader,
    Contributor,
    Observer,
    Coordinator,
}

/// 参与者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub id: String,
    pub role: ParticipantRole,
    pub capabilities: Vec<String>,
    pub status: String,
    pub last_seen: i64,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
    Notification,
    Broadcast,
    Acknowledgment,
}

/// Pub/Sub 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubChatMessage {
    pub id: String,
    pub session_id: String,
    pub sender_id: String,
    pub recipient_id: Option<String>,
    pub message_type: MessageType,
    pub content: String,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
}

/// 协作会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    pub id: String,
    pub name: String,
    pub participants: Vec<ParticipantInfo>,
    pub status: SessionStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: HashMap<String, String>,
}

/// Agent 协作工具
pub struct AgentCollaborationTool {
    metadata: ToolMetadata,
    ipfs_api_url: String,
    sessions: Arc<RwLock<HashMap<String, CollaborationSession>>>,
    messages: Arc<RwLock<Vec<PubSubChatMessage>>>,
}

impl AgentCollaborationTool {
    pub fn new(ipfs_api_url: String) -> Self {
        Self {
            metadata: ToolMetadata {
                id: "agent_collaboration".to_string(),
                name: "Agent Collaboration Tool".to_string(),
                description: "支持多智能体协作和通信的工具".to_string(),
                category: ToolCategory::Communication,
                priority: ToolPriority::Medium,
                status: ToolStatus::Available,
                version: "1.0.0".to_string(),
                author: "Alou Team".to_string(),
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
                dependencies: vec!["ipfs".to_string()],
                platforms: vec!["windows".to_string(), "macos".to_string(), "linux".to_string()],
                permissions: vec!["network".to_string(), "read".to_string(), "write".to_string()],
            },
            ipfs_api_url,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建新的协作会话
    pub async fn create_session(&self, name: String, participant: ParticipantInfo) -> Result<String, ToolError> {
        let session_id = format!("session_{}", uuid::Uuid::new_v4());
        let session = CollaborationSession {
            id: session_id.clone(),
            name,
            participants: vec![participant],
            status: SessionStatus::Active,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    /// 加入协作会话
    pub async fn join_session(&self, session_id: String, participant: ParticipantInfo) -> Result<(), ToolError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            if matches!(session.status, SessionStatus::Active) {
                session.participants.push(participant);
                session.updated_at = chrono::Utc::now().timestamp();
                Ok(())
            } else {
                Err(ToolError::ExecutionFailed("Session is not active".to_string()))
            }
        } else {
            Err(ToolError::ExecutionFailed("Session not found".to_string()))
        }
    }

    /// 发送消息到会话
    pub async fn send_message(&self, message: PubSubChatMessage) -> Result<(), ToolError> {
        let mut messages = self.messages.write().await;
        messages.push(message);
        Ok(())
    }

    /// 获取会话消息
    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<PubSubChatMessage>, ToolError> {
        let messages = self.messages.read().await;
        let session_messages: Vec<PubSubChatMessage> = messages
            .iter()
            .filter(|msg| msg.session_id == session_id)
            .cloned()
            .collect();
        
        Ok(session_messages)
    }

    /// 获取会话信息
    pub async fn get_session(&self, session_id: &str) -> Result<CollaborationSession, ToolError> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id)
            .cloned()
            .ok_or_else(|| ToolError::ExecutionFailed("Session not found".to_string()))
    }
}

#[async_trait]
impl ToolExecutor for AgentCollaborationTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, args: serde_json::Value, context: &ExecutionContext) -> Result<ToolResult, ToolError> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' field".to_string()))?;

        match action {
            "create_session" => {
                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'name' field".to_string()))?
                    .to_string();

                let participant = ParticipantInfo {
                    id: context.session_id.clone(),
                    role: ParticipantRole::Leader,
                    capabilities: vec!["basic".to_string()],
                    status: "online".to_string(),
                    last_seen: chrono::Utc::now().timestamp(),
                };

                let session_id = self.create_session(name, participant).await?;

                Ok(ToolResult {
                    success: true,
                    output: Some(format!("{{\"session_id\": \"{}\", \"message\": \"Session created successfully\"}}", session_id)),
                    execution_time_ms: 0,
                    error: None,
                    context: None,
                    data: serde_json::Value::Null,
                    warnings: vec![],
                })
            }
            "join_session" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'session_id' field".to_string()))?
                    .to_string();

                let role_str = args.get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("contributor");

                let role = match role_str {
                    "leader" => ParticipantRole::Leader,
                    "contributor" => ParticipantRole::Contributor,
                    "observer" => ParticipantRole::Observer,
                    "coordinator" => ParticipantRole::Coordinator,
                    _ => ParticipantRole::Contributor,
                };

                let participant = ParticipantInfo {
                    id: context.session_id.clone(),
                    role,
                    capabilities: vec!["basic".to_string()],
                    status: "online".to_string(),
                    last_seen: chrono::Utc::now().timestamp(),
                };

                self.join_session(session_id, participant).await?;

                Ok(ToolResult {
                    success: true,
                    output: Some("{\"message\": \"Joined session successfully\"}".to_string()),
                    execution_time_ms: 0,
                    error: None,
                    context: None,
                    data: serde_json::Value::Null,
                    warnings: vec![],
                })
            }
            "send_message" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'session_id' field".to_string()))?
                    .to_string();

                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' field".to_string()))?
                    .to_string();

                let recipient_id = args.get("recipient_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let message_type_str = args.get("message_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("request");

                let message_type = match message_type_str {
                    "request" => MessageType::Request,
                    "response" => MessageType::Response,
                    "notification" => MessageType::Notification,
                    "broadcast" => MessageType::Broadcast,
                    "ack" => MessageType::Acknowledgment,
                    _ => MessageType::Request,
                };

                let message = PubSubChatMessage {
                    id: format!("msg_{}", uuid::Uuid::new_v4()),
                    session_id,
                    sender_id: context.session_id.clone(),
                    recipient_id,
                    message_type,
                    content,
                    timestamp: chrono::Utc::now().timestamp(),
                    metadata: HashMap::new(),
                };

                self.send_message(message).await?;

                Ok(ToolResult {
                    success: true,
                    output: Some("{\"message\": \"Message sent successfully\"}".to_string()),
                    execution_time_ms: 0,
                    error: None,
                    context: None,
                    data: serde_json::Value::Null,
                    warnings: vec![],
                })
            }
            "get_messages" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'session_id' field".to_string()))?
                    .to_string();

                let messages = self.get_session_messages(&session_id).await?;

                Ok(ToolResult {
                    success: true,
                    output: Some(format!("{{\"messages\": [{}], \"count\": {}}}",
                        messages.iter().map(|m| format!("\"{}\"", m.content)).collect::<Vec<_>>().join(", "),
                        messages.len())),
                    execution_time_ms: 0,
                    error: None,
                    context: None,
                    data: serde_json::Value::Null,
                    warnings: vec![],
                })
            }
            "get_session" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'session_id' field".to_string()))?
                    .to_string();

                let session = self.get_session(&session_id).await?;

                Ok(ToolResult {
                    success: true,
                    output: Some(format!("{{\"session\": {{\"id\": \"{}\", \"name\": \"{}\"}}}}", session.id, session.name)),
                    execution_time_ms: 0,
                    error: None,
                    context: None,
                    data: serde_json::Value::Null,
                    warnings: vec![],
                })
            }
            _ => Err(ToolError::InvalidArguments(format!("Unknown action: {}", action))),
        }
    }

    async fn validate_args(&self, args: &serde_json::Value) -> Result<(), ToolError> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' field".to_string()))?;

        match action {
            "create_session" => {
                if args.get("name").is_none() {
                    return Err(ToolError::InvalidArguments("Missing 'name' field for create_session".to_string()));
                }
            }
            "join_session" | "get_session" | "get_messages" => {
                if args.get("session_id").is_none() {
                    return Err(ToolError::InvalidArguments(format!("Missing 'session_id' field for {}", action)));
                }
            }
            "send_message" => {
                if args.get("session_id").is_none() || args.get("content").is_none() {
                    return Err(ToolError::InvalidArguments("Missing 'session_id' or 'content' field for send_message".to_string()));
                }
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!("Unknown action: {}", action)));
            }
        }

        Ok(())
    }

    fn help(&self) -> String {
        "Agent Collaboration Tool: Facilitates multi-agent collaboration and communication.\n\nActions:\n- create_session: Create a new collaboration session\n- join_session: Join an existing session\n- send_message: Send a message to a session\n- get_messages: Retrieve messages from a session\n- get_session: Get session information".to_string()
    }
}
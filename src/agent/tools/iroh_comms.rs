//! Iroh 通讯工具
//!
//! 提供基于 iroh 的 P2P 通讯功能，包括节点连接、消息发送、文件传输等

use super::{ToolExecutor, ToolMetadata, ToolCategory, ToolPriority, ToolStatus, ExecutionContext, ToolResult, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

// 导入项目中的 iroh 通讯模块
use crate::comms::transport::iroh::{IrohConnectionManager, IrohConnectionConfig, WrappedMessage, FILE_TRANSFER_MESSAGE_TYPE, GOSSIP_MESSAGE_TYPE, CONTROL_MESSAGE_TYPE};

/// Iroh 通讯工具
pub struct IrohCommsTool {
    metadata: ToolMetadata,
    manager: Arc<Mutex<Option<Arc<IrohConnectionManager>>>>,
}

impl IrohCommsTool {
    /// 创建新的 Iroh 通讯工具
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = IrohConnectionConfig::default();
        let manager = IrohConnectionManager::new(config).await?;
        
        Ok(Self {
            metadata: ToolMetadata {
                id: "iroh_comms".to_string(),
                name: "Iroh Communications Tool".to_string(),
                description: "P2P communications using iroh - connect to peers, send/receive messages, transfer files".to_string(),
                category: ToolCategory::Communication,
                priority: ToolPriority::High,
                status: ToolStatus::Available,
                version: "1.0.0".to_string(),
                author: "Alou Team".to_string(),
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
                dependencies: vec!["iroh".to_string()],
                platforms: vec!["windows".to_string(), "macos".to_string(), "linux".to_string()],
                permissions: vec!["network".to_string(), "p2p".to_string()],
            },
            manager: Arc::new(Mutex::new(Some(Arc::new(manager)))),
        })
    }
}

#[async_trait]
impl ToolExecutor for IrohCommsTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _context: &ExecutionContext,
    ) -> Result<ToolResult, ToolError> {
        let comms_op: IrohCommsOperation = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid arguments: {}", e)))?;

        // 获取连接管理器
        let manager_opt = self.manager.lock().await;
        let manager = match manager_opt.as_ref() {
            Some(mgr) => mgr.clone(),
            None => return Err(ToolError::ExecutionFailed("Iroh connection manager not initialized".to_string())),
        };
        drop(manager_opt); // 释放锁

        match comms_op {
            IrohCommsOperation::Connect { peer_id } => {
                self.connect_to_peer(manager, peer_id).await
            },
            IrohCommsOperation::SendMessage { peer_id, message_type, message } => {
                self.send_message(manager, peer_id, message_type, message).await
            },
            IrohCommsOperation::BroadcastMessage { message_type, message } => {
                self.broadcast_message(manager, message_type, message).await
            },
            IrohCommsOperation::ReceiveMessage { timeout_ms } => {
                self.receive_message(manager, timeout_ms).await
            },
            IrohCommsOperation::GetNodeId => {
                self.get_node_id(manager).await
            },
            IrohCommsOperation::GetConnectionStats => {
                self.get_connection_stats(manager).await
            },
            IrohCommsOperation::Disconnect { peer_id } => {
                self.disconnect(manager, peer_id).await
            },
            IrohCommsOperation::ListConnections => {
                self.list_connections(manager).await
            },
        }
    }

    async fn validate_args(&self, args: &serde_json::Value) -> Result<(), ToolError> {
        if let Ok(_op) = serde_json::from_value::<IrohCommsOperation>(args.clone()) {
            Ok(())
        } else {
            Err(ToolError::InvalidArguments("Invalid iroh comms operation arguments".to_string()))
        }
    }

    fn help(&self) -> String {
        r#"Iroh Communications Tool - P2P networking with iroh

Available operations:
- connect: Connect to a peer
- send_message: Send a message to a specific peer
- broadcast_message: Broadcast a message to all connected peers
- receive_message: Receive a message from any peer
- get_node_id: Get the local node ID
- get_connection_stats: Get connection statistics
- disconnect: Disconnect from a peer
- list_connections: List all active connections

Connect options:
- peer_id: Peer's z-base-32 encoded node ID

Send message options:
- peer_id: Target peer's node ID
- message_type: Type of message (file_transfer, gossip, control)
- message: Message content as string

Broadcast message options:
- message_type: Type of message (file_transfer, gossip, control)
- message: Message content as string

Receive message options:
- timeout_ms: Timeout in milliseconds (default: 5000)

Example usage:
{
  "operation": "connect",
  "peer_id": "2aaaaabbbbbbccccccdddddd"
}

{
  "operation": "send_message",
  "peer_id": "2aaaaabbbbbbccccccdddddd",
  "message_type": "gossip",
  "message": "Hello, peer!"
}

{
  "operation": "broadcast_message",
  "message_type": "control",
  "message": "Broadcast message to all peers"
}"#.to_string()
    }
}

impl IrohCommsTool {
    /// 连接到远程节点
    async fn connect_to_peer(
        &self,
        manager: Arc<IrohConnectionManager>,
        peer_id: String,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        match manager.connect_to_peer(&peer_id).await {
            Ok(_) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                Ok(ToolResult {
                    success: true,
                    data: serde_json::json!({
                        "peer_id": peer_id,
                        "connected": true,
                        "node_id": manager.node_id()
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    output: Some(format!("Successfully connected to peer: {}", peer_id)),
                    warnings: vec![],
                    context: None,
                })
            },
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({
                        "peer_id": peer_id,
                        "connected": false
                    }),
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time,
                    output: Some(format!("Failed to connect to peer {}: {}", peer_id, e)),
                    warnings: vec![],
                    context: None,
                })
            }
        }
    }

    /// 发送消息到指定节点
    async fn send_message(
        &self,
        manager: Arc<IrohConnectionManager>,
        peer_id: String,
        message_type: String,
        message: String,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        // 创建包装消息
        let wrapped_msg = WrappedMessage::new(
            message_type.clone(),
            manager.node_id(),
            message.as_bytes().to_vec(),
        );
        
        let msg_bytes = match wrapped_msg.serialize() {
            Ok(bytes) => bytes,
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                return Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({}),
                    error: Some(format!("Failed to serialize message: {}", e)),
                    execution_time_ms: execution_time,
                    output: Some(format!("Failed to serialize message: {}", e)),
                    warnings: vec![],
                    context: None,
                });
            }
        };

        // 先保存消息长度，因为send_message会消耗msg_bytes
        let msg_len = msg_bytes.len();

        match manager.send_message(&peer_id, msg_bytes).await {
            Ok(_) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                Ok(ToolResult {
                    success: true,
                    data: serde_json::json!({
                        "peer_id": peer_id,
                        "message_type": message_type,
                        "message_sent": true,
                        "bytes_sent": msg_len
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    output: Some(format!("Successfully sent {} message to {}", message_type, peer_id)),
                    warnings: vec![],
                    context: None,
                })
            },
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({
                        "peer_id": peer_id,
                        "message_type": message_type,
                        "message_sent": false
                    }),
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time,
                    output: Some(format!("Failed to send message to {}: {}", peer_id, e)),
                    warnings: vec![],
                    context: None,
                })
            }
        }
    }

    /// 广播消息到所有连接的节点
    async fn broadcast_message(
        &self,
        manager: Arc<IrohConnectionManager>,
        message_type: String,
        message: String,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        // 创建包装消息
        let wrapped_msg = WrappedMessage::new(
            message_type.clone(),
            manager.node_id(),
            message.as_bytes().to_vec(),
        );
        
        let msg_bytes = match wrapped_msg.serialize() {
            Ok(bytes) => bytes,
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                return Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({}),
                    error: Some(format!("Failed to serialize message: {}", e)),
                    execution_time_ms: execution_time,
                    output: Some(format!("Failed to serialize message: {}", e)),
                    warnings: vec![],
                    context: None,
                });
            }
        };

        // 先保存消息长度，因为broadcast_message会消耗msg_bytes
        let msg_len = msg_bytes.len();

        match manager.broadcast_message(msg_bytes).await {
            Ok(sent_count) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                Ok(ToolResult {
                    success: true,
                    data: serde_json::json!({
                        "message_type": message_type,
                        "message_sent": true,
                        "bytes_sent": msg_len,
                        "peers_broadcasted": sent_count
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    output: Some(format!("Successfully broadcasted {} message to {} peers", message_type, sent_count)),
                    warnings: vec![],
                    context: None,
                })
            },
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time,
                    output: Some(format!("Failed to broadcast message: {}", e)),
                    warnings: vec![],
                    context: None,
                })
            }
        }
    }

    /// 接收消息
    async fn receive_message(
        &self,
        manager: Arc<IrohConnectionManager>,
        timeout_ms: Option<u64>,
    ) -> Result<ToolResult, ToolError> {
        let timeout = timeout_ms.unwrap_or(5000);
        let start_time = std::time::Instant::now();
        
        // 在指定时间内尝试接收消息
        loop {
            match manager.receive_message().await {
                Ok(Some((peer_id, data))) => {
                    // 尝试反序列化为包装消息
                    let wrapped_msg = match WrappedMessage::deserialize(&data) {
                        Ok(msg) => msg,
                        Err(_) => {
                            // 如果反序列化失败，直接返回原始数据
                            let execution_time = start_time.elapsed().as_millis() as u64;
                            
                            return Ok(ToolResult {
                                success: true,
                                data: serde_json::json!({
                                    "peer_id": peer_id,
                                    "message_type": "raw",
                                    "payload": String::from_utf8_lossy(&data).to_string(),
                                    "bytes_received": data.len()
                                }),
                                error: None,
                                execution_time_ms: execution_time,
                                output: Some(format!("Received raw message from {}: {} bytes", peer_id, data.len())),
                                warnings: vec![],
                                context: None,
                            });
                        }
                    };
                    
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    
                    return Ok(ToolResult {
                        success: true,
                        data: serde_json::json!({
                            "peer_id": peer_id,
                            "message_type": wrapped_msg.message_type,
                            "sender_id": wrapped_msg.sender_id,
                            "timestamp": wrapped_msg.timestamp,
                            "payload": String::from_utf8_lossy(&wrapped_msg.payload).to_string(),
                            "bytes_received": data.len()
                        }),
                        error: None,
                        execution_time_ms: execution_time,
                        output: Some(format!("Received {} message from {}: {}", wrapped_msg.message_type, peer_id, String::from_utf8_lossy(&wrapped_msg.payload))),
                        warnings: vec![],
                        context: None,
                    });
                },
                Ok(None) => {
                    // 没有收到消息，检查是否超时
                    if start_time.elapsed().as_millis() as u64 >= timeout {
                        let execution_time = start_time.elapsed().as_millis() as u64;
                        
                        return Ok(ToolResult {
                            success: true, // 没有错误，只是没有收到消息
                            data: serde_json::json!({
                                "received": false,
                                "timeout": true
                            }),
                            error: None,
                            execution_time_ms: execution_time,
                            output: Some("Timeout waiting for message".to_string()),
                            warnings: vec![],
                            context: None,
                        });
                    }
                    
                    // 短暂休眠后重试
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                },
                Err(e) => {
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    
                    return Ok(ToolResult {
                        success: false,
                        data: serde_json::json!({}),
                        error: Some(e.to_string()),
                        execution_time_ms: execution_time,
                        output: Some(format!("Error receiving message: {}", e)),
                        warnings: vec![],
                        context: None,
                    });
                }
            }
        }
    }

    /// 获取节点ID
    async fn get_node_id(
        &self,
        manager: Arc<IrohConnectionManager>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        let node_id = manager.node_id();
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "node_id": node_id
            }),
            error: None,
            execution_time_ms: execution_time,
            output: Some(format!("Local node ID: {}", node_id)),
            warnings: vec![],
            context: None,
        })
    }

    /// 获取连接统计
    async fn get_connection_stats(
        &self,
        manager: Arc<IrohConnectionManager>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        let stats = manager.get_connection_stats().await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: serde_json::json!(stats),
            error: None,
            execution_time_ms: execution_time,
            output: Some(format!("Active connections: {}, Max: {}", stats.active_connections, stats.max_connections)),
            warnings: vec![],
            context: None,
        })
    }

    /// 断开连接
    async fn disconnect(
        &self,
        manager: Arc<IrohConnectionManager>,
        peer_id: String,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        match manager.disconnect(&peer_id).await {
            Ok(_) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                Ok(ToolResult {
                    success: true,
                    data: serde_json::json!({
                        "peer_id": peer_id,
                        "disconnected": true
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    output: Some(format!("Successfully disconnected from {}", peer_id)),
                    warnings: vec![],
                    context: None,
                })
            },
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                Ok(ToolResult {
                    success: false,
                    data: serde_json::json!({
                        "peer_id": peer_id,
                        "disconnected": false
                    }),
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time,
                    output: Some(format!("Failed to disconnect from {}: {}", peer_id, e)),
                    warnings: vec![],
                    context: None,
                })
            }
        }
    }

    /// 列出所有连接
    async fn list_connections(
        &self,
        manager: Arc<IrohConnectionManager>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        let stats = manager.get_connection_stats().await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // 获取连接列表（目前我们只存储了连接数量，但可以扩展获取详细信息）
        let connections_list = (0..stats.active_connections)
            .map(|i| format!("connection_{}", i))
            .collect::<Vec<String>>();
        
        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "active_connections": stats.active_connections,
                "max_connections": stats.max_connections,
                "connections_list": connections_list,
                "node_id": stats.node_id
            }),
            error: None,
            execution_time_ms: execution_time,
            output: Some(format!("Active connections: {}", stats.active_connections)),
            warnings: vec![],
            context: None,
        })
    }
}

/// Iroh 通讯操作枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum IrohCommsOperation {
    /// 连接到远程节点
    Connect {
        /// 对等节点的ID（z-base-32编码）
        peer_id: String,
    },
    /// 发送消息到指定节点
    SendMessage {
        /// 目标对等节点ID
        peer_id: String,
        /// 消息类型
        message_type: String,
        /// 消息内容
        message: String,
    },
    /// 广播消息到所有连接的节点
    BroadcastMessage {
        /// 消息类型
        message_type: String,
        /// 消息内容
        message: String,
    },
    /// 接收消息
    ReceiveMessage {
        /// 超时时间（毫秒）
        timeout_ms: Option<u64>,
    },
    /// 获取节点ID
    GetNodeId,
    /// 获取连接统计信息
    GetConnectionStats,
    /// 断开与指定节点的连接
    Disconnect {
        /// 对等节点ID
        peer_id: String,
    },
    /// 列出所有活动连接
    ListConnections,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_iroh_comms_tool_creation() {
        let result = IrohCommsTool::new().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_iroh_comms_validation() {
        let tool = IrohCommsTool::new().await.unwrap();

        // 有效的参数
        let valid_args = serde_json::json!({
            "operation": "get_node_id"
        });
        assert!(tool.validate_args(&valid_args).await.is_ok());

        // 无效的参数
        let invalid_args = serde_json::json!({
            "invalid": "args"
        });
        assert!(tool.validate_args(&invalid_args).await.is_err());
    }
}
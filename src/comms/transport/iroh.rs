/**
 * Iroh传输层实现
 * 统一的iroh集成，包含Gossip消息和P2P文件传输
 */

use anyhow::{Result, anyhow};
use iroh::{Endpoint, endpoint::Connection, EndpointAddr, PublicKey};
use iroh::endpoint_info::EndpointIdExt;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use tokio::io::AsyncReadExt;

// 兼容原有的Gossip功能
use crate::consensus::SignedGossip;

/// Iroh连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConnectionConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 节点ID
    pub node_id: Option<String>,
    /// bootstrap节点列表
    pub bootstrap_nodes: Vec<String>,
    /// 是否启用中继
    pub enable_relay: bool,
    /// 最大并发连接数
    pub max_connections: usize,
}

impl Default for IrohConnectionConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:0".to_string(),
            node_id: None,
            bootstrap_nodes: vec![],
            enable_relay: true,
            max_connections: 50,
        }
    }
}

/// Iroh连接管理器
#[derive(Clone)]
pub struct IrohConnectionManager {
    endpoint: Endpoint,
    config: IrohConnectionConfig,
    connections: Arc<Mutex<HashMap<String, Connection>>>,
    message_tx: mpsc::Sender<(String, Vec<u8>)>,
    node_id: String,
}

impl IrohConnectionManager {
    /// 创建新的连接管理器
    pub async fn new(config: IrohConnectionConfig) -> Result<Self> {
        info!("🔗 初始化 iroh 连接管理器");
        
        // 创建iroh端点 - 使用正确的API
        let endpoint = Endpoint::builder()
            .bind_addr_v4("0.0.0.0:0".parse().unwrap())
            .alpns(vec![b"williw-p2p".to_vec()])  // 设置ALPN协议
            .bind()
            .await?;
        
        // 创建数据目录 - 使用统一目录
        let data_dir = std::path::PathBuf::from("./williw_p2p_data");
        std::fs::create_dir_all(&data_dir)?;
        
        let node_id = endpoint.id().to_z32();
        info!("✅ iroh 端点已创建，节点ID: {}", node_id);
        
        let (message_tx, _message_rx) = mpsc::channel::<(String, Vec<u8>)>(1000);
        let connections = Arc::new(Mutex::new(HashMap::new()));
        
        Ok(Self {
            endpoint,
            config,
            connections,
            message_tx,
            node_id,
        })
    }
    
    /// 连接到远程节点
    pub async fn connect_to_peer(&self, peer_addr: &str) -> Result<()> {
        info!("🔗 连接到远程节点: {}", peer_addr);
        
        // 实现真实的iroh连接 - 使用正确的API
        // 尝试从z-base-32格式解析PublicKey
        let public_key = match PublicKey::from_z32(peer_addr) {
            Ok(key) => key,
            Err(e) => {
                // 如果z-base-32解析失败，尝试标准FromStr
                match peer_addr.parse::<PublicKey>() {
                    Ok(key) => key,
                    Err(_) => {
                        return Err(anyhow!("无效的节点ID格式: {} (z-base-32或base32解析都失败)", peer_addr));
                    }
                }
            }
        };
            
        let endpoint_addr: EndpointAddr = EndpointAddr::from(public_key);
            
        // 使用iroh 0.95的正确connect API
        // 需要提供EndpointAddr和ALPN协议
        match self.endpoint.connect(endpoint_addr, b"williw-p2p").await {
            Ok(connection) => {
                // 存储连接
                let mut connections = self.connections.lock().await;
                connections.insert(peer_addr.to_string(), connection);
                info!("✅ 已连接到节点: {}", peer_addr);
                Ok(())
            }
            Err(e) => {
                error!("连接失败: {}", e);
                Err(anyhow!("无法连接到节点 {}: {}", peer_addr, e))
            }
        }
    }
    
    /// 发送消息到指定节点
    pub async fn send_message(&self, peer_id: &str, message: Vec<u8>) -> Result<()> {
        debug!("📤 发送消息到 {}: {} bytes", peer_id, message.len());
        
        let connections = self.connections.lock().await;
        if let Some(connection) = connections.get(peer_id) {
            // 使用iroh的uni流发送真实消息
            self.send_via_uni_stream(connection, &message).await?;
            debug!("✅ 消息发送成功");
            Ok(())
        } else {
            Err(anyhow!("未找到到节点 {} 的连接", peer_id))
        }
    }
    
    /// 通过iroh uni流发送消息
    async fn send_via_uni_stream(&self, connection: &Connection, message: &[u8]) -> Result<()> {
        // 打开单向流
        let mut send_stream = connection.open_uni().await?;
        
        // 发送消息长度前缀（4字节）
        let len_bytes = (message.len() as u32).to_le_bytes();
        send_stream.write_all(&len_bytes).await?;
        
        // 发送消息内容
        send_stream.write_all(message).await?;
        
        // 关闭流
        send_stream.finish();
        
        Ok(())
    }
    
    /// 广播消息到所有连接的节点
    pub async fn broadcast_message(&self, message: Vec<u8>) -> Result<usize> {
        let connections = self.connections.lock().await;
        let mut sent_count = 0;
        
        for (peer_id, connection) in connections.iter() {
            match self.send_via_uni_stream(connection, &message).await {
                Ok(_) => {
                    sent_count += 1;
                    debug!("✅ 消息已广播到 {}", peer_id);
                }
                Err(e) => {
                    warn!("❌ 广播到 {} 失败: {}", peer_id, e);
                }
            }
        }
        
        info!("📡 消息已广播到 {} 个节点", sent_count);
        Ok(sent_count)
    }
    
    /// 接收消息（简化版本）
    pub async fn receive_message(&self) -> Result<Option<(String, Vec<u8>)>> {
        // 尝试接受传入连接
        if let Some(incoming) = self.endpoint.accept().await {
            match incoming.accept() {
                Ok(accepting) => {
                    match accepting.await {
                        Ok(connection) => {
                            let peer_addr = "incoming_peer".to_string(); // 暂时使用固定字符串
                            info!("🔗 接收到来自 {} 的连接", peer_addr);
                            
                            // 尝试从连接接收数据
                            match self.receive_from_connection(&connection).await {
                                Ok(data) => {
                                    if !data.is_empty() {
                                        info!("📨 成功接收到 {} 字节的数据", data.len());
                                        return Ok(Some((peer_addr, data)));
                                    }
                                }
                                Err(e) => {
                                    warn!("⚠️ 从连接接收数据失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("⚠️ 接受连接失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️ 接受传入连接失败: {}", e);
                }
            }
        }
        
        // 检查现有连接是否有新消息
        let connections = self.connections.lock().await;
        for (peer_id, connection) in connections.iter() {
            match self.receive_from_connection(connection).await {
                Ok(data) => {
                    if !data.is_empty() {
                        info!("📨 从现有连接 {} 接收到 {} 字节", peer_id, data.len());
                        return Ok(Some((peer_id.clone(), data)));
                    }
                }
                Err(_) => {
                    // 忽略接收错误，继续检查其他连接
                }
            }
        }
        
        Ok(None)
    }
    
    /// 从连接接收消息
    async fn receive_from_connection(&self, connection: &Connection) -> Result<Vec<u8>> {
        // 等待传入的uni流
        match connection.accept_uni().await {
            Ok(mut recv_stream) => {
                // 读取消息长度前缀
                let mut len_bytes = [0u8; 4];
                recv_stream.read_exact(&mut len_bytes).await?;
                let message_len = u32::from_le_bytes(len_bytes) as usize;
                
                // 读取消息内容
                let mut message = vec![0u8; message_len];
                recv_stream.read_exact(&mut message).await?;
                
                debug!("📨 接收到 {} 字节的消息", message_len);
                Ok(message)
            }
            Err(e) => {
                Err(anyhow!("接收uni流失败: {}", e))
            }
        }
    }
    
    /// 获取节点ID
    pub fn node_id(&self) -> String {
        self.node_id.clone()
    }
    
    /// 获取连接统计
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let connections = self.connections.lock().await;
        ConnectionStats {
            active_connections: connections.len(),
            max_connections: self.config.max_connections,
            node_id: self.node_id.to_string(),
        }
    }
    
    /// 断开指定连接
    pub async fn disconnect(&self, peer_id: &str) -> Result<()> {
        info!("🔌 断开与节点 {} 的连接", peer_id);
        
        let mut connections = self.connections.lock().await;
        if let Some(_connection) = connections.remove(peer_id) {
            info!("✅ 已断开与节点 {} 的连接", peer_id);
            Ok(())
        } else {
            warn!("⚠️ 未找到到节点 {} 的连接", peer_id);
            Err(anyhow!("未找到连接"))
        }
    }
    
    /// 清理所有连接
    pub async fn disconnect_all(&self) {
        info!("🔌 断开所有连接");
        
        let mut connections = self.connections.lock().await;
        connections.clear();
        
        info!("✅ 所有连接已断开");
    }
}

/// 连接统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub active_connections: usize,
    pub max_connections: usize,
    pub node_id: String,
}

/// 消息类型标识
pub const FILE_TRANSFER_MESSAGE_TYPE: &str = "file_transfer";
pub const GOSSIP_MESSAGE_TYPE: &str = "gossip";
pub const CONTROL_MESSAGE_TYPE: &str = "control";

/// 包装消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrappedMessage {
    pub message_type: String,
    pub sender_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub payload: Vec<u8>,
}

impl WrappedMessage {
    pub fn new(message_type: String, sender_id: String, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            sender_id,
            timestamp: chrono::Utc::now(),
            payload,
        }
    }
    
    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
    
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }
}

/// 兼容原有的QuicGateway接口
pub struct QuicGateway {
    connection_manager: Arc<IrohConnectionManager>,
    received_messages: Arc<RwLock<Vec<SignedGossip>>>,
}

impl QuicGateway {
    pub async fn new(bind: std::net::SocketAddr) -> Result<Self> {
        let config = IrohConnectionConfig {
            bind_addr: bind.to_string(),
            ..Default::default()
        };
        
        let connection_manager = Arc::new(IrohConnectionManager::new(config).await?);
        let received_messages = Arc::new(RwLock::new(Vec::new()));
        
        Ok(Self {
            connection_manager,
            received_messages,
        })
    }

    pub async fn connect(&self, addr: std::net::SocketAddr) -> Result<()> {
        let addr_str = addr.to_string();
        self.connection_manager.connect_to_peer(&addr_str).await?;
        Ok(())
    }
    
    /// 测量到指定节点的网络距离
    pub async fn measure_network_distance(&self, _node_addr: &str) -> crate::types::NetworkDistance {
        // 返回默认的网络距离
        crate::types::NetworkDistance::new()
    }
    
    /// 获取本地网络的 DERP 节点延迟信息
    pub async fn get_local_derp_delays(&self) -> Vec<(String, u64)> {
        // 返回空的延迟信息
        Vec::new()
    }
    
    /// 获取本地网络报告
    pub async fn get_net_report(&self) -> Option<()> {
        // 返回None，因为我们现在不使用实际的iroh网络
        None
    }
    
    pub fn take_received_messages(&self) -> Vec<SignedGossip> {
        std::mem::take(&mut *self.received_messages.write())
    }

    pub async fn broadcast(&self, signed: &SignedGossip) -> bool {
        // 将SignedGossip序列化并通过iroh广播
        match serde_json::to_vec(signed) {
            Ok(data) => {
                let wrapped_message = WrappedMessage::new(
                    GOSSIP_MESSAGE_TYPE.to_string(),
                    self.connection_manager.node_id().to_string(),
                    data,
                );
                
                match self.connection_manager.broadcast_message(wrapped_message.serialize().unwrap_or_default()).await {
                    Ok(count) => count > 0,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
}

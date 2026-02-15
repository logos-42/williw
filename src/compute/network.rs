//! 分布式推理网络通信层
//!
//! 将分布式推理协调器与 iroh P2P 网络集成，实现跨节点通信

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tracing::{info, warn, debug};

use super::protocol::InferenceMessage;
use crate::comms::transport::iroh::{IrohConnectionManager, IrohConnectionConfig};

/// 分布式推理网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceNetworkConfig {
    /// 是否启用网络功能
    pub enabled: bool,
    /// iroh 绑定地址
    pub bind_addr: String,
    /// 引导节点列表
    pub bootstrap_nodes: Vec<String>,
    /// 消息超时时间（秒）
    pub message_timeout_secs: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
}

impl Default for InferenceNetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_addr: "0.0.0.0:0".to_string(),
            bootstrap_nodes: vec![],
            message_timeout_secs: 60,
            max_retries: 3,
            heartbeat_interval_secs: 30,
        }
    }
}

/// 网络消息包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// 消息类型
    pub msg_type: NetworkMessageType,
    /// 消息内容
    pub payload: Vec<u8>,
    /// 时间戳
    pub timestamp: i64,
    /// 源节点 ID
    pub source_node: String,
}

/// 网络消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkMessageType {
    /// 推理请求
    InferenceRequest,
    /// 推理结果
    InferenceResult,
    /// 分片执行
    ExecuteShard,
    /// 分片结果
    ShardResult,
    /// 节点发现
    NodeDiscovery,
    /// 心跳
    Heartbeat,
    /// 模型分片注册
    RegisterShard,
    /// 任务状态查询
    TaskStatusQuery,
}

/// 分布式推理网络接口
#[async_trait]
pub trait InferenceNetwork: Send + Sync {
    /// 发送推理消息到指定节点
    async fn send_inference_message(
        &self,
        peer_id: &str,
        message: InferenceMessage,
    ) -> Result<()>;
    
    /// 广播推理消息到所有节点
    async fn broadcast_inference_message(
        &self,
        message: InferenceMessage,
    ) -> Result<usize>;
    
    /// 接收推理消息
    async fn receive_inference_message(&self) -> Result<Option<(String, InferenceMessage)>>;
    
    /// 获取本节点 ID
    fn get_node_id(&self) -> &str;
    
    /// 获取连接的节点列表
    async fn get_connected_peers(&self) -> Vec<String>;
    
    /// 连接到远程节点
    async fn connect_to_peer(&self, peer_addr: &str) -> Result<()>;
}

/// 基于 iroh 的分布式推理网络实现
pub struct IrohInferenceNetwork {
    /// iroh 连接管理器
    connection_manager: Arc<IrohConnectionManager>,
    /// 配置
    config: InferenceNetworkConfig,
    /// 本节点 ID
    node_id: String,
    /// 推理消息接收通道
    inference_rx: Arc<Mutex<mpsc::Receiver<(String, InferenceMessage)>>>,
    inference_tx: mpsc::Sender<(String, InferenceMessage)>,
}

impl IrohInferenceNetwork {
    /// 创建新的分布式推理网络
    pub async fn new(config: InferenceNetworkConfig) -> Result<Self> {
        if !config.enabled {
            return Err(anyhow!("分布式推理网络未启用"));
        }
        
        info!("🌐 初始化分布式推理网络");
        
        // 创建 iroh 连接管理器
        let iroh_config = IrohConnectionConfig {
            bind_addr: config.bind_addr.clone(),
            node_id: None,
            bootstrap_nodes: config.bootstrap_nodes.clone(),
            enable_relay: true,
            max_connections: 50,
        };
        
        let connection_manager = Arc::new(IrohConnectionManager::new(iroh_config).await?);
        let node_id = connection_manager.node_id();
        
        info!("✅ 分布式推理网络已初始化，节点 ID: {}", node_id);
        
        // 创建推理消息通道
        let (inference_tx, inference_rx) = mpsc::channel(1000);
        
        let network = Self {
            connection_manager,
            config,
            node_id,
            inference_rx: Arc::new(Mutex::new(inference_rx)),
            inference_tx,
        };
        
        // 启动消息处理循环
        network.start_message_handler().await;
        
        Ok(network)
    }
    
    /// 启动消息处理循环
    async fn start_message_handler(&self) {
        let connection_manager = self.connection_manager.clone();
        let inference_tx = self.inference_tx.clone();
        
        tokio::spawn(async move {
            info!("🔄 启动推理消息处理循环");
            
            loop {
                match connection_manager.receive_message().await {
                    Ok(Some((peer_id, raw_message))) => {
                        // 尝试解析为网络消息
                        if let Ok(net_msg) = serde_json::from_slice::<NetworkMessage>(&raw_message) {
                            // 尝试解析为推理消息
                            if let Ok(inference_msg) = serde_json::from_slice::<InferenceMessage>(&net_msg.payload) {
                                if let Err(e) = inference_tx.send((peer_id, inference_msg)).await {
                                    warn!("发送推理消息到通道失败: {}", e);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // 没有消息，短暂休眠
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                    Err(e) => {
                        warn!("接收消息错误: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });
    }
    
    /// 序列化推理消息
    fn serialize_inference_message(&self, message: &InferenceMessage) -> Result<Vec<u8>> {
        let payload = serde_json::to_vec(message)?;
        let net_msg = NetworkMessage {
            msg_type: Self::get_message_type(message),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
            source_node: self.node_id.clone(),
        };
        Ok(serde_json::to_vec(&net_msg)?)
    }
    
    /// 获取消息类型
    fn get_message_type(message: &InferenceMessage) -> NetworkMessageType {
        match message {
            InferenceMessage::DistributedInferenceRequest { .. } => NetworkMessageType::ExecuteShard,
            InferenceMessage::DistributedInferenceResponse { .. } => NetworkMessageType::ShardResult,
            InferenceMessage::AggregatedInferenceResult { .. } => NetworkMessageType::ShardResult,
            InferenceMessage::ExecuteShard { .. } => NetworkMessageType::ExecuteShard,
            InferenceMessage::ExecutionResult { .. } => NetworkMessageType::ShardResult,
            InferenceMessage::RegisterShard { .. } => NetworkMessageType::RegisterShard,
            InferenceMessage::TaskStatusQuery { .. } => NetworkMessageType::TaskStatusQuery,
            InferenceMessage::TaskStatusResponse { .. } => NetworkMessageType::TaskStatusQuery,
            InferenceMessage::Heartbeat { .. } => NetworkMessageType::Heartbeat,
            InferenceMessage::QueryShard { .. } => NetworkMessageType::RegisterShard,
            InferenceMessage::ShardLocation { .. } => NetworkMessageType::RegisterShard,
            InferenceMessage::ShardTableSync { .. } => NetworkMessageType::RegisterShard,
        }
    }
    
    /// 获取连接地址（供其他节点连接）
    pub fn get_connection_addr(&self) -> Result<String> {
        // 返回节点 ID，其他节点可以通过中继连接
        Ok(self.node_id.clone())
    }
}

#[async_trait]
impl InferenceNetwork for IrohInferenceNetwork {
    async fn send_inference_message(
        &self,
        peer_id: &str,
        message: InferenceMessage,
    ) -> Result<()> {
        debug!("📤 发送推理消息到 {}: {:?}", peer_id, std::mem::discriminant(&message));
        
        let serialized = self.serialize_inference_message(&message)?;
        self.connection_manager.send_message(peer_id, serialized).await?;
        
        debug!("✅ 推理消息发送成功");
        Ok(())
    }
    
    async fn broadcast_inference_message(
        &self,
        message: InferenceMessage,
    ) -> Result<usize> {
        info!("📡 广播推理消息: {:?}", std::mem::discriminant(&message));
        
        let serialized = self.serialize_inference_message(&message)?;
        let count = self.connection_manager.broadcast_message(serialized).await?;
        
        info!("✅ 推理消息已广播到 {} 个节点", count);
        Ok(count)
    }
    
    async fn receive_inference_message(&self) -> Result<Option<(String, InferenceMessage)>> {
        let mut rx = self.inference_rx.lock().await;
        match rx.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(_) => Ok(None),
        }
    }
    
    fn get_node_id(&self) -> &str {
        &self.node_id
    }
    
    async fn get_connected_peers(&self) -> Vec<String> {
        // 从连接管理器获取连接列表
        // 简化实现，返回空列表
        vec![]
    }
    
    async fn connect_to_peer(&self, peer_addr: &str) -> Result<()> {
        info!("🔗 连接到推理节点: {}", peer_addr);
        self.connection_manager.connect_to_peer(peer_addr).await
    }
}

/// 模拟网络实现（用于测试）
pub struct MockInferenceNetwork {
    node_id: String,
    peers: Arc<RwLock<Vec<String>>>,
    message_log: Arc<RwLock<Vec<(String, String, InferenceMessage)>>>,
}

impl MockInferenceNetwork {
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            message_log: Arc::new(RwLock::new(vec![])),
        }
    }
    
    /// 添加模拟节点
    pub async fn add_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        peers.push(peer_id.to_string());
    }
    
    /// 获取消息日志
    pub async fn get_message_log(&self) -> Vec<(String, String, InferenceMessage)> {
        self.message_log.read().await.clone()
    }
}

#[async_trait]
impl InferenceNetwork for MockInferenceNetwork {
    async fn send_inference_message(
        &self,
        peer_id: &str,
        message: InferenceMessage,
    ) -> Result<()> {
        let mut log = self.message_log.write().await;
        log.push((self.node_id.clone(), peer_id.to_string(), message));
        Ok(())
    }
    
    async fn broadcast_inference_message(
        &self,
        message: InferenceMessage,
    ) -> Result<usize> {
        let peers = self.peers.read().await;
        let mut log = self.message_log.write().await;
        let count = peers.len();
        
        for peer_id in peers.iter() {
            log.push((self.node_id.clone(), peer_id.clone(), message.clone()));
        }
        
        Ok(count)
    }
    
    async fn receive_inference_message(&self) -> Result<Option<(String, InferenceMessage)>> {
        // 模拟网络不接收消息
        Ok(None)
    }
    
    fn get_node_id(&self) -> &str {
        &self.node_id
    }
    
    async fn get_connected_peers(&self) -> Vec<String> {
        self.peers.read().await.clone()
    }
    
    async fn connect_to_peer(&self, _peer_addr: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_network() {
        let network = MockInferenceNetwork::new("test_node");
        
        network.add_peer("peer_1").await;
        network.add_peer("peer_2").await;
        
        let message = InferenceMessage::Heartbeat {
            node_id: "test_node".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            load: 0.5,
            available_memory_mb: 1024,
        };
        
        let count = network.broadcast_inference_message(message).await.unwrap();
        assert_eq!(count, 2);
        
        let log = network.get_message_log().await;
        assert_eq!(log.len(), 2);
    }
}

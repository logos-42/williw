/**
 * Iroh传输层实现
 * 统一的iroh集成，包含Gossip消息和P2P文件传输
 */

use anyhow::{Result, anyhow};
use iroh::{Endpoint, endpoint::Connection, EndpointAddr, PublicKey, RelayUrl, TransportAddr};
use iroh::endpoint_info::EndpointIdExt;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};

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
    message_rx: Arc<Mutex<mpsc::Receiver<(String, Vec<u8>)>>>,
    node_id: String,
}

impl IrohConnectionManager {
    /// 创建新的连接管理器
    pub async fn new(config: IrohConnectionConfig) -> Result<Self> {
        info!("🔗 初始化 iroh 连接管理器, bind_addr: {}", config.bind_addr);
        
        // 创建iroh端点 - 使用配置中的地址
        let bind_addr = config.bind_addr.parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());
        let endpoint = Endpoint::builder()
            .bind_addr_v4(bind_addr)
            .alpns(vec![b"williw-p2p".to_vec()])  // 设置ALPN协议
            .bind()
            .await?;
        
        // 创建数据目录 - 使用统一目录
        let data_dir = std::path::PathBuf::from("./williw_p2p_data");
        std::fs::create_dir_all(&data_dir)?;
        
        let node_id = endpoint.id().to_z32();
        info!("✅ iroh 端点已创建，节点ID: {}", node_id);
        
        let (message_tx, message_rx) = mpsc::channel::<(String, Vec<u8>)>(1000);
        let connections = Arc::new(Mutex::new(HashMap::new()));

        // 后台持续接受新的连接，并将消息转发到接收队列
        Self::start_incoming_connection_task(
            endpoint.clone(),
            connections.clone(),
            message_tx.clone(),
        );
        
        Ok(Self {
            endpoint,
            config,
            connections,
            message_tx,
            message_rx: Arc::new(Mutex::new(message_rx)),
            node_id,
        })
    }
    
    /// 连接到远程节点
    pub async fn connect_to_peer(&self, peer_addr: &str) -> Result<()> {
        info!("🔗 连接到远程节点: {}", peer_addr);
        let (endpoint_addr, canonical_peer_id) = Self::parse_endpoint_descriptor(peer_addr)?;
            
        // 使用iroh 0.95的正确connect API
        // 需要提供EndpointAddr和ALPN协议
        match self.endpoint.connect(endpoint_addr, b"williw-p2p").await {
            Ok(connection) => {
                // 存储连接
                let mut connections = self.connections.lock().await;
                connections.insert(canonical_peer_id.clone(), connection.clone());

                Self::spawn_connection_reader(
                    canonical_peer_id.clone(),
                    connection,
                    self.message_tx.clone(),
                );

                info!("✅ 已连接到节点: {}", canonical_peer_id);
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

        let connection = {
            let connections = self.connections.lock().await;
            connections.get(peer_id).cloned()
        };

        if let Some(connection) = connection {
            // 使用iroh的uni流发送真实消息
            self.send_via_uni_stream(connection, &message).await?;
            debug!("✅ 消息发送成功");
            Ok(())
        } else {
            Err(anyhow!("未找到到节点 {} 的连接", peer_id))
        }
    }
    
    /// 通过iroh uni流发送消息
    async fn send_via_uni_stream(&self, connection: Connection, message: &[u8]) -> Result<()> {
        // 打开单向流
        let mut send_stream = connection.open_uni().await?;
        
        // 发送消息长度前缀（4字节）
        let len_bytes = (message.len() as u32).to_le_bytes();
        send_stream.write_all(&len_bytes).await?;
        
        // 发送消息内容
        send_stream.write_all(message).await?;
        
        // 关闭流
        let _ = send_stream.finish();
        
        Ok(())
    }
    
    /// 发送文件到指定节点
    pub async fn send_file(&self, peer_id: &str, file_path: &str) -> Result<(u64, String)> {
        use std::fs::File;
        use std::io::Read;
        
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            return Err(anyhow!("文件不存在: {}", file_path));
        }
        
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        
        // 读取文件内容
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        // 计算校验和
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        buffer.hash(&mut hasher);
        let checksum = format!("{:016x}", hasher.finish());
        
        // 发送文件数据
        self.send_message(peer_id, buffer).await?;
        
        log::info!("📤 文件 {} ({} bytes) 已发送到 {}", file_path, file_size, peer_id);
        
        Ok((file_size, checksum))
    }
    
    /// 广播消息到所有连接的节点
    pub async fn broadcast_message(&self, message: Vec<u8>) -> Result<usize> {
        let connections: Vec<(String, Connection)> = {
            let connections = self.connections.lock().await;
            connections
                .iter()
                .map(|(peer_id, connection)| (peer_id.clone(), connection.clone()))
                .collect()
        };
        let mut sent_count = 0;
        
        for (peer_id, connection) in connections {
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
        let mut receiver = self.message_rx.lock().await;
        match timeout(Duration::from_millis(50), receiver.recv()).await {
            Ok(Some(message)) => Ok(Some(message)),
            Ok(None) => Ok(None),
            Err(_) => Ok(None),
        }
    }
    
    /// 从连接接收消息
    async fn receive_from_connection(connection: &Connection) -> Result<Vec<u8>> {
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

    fn start_incoming_connection_task(
        endpoint: Endpoint,
        connections: Arc<Mutex<HashMap<String, Connection>>>,
        message_tx: mpsc::Sender<(String, Vec<u8>)>,
    ) {
        tokio::spawn(async move {
            while let Some(incoming) = endpoint.accept().await {
                match incoming.accept() {
                    Ok(accepting) => match accepting.await {
                        Ok(connection) => {
                            let peer_id = connection.remote_id().to_z32();
                            info!("🔗 接收到来自 {} 的连接", peer_id);

                            {
                                let mut guard = connections.lock().await;
                                guard.insert(peer_id.clone(), connection.clone());
                            }

                            Self::spawn_connection_reader(peer_id, connection, message_tx.clone());
                        }
                        Err(e) => {
                            warn!("⚠️ 接受连接失败: {}", e);
                        }
                    },
                    Err(e) => {
                        warn!("⚠️ 接受传入连接失败: {}", e);
                    }
                }
            }
        });
    }

    fn spawn_connection_reader(
        peer_id: String,
        connection: Connection,
        message_tx: mpsc::Sender<(String, Vec<u8>)>,
    ) {
        tokio::spawn(async move {
            loop {
                match Self::receive_from_connection(&connection).await {
                    Ok(data) => {
                        if data.is_empty() {
                            continue;
                        }
                        if message_tx.send((peer_id.clone(), data)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("连接 {} 读取停止: {}", peer_id, e);
                        break;
                    }
                }
            }
        });
    }

    fn parse_endpoint_descriptor(peer_addr: &str) -> Result<(EndpointAddr, String)> {
        #[derive(Debug, Deserialize)]
        struct JsonDescriptor {
            id: String,
            #[serde(default)]
            addrs: Vec<String>,
        }

        let peer_addr = peer_addr.trim();
        if peer_addr.is_empty() {
            return Err(anyhow!("peer descriptor is empty"));
        }

        if peer_addr.starts_with('{') {
            let descriptor: JsonDescriptor = serde_json::from_str(peer_addr)
                .map_err(|e| anyhow!("无效 JSON peer 描述: {}", e))?;
            let endpoint_id = Self::parse_endpoint_id(&descriptor.id)?;
            let transports = Self::parse_transports(&descriptor.addrs)?;
            let endpoint_addr = if transports.is_empty() {
                EndpointAddr::from(endpoint_id)
            } else {
                EndpointAddr::from_parts(endpoint_id, transports)
            };
            return Ok((endpoint_addr, endpoint_id.to_z32()));
        }

        if let Some((id_part, addrs_part)) = peer_addr.split_once('@') {
            let endpoint_id = Self::parse_endpoint_id(id_part)?;
            let addr_list: Vec<String> = addrs_part
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let transports = Self::parse_transports(&addr_list)?;
            let endpoint_addr = if transports.is_empty() {
                EndpointAddr::from(endpoint_id)
            } else {
                EndpointAddr::from_parts(endpoint_id, transports)
            };
            return Ok((endpoint_addr, endpoint_id.to_z32()));
        }

        let endpoint_id = Self::parse_endpoint_id(peer_addr)?;
        Ok((EndpointAddr::from(endpoint_id), endpoint_id.to_z32()))
    }

    fn parse_endpoint_id(raw: &str) -> Result<PublicKey> {
        let raw = raw.trim();
        if let Ok(key) = PublicKey::from_z32(raw) {
            return Ok(key);
        }
        if let Ok(key) = raw.parse::<PublicKey>() {
            return Ok(key);
        }
        Err(anyhow!("无效的节点ID: {}", raw))
    }

    fn parse_transports(addrs: &[String]) -> Result<Vec<TransportAddr>> {
        let mut transports = Vec::new();
        for raw in addrs {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            if let Ok(ip) = raw.parse::<SocketAddr>() {
                transports.push(TransportAddr::Ip(ip));
                continue;
            }
            if let Ok(relay_url) = raw.parse::<RelayUrl>() {
                transports.push(TransportAddr::Relay(relay_url));
                continue;
            }
            return Err(anyhow!(
                "无法解析地址 '{}', 仅支持 'ip:port' 或 relay url",
                raw
            ));
        }
        Ok(transports)
    }

    /// 输出可共享的连接描述符：`<endpoint_id>@<addr1>,<addr2>...`
    pub fn endpoint_descriptor(&self) -> String {
        let endpoint_addr = self.endpoint.addr();
        let mut addrs: Vec<String> = endpoint_addr.ip_addrs().map(|addr| addr.to_string()).collect();
        addrs.extend(endpoint_addr.relay_urls().map(|relay| relay.to_string()));

        if addrs.is_empty() {
            self.node_id.clone()
        } else {
            format!("{}@{}", self.node_id, addrs.join(","))
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
        if connections.remove(peer_id).is_some() {
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
        
        let gateway = Self {
            connection_manager,
            received_messages,
        };

        let connection_manager = gateway.connection_manager.clone();
        let received_messages = gateway.received_messages.clone();
        tokio::spawn(async move {
            loop {
                match connection_manager.receive_message().await {
                    Ok(Some((peer_id, data))) => {
                        if data.is_empty() {
                            continue;
                        }

                        let payload = match WrappedMessage::deserialize(&data) {
                            Ok(wrapped) => {
                                if wrapped.message_type != GOSSIP_MESSAGE_TYPE {
                                    debug!(
                                        "收到非 gossip 消息，类型={}，来源={}",
                                        wrapped.message_type,
                                        peer_id
                                    );
                                    continue;
                                }
                                wrapped.payload
                            }
                            Err(_) => data,
                        };

                        match serde_json::from_slice::<SignedGossip>(&payload) {
                            Ok(message) => {
                                received_messages.write().push(message);
                            }
                            Err(e) => {
                                debug!("忽略无法解析的 gossip 负载: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        // 无消息，继续轮询
                    }
                    Err(e) => {
                        warn!("接收消息失败: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
        
        // 输出真实的 iroh 节点 ID
        info!("🎯 QuicGateway 创建完成，节点ID: {}", gateway.node_id());
        
        Ok(gateway)
    }

    pub async fn connect(&self, addr: std::net::SocketAddr) -> Result<()> {
        self.connection_manager.connect_to_peer(&addr.to_string()).await
    }

    pub async fn connect_peer(&self, peer_descriptor: String) -> Result<()> {
        self.connection_manager.connect_to_peer(&peer_descriptor).await
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

    /// 获取真实的 iroh 节点 ID
    pub fn node_id(&self) -> String {
        self.connection_manager.node_id()
    }

    pub fn endpoint_descriptor(&self) -> String {
        self.connection_manager.endpoint_descriptor()
    }
}

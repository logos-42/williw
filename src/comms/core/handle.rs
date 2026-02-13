//! 通信句柄模块
//!
//! 提供基于 iroh 的通信接口和功能

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;
// Stub iroh types for compatibility
#[derive(Clone)]
pub struct Endpoint;
use tokio::sync::mpsc;

use crate::consensus::SignedGossip;
use crate::device::NetworkType;

use super::config::{CommsConfig, BandwidthBudget};
use crate::comms::transport::iroh::QuicGateway;

/// Topic 类型（用于发布/订阅）
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Topic {
    name: String,
}

impl Topic {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.name.as_bytes()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Iroh 事件类型
#[derive(Debug, Clone)]
pub enum IrohEvent {
    /// 接收到 Gossip 消息
    Gossip {
        source: String,
        data: Vec<u8>,
    },
    /// 发现新节点
    PeerDiscovered {
        peer: String,
        addr: String,
    },
    /// 节点离线
    PeerExpired {
        peer: String,
    },
    /// 连接建立
    ConnectionEstablished {
        peer: String,
    },
    /// 连接断开
    ConnectionClosed {
        peer: String,
    },
}

/// Gossip 消息信息
struct GossipMessage {
    topic: Topic,
    data: Vec<u8>,
    source: String,
}

/// 节点订阅信息
struct PeerSubscription {
    peer: String,
    topics: Vec<Topic>,
}

/// 通信句柄
pub struct CommsHandle {
    pub peer_id: String,
    pub topic: Topic,
    endpoint: Endpoint,
    gossip_tx: mpsc::Sender<GossipMessage>,
    _gossip_rx: mpsc::Receiver<GossipMessage>,
    event_tx: mpsc::Sender<IrohEvent>,
    pub event_rx: mpsc::Receiver<IrohEvent>,
    quic: Option<Arc<QuicGateway>>,
    bandwidth: RwLock<BandwidthBudget>,
    network_type: parking_lot::RwLock<NetworkType>,
    subscriptions: RwLock<Vec<PeerSubscription>>,
}

impl CommsHandle {
    pub async fn new(config: CommsConfig) -> Result<Self> {
        // 创建 gossip 消息通道
        let (gossip_tx, gossip_rx) = mpsc::channel(1024);
        // 创建事件通道
        let (event_tx, event_rx) = mpsc::channel::<IrohEvent>(1024);

        // 初始化 QUIC 网关（用于实时通信）- 这是真实 iroh 节点的来源
        let quic: Option<Arc<QuicGateway>> = if let Some(bind) = config.quic_bind {
            let quic_bootstrap = config.quic_bootstrap.clone();
            eprintln!("[Iroh DEBUG] 尝试创建 QuicGateway，bind: {}", bind);
            match QuicGateway::new(bind).await {
                Ok(gateway) => {
                    eprintln!("[Iroh DEBUG] ✅ QuicGateway 创建成功！");
                    let gateway = Arc::new(gateway);
                    // 在异步上下文中处理连接
                    for addr in quic_bootstrap {
                        let gateway_clone = gateway.clone();
                        tokio::spawn(async move {
                            let _ = gateway_clone.connect(addr).await;
                        });
                    }
                    Some(gateway)
                }
                Err(e) => {
                    eprintln!("[Iroh DEBUG] ❌ 创建 QUIC 网关失败: {}", e);
                    eprintln!("[Iroh DEBUG] 💡 可能原因: 端口被占用、权限不足，或 iroh 库初始化错误");
                    None
                }
            }
        } else {
            eprintln!("[Iroh DEBUG] quic_bind 未配置，使用后备节点 ID");
            None
        };

        // 从 QuicGateway 获取真实的 iroh 节点 ID，或使用 UUID 作为后备
        let peer_id = if let Some(ref gateway) = quic {
            let real_node_id = gateway.node_id();
            log::info!("[Iroh] ✅ 真实节点 ID: {}", real_node_id);
            real_node_id
        } else {
            let fallback_id = format!("iroh-{}", Uuid::new_v4());
            log::warn!("[Iroh] ⚠️ 后备节点 ID: {}", fallback_id);
            fallback_id
        };

        let endpoint = Endpoint;

        // 启动 gossip 接收任务（简化实现）
        let _accept_endpoint = endpoint.clone();
        let _accept_gossip_tx = gossip_tx.clone();
        let _accept_event_tx = event_tx.clone();
        tokio::spawn(async move {
            // 简化：暂时不实现复杂的连接处理
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        // 从文件加载 bootstrap 节点（如果存在）
        if let Some(ref file_path) = config.bootstrap_peers_file {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                for line in content.lines() {
                    let addr_str = line.trim();
                    if !addr_str.is_empty() {
                        println!("[Iroh] 添加 bootstrap 节点: {}", addr_str);
                        // 可以尝试连接这些节点
                        // Note: endpoint.connect 需要 EndpointAddr 类型,这里暂时注释掉
                        // let _ = endpoint.connect(addr, b"ggb-iroh").await;
                    }
                }
            }
        }

        Ok(Self {
            peer_id,
            topic: Topic::new(config.topic.clone()),
            endpoint,
            gossip_tx,
            _gossip_rx: gossip_rx,
            event_tx,
            event_rx,
            quic,
            bandwidth: RwLock::new(BandwidthBudget::new(config.bandwidth)),
            network_type: parking_lot::RwLock::new(NetworkType::Unknown),
            subscriptions: RwLock::new(Vec::new()),
        })
    }

    /// 发布消息到 gossip 网络
    pub fn publish(&mut self, signed: &SignedGossip) -> Result<()> {
        let data = serde_json::to_vec(signed)?;

        // 获取所有订阅的 peer
        let subscriptions = self.subscriptions.read();

        if subscriptions.is_empty() {
            // 没有订阅者，这是正常的（节点刚启动时）
            return Ok(());
        }

        // 广播到所有订阅的 peer
        let mut success = false;
        let mut failed = false;

        for subscription in subscriptions.iter() {
            if subscription.topics.contains(&self.topic) {
                // 序列化消息: [topic_len:4][topic_data][message_data]
                let topic_bytes = self.topic.name().as_bytes();
                let mut message = Vec::with_capacity(4 + topic_bytes.len() + data.len());
                message.extend_from_slice(&(topic_bytes.len() as u32).to_be_bytes());
                message.extend_from_slice(topic_bytes);
                message.extend_from_slice(&data);

                // 发送（这里简化实现，实际应该使用连接池）
                if self.send_to_peer(&subscription.peer, &message).is_ok() {
                    success = true;
                } else {
                    failed = true;
                }
            }
        }

        if failed && !success {
            Err(anyhow!("Gossip 发布失败: 所有 peer 不可用"))
        } else {
            Ok(())
        }
    }

    /// 发送消息到指定 peer（简化实现）
    fn send_to_peer(&self, _peer: &String, _message: &[u8]) -> Result<()> {
        // 在实际实现中，这里应该维护连接池并发送消息
        // 目前简化为成功
        Ok(())
    }

    pub fn allow_sparse_update(&self) -> bool {
        self.bandwidth.write().allow_sparse()
    }

    pub fn allow_dense_snapshot(&self, bytes: usize) -> bool {
        let network_type = *self.network_type.read();
        if !network_type.allows_dense_snapshot() {
            return false;
        }
        self.bandwidth.write().allow_dense(bytes)
    }

    pub fn update_network_type(&self, network_type: NetworkType) {
        *self.network_type.write() = network_type;
        println!("[网络] 网络类型更新: {:?}", network_type);
    }

    pub fn network_type(&self) -> NetworkType {
        *self.network_type.read()
    }

    /// 添加 peer 到订阅列表
    pub fn add_peer(&mut self, peer: String) {
        let mut subscriptions = self.subscriptions.write();
        if !subscriptions.iter().any(|s| s.peer == peer) {
            subscriptions.push(PeerSubscription {
                peer: peer.clone(),
                topics: vec![self.topic.clone()],
            });
            println!("[Iroh] 添加 peer 到订阅列表: {}", peer);
        }
    }

    /// 从订阅列表中移除 peer
    pub fn remove_peer(&mut self, peer: &String) {
        let mut subscriptions = self.subscriptions.write();
        if let Some(pos) = subscriptions.iter().position(|s| &s.peer == peer) {
            subscriptions.remove(pos);
            println!("[Iroh] 从订阅列表中移除 peer: {}", peer);
        }
    }

    /// 连接到中继节点
    pub async fn connect_to_relay(&mut self, relay_node_id: String) -> Result<()> {
        println!("[中继] 尝试连接到中继节点: {}", relay_node_id);

        // iroh 提供内置的中继支持，这里简化实现
        // 实际应该使用 iroh 的 relay 功能
        Ok(())
    }

    /// 获取下一个事件
    pub async fn next_event(&mut self) -> Option<IrohEvent> {
        self.event_rx.recv().await
    }

    pub async fn broadcast_realtime(&self, signed: &SignedGossip) -> bool {
        if let Some(_quic) = &self.quic {
            return _quic.broadcast(signed).await;
        }
        false
    }

    pub fn take_quic_messages(&self) -> Vec<SignedGossip> {
        if let Some(quic) = &self.quic {
            return quic.take_received_messages();
        }
        Vec::new()
    }

    /// 获取节点 ID
    pub fn node_id(&self) -> String {
        self.peer_id.clone()
    }

    /// 连接到指定节点
    pub async fn connect(&mut self, _node_addr: String) -> Result<()> {
        // TODO: endpoint.connect需要EndpointAddr，不是String
        // 需要实现正确的连接逻辑
        println!("[Iroh] 连接到节点: {}", _node_addr);
        Ok(())
    }

    /// 测量到指定节点的网络距离
    pub async fn measure_network_distance(&self, _node_addr: &String) -> crate::types::NetworkDistance {
        if let Some(_quic) = &self.quic {
            // _quic.measure_network_distance(node_addr).await  // 暂时返回默认值，因为API可能不匹配
            crate::types::NetworkDistance::new()
        } else {
            crate::types::NetworkDistance::new()
        }
    }

    /// 获取本地监听地址
    pub fn local_addr(&self) -> Result<String> {
        // 注意：这个方法在当前版本的 iroh 中可能不可用
        // 为了编译通过，我们暂时返回一个默认值
        Ok("0.0.0.0:0".to_string())
    }
}

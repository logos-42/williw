//! 通讯模块
//!
//! 使用 iroh 作为底层通讯协议
//!
//! ## 模块结构
//! - `core`: 核心通信功能（配置、句柄、路由）
//! - `p2p`: P2P文件分发（分发器、发送端、接收端）
//! - `frontend`: 前端集成（管理器、启动器、Web）
//! - `transport`: 传输层（iroh集成、传输协议）
//! - `monitoring`: 监控（仪表板）
//! - `integration`: 应用集成

pub mod core;
pub mod p2p;
pub mod frontend;
pub mod transport;
pub mod monitoring;
pub mod integration;

// 重新导出常用类型
pub use core::{CommsConfig, BandwidthBudgetConfig, CommsHandle, IrohEvent, Topic};
pub use p2p::{P2PModelDistributor, TransferEvent, EventManager, get_global_event_manager};
pub use transport::{IrohConnectionManager, IrohConnectionConfig, ConnectionStats, WrappedMessage};
pub use monitoring::MonitoringDashboard;
pub use frontend::{P2PFrontendManager, P2PFrontendStarter};
pub use integration::{P2PAppIntegration, P2PAppFactory, P2PEnabledApp};

// 导出P2P发送端和接收端
pub use p2p::sender::{P2PModelSender, P2PSenderArgs};
pub use p2p::receiver::{P2PModelReceiver, P2PReceiverArgs};
pub use transport::protocol::{FileTransferProtocol, TransferProtocolConfig, ChecksumAlgorithm};

// 为了向后兼容，导出旧模块名
pub use p2p::distributor as p2p_distributor;
pub use frontend::manager as p2p_frontend_manager;
pub use frontend::starter as p2p_frontend_starter;

use anyhow::Result;

/// 网络配置
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub addr: std::net::SocketAddr,
    pub enable_encryption: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:0".parse().unwrap(),
            enable_encryption: true,
        }
    }
}

/// 网络句柄
pub struct NetworkHandle {
    _placeholder: std::marker::PhantomData<()>,
}

impl NetworkHandle {
    /// 创建新的网络句柄
    pub async fn new(_config: NetworkConfig) -> Result<Self> {
        // 模拟 iroh 网络句柄的创建
        // 实际实现将取决于 iroh 的具体 API 版本
        Ok(Self { _placeholder: std::marker::PhantomData })
    }
    
    /// 获取节点ID
    pub fn node_id(&self) -> String {  // 使用 String 代替 iroh::net::NodeId
        // 返回模拟节点ID
        "SIMULATED_NODE_ID".to_string()
    }
    
    /// 关闭网络句柄
    pub async fn shutdown(self) -> Result<()> {
        // 模拟关闭操作
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_handle_creation() -> Result<()> {
        let config = NetworkConfig::default();
        let handle = NetworkHandle::new(config).await?;
        handle.shutdown().await?;
        Ok(())
    }
}

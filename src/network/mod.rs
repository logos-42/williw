//! 网络通信模块
//! 
//! 提供统一的网络通信接口，支持多种传输协议和智能路由。

pub mod transport;
pub mod routing;
pub mod latency;

// 重新导出公共接口
pub use transport::{TransportConfig, TransportStats, TransportType, create_transport, Transport};
pub use routing::{RoutingConfig, RoutingStats, SimpleRouter, create_router, Router};
pub use latency::*;

/// 网络配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    /// 传输协议配置
    pub transport: transport::TransportConfig,
    /// 路由配置
    pub routing: routing::RoutingConfig,
    /// 带宽预算配置
    pub bandwidth_budget: BandwidthBudgetConfig,
}

/// 带宽预算配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BandwidthBudgetConfig {
    /// 总带宽限制（MB/s）
    pub total_bandwidth_mbps: f64,
    /// 每个连接的带宽限制（MB/s）
    pub per_connection_bandwidth_mbps: f64,
    /// 是否启用动态带宽调整
    pub enable_dynamic_adjustment: bool,
    /// 带宽监控间隔（秒）
    pub monitoring_interval_secs: u64,
}

impl Default for BandwidthBudgetConfig {
    fn default() -> Self {
        Self {
            total_bandwidth_mbps: 100.0,
            per_connection_bandwidth_mbps: 10.0,
            enable_dynamic_adjustment: true,
            monitoring_interval_secs: 5,
        }
    }
}

/// 网络句柄
pub struct NetworkHandle {
    transport: transport::IrohTransport,
    router: routing::SimpleRouter,
    config: NetworkConfig,
}

impl NetworkHandle {
    /// 创建新的网络句柄
    pub async fn new(config: NetworkConfig) -> anyhow::Result<Self> {
        let transport = transport::create_transport(&config.transport).await?;
        let router = routing::create_router(&config.routing).await?;
        
        Ok(Self {
            transport,
            router,
            config,
        })
    }
    
    /// 发送消息
    pub async fn send(&self, destination: &str, message: &[u8]) -> anyhow::Result<()> {
        let routing_route = self.router.select_route(destination).await?;
        // Convert routing RouteInfo to transport RouteInfo
        let transport_route = transport::RouteInfo {
            destination: routing_route.destination,
            transport_type: transport::TransportType::Iroh,
            address: routing_route.path.first().unwrap_or(&destination.to_string()).clone(),
            quality_score: routing_route.quality_score,
        };
        self.transport.send(&transport_route, message).await
    }
    
    /// 接收消息
    pub async fn receive(&self) -> anyhow::Result<(String, Vec<u8>)> {
        self.transport.receive().await
    }
    
    /// 获取网络统计信息
    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            transport_stats: self.transport.get_stats(),
            routing_stats: self.router.get_stats(),
        }
    }
}

/// 网络统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkStats {
    pub transport_stats: transport::TransportStats,
    pub routing_stats: routing::RoutingStats,
}

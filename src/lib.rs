//! williw 库模块
//!
//! 将核心功能导出为库，以便测试和集成使用
//! 使用 iroh 作为通讯模块，nori 作为加密手段

#![allow(non_snake_case)]

// 核心模块
pub mod device;
pub mod crypto;
pub mod consensus;

// Solana 区块链集成 - 暂时禁用，由于依赖冲突
// pub mod solana;

// Cloudflare Workers 集成
#[cfg(feature = "workers")]
pub mod workers;

// Android JNI 集成
#[cfg(feature = "android")]
pub mod android;

// 节点模块
pub mod node;

// 训练模块
pub mod training;

// 拓扑模块
pub mod topology;

// 统计模块
pub mod stats;

// 配置模块
pub mod config;

// 通讯模块 - 使用 iroh
pub mod comms;

// 类型定义模块
pub mod types;

// 零知识证明模块 - 使用 nori
#[cfg(feature = "zk_proof")]
pub mod zk;

// 网络模块（包含FFI接口）
pub mod network;

// 重新导出常用类型
pub use device::{DeviceConfig, DeviceCapabilities, DeviceManager};
pub use consensus::{ConsensusConfig, ConsensusEngine};
pub use node::Node;

// 重新导出Android模块
#[cfg(feature = "android")]
pub use android::*;

// Agent 模块
pub mod agent;

// 类型别名
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// williw 应用
pub struct WilliwApp {
    config: config::AppConfig,
    device_manager: DeviceManager,
    network: Option<comms::NetworkHandle>,
}

impl WilliwApp {
    /// 创建新的 williw 应用
    pub fn new(config: config::AppConfig) -> Result<Self> {
        let device_manager = DeviceManager::new();

        Ok(Self {
            config,
            device_manager,
            network: None,
        })
    }
    
    /// 启动应用
    pub async fn start(&mut self) -> Result<()> {
        // 初始化网络
        let network_config = comms::NetworkConfig::default();
        let network_handle = comms::NetworkHandle::new(network_config).await?;
        self.network = Some(network_handle);
        
        // 启动设备管理器
        // 注意：这里假设 DeviceManager 有适当的初始化方法
        
        Ok(())
    }
    
    /// 停止应用
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(network) = self.network.take() {
            network.shutdown().await?;
        }
        Ok(())
    }
    
    /// 获取应用状态
    pub fn get_status(&self) -> AppStatus {
        AppStatus {
            config: self.config.clone(),
            device_capabilities: self.device_manager.get(), // 使用正确的 get 方法
            network_connected: self.network.is_some(),
        }
    }
}

/// 应用状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppStatus {
    pub config: config::AppConfig,
    pub device_capabilities: DeviceCapabilities,
    pub network_connected: bool,
}
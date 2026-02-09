//! 计算管理模块
//!
//! 提供GPU节点管理、去中心化算力共享和任务调度功能

pub mod gpu_manager;
pub mod decentralized_compute;

pub use gpu_manager::*;
pub use decentralized_compute::*;

use std::sync::Arc;
use tokio::sync::RwLock;

/// 计算资源管理器
pub struct ComputeResourceManager {
    /// GPU管理器
    gpu_manager: Arc<GpuManager>,
    /// 去中心化计算网络
    compute_network: Arc<RwLock<Option<DecentralizedComputeNetwork>>>,
    /// 节点ID
    node_id: String,
}

impl ComputeResourceManager {
    /// 创建计算资源管理器
    pub async fn new(node_id: String) -> Result<Self, String> {
        let gpu_manager = Arc::new(GpuManager::new(node_id.clone()).await?);
        
        Ok(Self {
            gpu_manager,
            compute_network: Arc::new(RwLock::new(None)),
            node_id,
        })
    }

    /// 初始化计算资源
    pub async fn initialize(&self) -> Result<(), String> {
        println!("🔧 [COMPUTE] Initializing compute resources...");

        // 初始化GPU管理器网络
        self.gpu_manager.initialize_network().await?;

        // 初始化去中心化计算网络
        let network = DecentralizedComputeNetwork::new(self.node_id.clone()).await?;
        network.start().await?;
        
        *self.compute_network.write().await = Some(network);

        println!("✅ [COMPUTE] Compute resources initialized");
        Ok(())
    }

    /// 启动心跳监控
    pub async fn start_monitoring(&self) {
        self.gpu_manager.start_heartbeat_monitor().await;
    }

    /// 获取GPU管理器
    pub fn gpu_manager(&self) -> Arc<GpuManager> {
        self.gpu_manager.clone()
    }

    /// 获取计算网络
    pub async fn get_network(&self) -> Option<Arc<DecentralizedComputeNetwork>> {
        self.compute_network.read().await.as_ref().map(|n| Arc::new(n.clone()))
    }
}

// 为DecentralizedComputeNetwork实现Clone以支持Arc::new
impl Clone for DecentralizedComputeNetwork {
    fn clone(&self) -> Self {
        // 注意：这里简化实现，实际可能需要更复杂的克隆逻辑
        panic!("Direct clone of DecentralizedComputeNetwork is not supported")
    }
}

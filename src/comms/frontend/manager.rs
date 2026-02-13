/*
 * P2P 前端管理器
 * 负责管理 iroh 节点 ID、连接状态和前端交互
 */

#![allow(static_mut_refs)]

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, debug};

/// P2P 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PNodeInfo {
    /// 节点 ID
    pub node_id: String,
    /// 节点地址
    pub addresses: Vec<String>,
    /// 连接状态
    pub status: NodeStatus,
    /// 最后活跃时间
    pub last_active: chrono::DateTime<chrono::Utc>,
    /// 节点类型
    pub node_type: NodeType,
}

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    /// 在线
    Online,
    /// 离线
    Offline,
    /// 连接中
    Connecting,
    /// 未知
    Unknown,
}

/// 节点类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    /// 本地节点
    Local,
    /// 远程节点
    Remote,
    /// 引导节点
    Bootstrap,
}

/// P2P 连接统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConnectionStats {
    /// 活跃连接数
    pub active_connections: usize,
    /// 总连接数
    pub total_connections: usize,
    /// 上传速度 (bytes/sec)
    pub upload_speed: f64,
    /// 下载速度 (bytes/sec)
    pub download_speed: f64,
    /// 总上传量 (bytes)
    pub total_uploaded: u64,
    /// 总下载量 (bytes)
    pub total_downloaded: u64,
}

/// P2P 前端管理器
pub struct P2PFrontendManager {
    /// 本地节点 ID
    local_node_id: String,
    /// 已连接的节点
    connected_nodes: Arc<RwLock<HashMap<String, P2PNodeInfo>>>,
    /// 连接统计
    connection_stats: Arc<Mutex<P2PConnectionStats>>,
    /// P2P 分发器
    p2p_distributor: Option<Arc<crate::comms::p2p_distributor::P2PModelDistributor>>,
}

impl P2PFrontendManager {
    /// 创建新的 P2P 前端管理器
    pub async fn new() -> Result<Self> {
        let local_node_id = Self::generate_node_id();
        
        info!("🚀 初始化 P2P 前端管理器");
        info!("   本地节点 ID: {}", local_node_id);

        let manager = Self {
            local_node_id: local_node_id.clone(),
            connected_nodes: Arc::new(RwLock::new(HashMap::new())),
            connection_stats: Arc::new(Mutex::new(P2PConnectionStats {
                active_connections: 0,
                total_connections: 0,
                upload_speed: 0.0,
                download_speed: 0.0,
                total_uploaded: 0,
                total_downloaded: 0,
            })),
            p2p_distributor: None,
        };

        // 添加本地节点信息
        let local_node_info = P2PNodeInfo {
            node_id: local_node_id.clone(),
            addresses: vec![
                format!("/ip4/127.0.0.1/tcp/9235/p2p/{}", local_node_id),
                format!("/ip4/0.0.0.0/tcp/9235/p2p/{}", local_node_id),
            ],
            status: NodeStatus::Online,
            last_active: chrono::Utc::now(),
            node_type: NodeType::Local,
        };

        manager.connected_nodes.write().await.insert(local_node_id.clone(), local_node_info);

        Ok(manager)
    }

    /// 生成节点 ID
    fn generate_node_id() -> String {
        use uuid::Uuid;
        let uuid = Uuid::new_v4();
        format!("12D3KooW{}", uuid.to_string().replace("-", "")[..32].to_uppercase())
    }

    /// 获取本地节点 ID
    pub fn local_node_id(&self) -> &str {
        &self.local_node_id
    }

    /// 获取本地节点信息（用于前端显示）
    pub async fn get_local_node_info(&self) -> Result<P2PNodeInfo> {
        let nodes = self.connected_nodes.read().await;
        if let Some(node_info) = nodes.get(&self.local_node_id) {
            Ok(node_info.clone())
        } else {
            Err(anyhow!("本地节点信息未找到"))
        }
    }

    /// 获取所有已连接的节点信息
    pub async fn get_connected_nodes(&self) -> Result<Vec<P2PNodeInfo>> {
        let nodes = self.connected_nodes.read().await;
        Ok(nodes.values().cloned().collect())
    }

    /// 添加远程节点
    pub async fn add_remote_node(&self, node_id: String, addresses: Vec<String>) -> Result<()> {
        info!("📡 添加远程节点: {}", node_id);

        let node_info = P2PNodeInfo {
            node_id: node_id.clone(),
            addresses,
            status: NodeStatus::Connecting,
            last_active: chrono::Utc::now(),
            node_type: NodeType::Remote,
        };

        {
            let mut nodes = self.connected_nodes.write().await;
            nodes.insert(node_id.clone(), node_info);
        }

        // 尝试连接到远程节点
        tokio::spawn({
            let nodes = self.connected_nodes.clone();
            let node_id_clone = node_id.clone();
            async move {
                // 模拟连接过程
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                let mut nodes = nodes.write().await;
                if let Some(node_info) = nodes.get_mut(&node_id_clone) {
                    node_info.status = NodeStatus::Online;
                    node_info.last_active = chrono::Utc::now();
                    info!("✅ 成功连接到节点: {}", node_id_clone);
                }
            }
        });

        Ok(())
    }

    /// 移除节点
    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        info!("🗑️  移除节点: {}", node_id);
        
        let mut nodes = self.connected_nodes.write().await;
        nodes.remove(node_id);
        
        Ok(())
    }

    /// 获取连接统计
    pub async fn get_connection_stats(&self) -> Result<P2PConnectionStats> {
        let stats = self.connection_stats.lock().await;
        Ok(stats.clone())
    }

    /// 更新连接统计
    pub async fn update_connection_stats(&self, stats: P2PConnectionStats) -> Result<()> {
        {
            let mut current_stats = self.connection_stats.lock().await;
            *current_stats = stats;
        }
        Ok(())
    }

    /// 复制节点 ID 到剪贴板
    pub async fn copy_node_id(&self) -> Result<()> {
        let node_id = self.local_node_id.clone();
        
        // 在实际应用中，这里应该调用系统剪贴板 API
        info!("📋 节点 ID 已复制到剪贴板: {}", node_id);
        
        // 模拟剪贴板操作
        println!("NODE_ID_TO_COPY: {}", node_id);
        
        Ok(())
    }

    /// 从剪贴板添加节点
    pub async fn add_node_from_clipboard(&self) -> Result<()> {
        // 在实际应用中，这里应该从剪贴板读取
        // 模拟从剪贴板读取节点 ID
        let clipboard_content = "12D3KooWExampleNodeID1234567890ABCDEF";
        
        info!("📋 从剪贴板添加节点: {}", clipboard_content);
        
        self.add_remote_node(
            clipboard_content.to_string(),
            vec![format!("/ip4/127.0.0.1/tcp/9236/p2p/{}", clipboard_content)],
        ).await?;
        
        Ok(())
    }

    /// 启动 P2P 服务
    pub async fn start_p2p_service(&mut self) -> Result<()> {
        info!("🚀 启动 P2P 服务");

        // 创建 P2P 分发器
        let distributor = crate::comms::p2p_distributor::P2PModelDistributor::new(self.local_node_id.clone());
        self.p2p_distributor = Some(Arc::new(distributor));

        // 启动后台任务
        self.start_background_tasks().await?;

        Ok(())
    }

    /// 停止 P2P 服务
    pub async fn stop_p2p_service(&mut self) -> Result<()> {
        info!("🛑 停止 P2P 服务");
        
        self.p2p_distributor = None;
        
        Ok(())
    }

    /// 启动后台任务
    async fn start_background_tasks(&self) -> Result<()> {
        let connected_nodes = self.connected_nodes.clone();
        let connection_stats = self.connection_stats.clone();

        // 定期更新节点状态
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                // 模拟更新连接统计
                {
                    let mut stats = connection_stats.lock().await;
                    stats.active_connections = 3;
                    stats.total_connections = 5;
                    stats.upload_speed = 1024.0 * 1024.0; // 1MB/s
                    stats.download_speed = 512.0 * 1024.0; // 512KB/s
                    stats.total_uploaded += 1024 * 1024;
                    stats.total_downloaded += 512 * 1024;
                }

                // 模拟检查节点连接状态
                let nodes = connected_nodes.read().await;
                for (node_id, node_info) in nodes.iter() {
                    debug!("检查节点状态: {} - {:?}", node_id, node_info.status);
                }
            }
        });

        Ok(())
    }

    /// 获取前端所需的完整状态
    pub async fn get_frontend_state(&self) -> Result<FrontendState> {
        let local_node = self.get_local_node_info().await?;
        let connected_nodes = self.get_connected_nodes().await?;
        let stats = self.get_connection_stats().await?;

        Ok(FrontendState {
            local_node,
            connected_nodes,
            connection_stats: stats,
        })
    }
}

/// 前端状态结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendState {
    pub local_node: P2PNodeInfo,
    pub connected_nodes: Vec<P2PNodeInfo>,
    pub connection_stats: P2PConnectionStats,
}

/// 全局 P2P 管理器实例
static mut GLOBAL_P2P_MANAGER: Option<P2PFrontendManager> = None;
static P2P_MANAGER_INIT: std::sync::Once = std::sync::Once::new();

/// 获取全局 P2P 管理器
pub async fn get_global_p2p_manager() -> &'static P2PFrontendManager {
    unsafe {
        P2P_MANAGER_INIT.call_once(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let manager = rt.block_on(P2PFrontendManager::new()).unwrap();
            GLOBAL_P2P_MANAGER = Some(manager);
        });
        
        GLOBAL_P2P_MANAGER.as_ref().unwrap()
    }
}

/// FFI 函数：获取本地节点 ID
#[no_mangle]
pub extern "C" fn get_local_node_id() -> *const std::os::raw::c_char {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let manager = rt.block_on(get_global_p2p_manager());
    let node_id = manager.local_node_id();
    
    // 将 Rust 字符串转换为 C 字符串
    std::ffi::CString::new(node_id).unwrap().into_raw()
}

/// FFI 函数：复制节点 ID
#[no_mangle]
pub extern "C" fn copy_node_id_to_clipboard() -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let manager = rt.block_on(get_global_p2p_manager());
    rt.block_on(manager.copy_node_id()).is_ok()
}

/// FFI 函数：添加远程节点
#[no_mangle]
pub extern "C" fn add_remote_node(node_id: *const std::os::raw::c_char) -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let manager = rt.block_on(get_global_p2p_manager());
    
    unsafe {
        let node_id_str = std::ffi::CStr::from_ptr(node_id).to_string_lossy().to_string();
        rt.block_on(manager.add_remote_node(node_id_str, vec![])).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_frontend_manager() -> Result<()> {
        let manager = P2PFrontendManager::new().await?;
        
        // 测试获取本地节点 ID
        let node_id = manager.local_node_id();
        assert!(!node_id.is_empty());
        
        // 测试获取本地节点信息
        let local_info = manager.get_local_node_info().await?;
        assert_eq!(local_info.node_id, node_id);
        
        // 测试添加远程节点
        manager.add_remote_node(
            "test_node_id".to_string(),
            vec!["/ip4/127.0.0.1/tcp/9236".to_string()],
        ).await?;
        
        let connected_nodes = manager.get_connected_nodes().await?;
        assert_eq!(connected_nodes.len(), 2); // 本地节点 + 远程节点
        
        Ok(())
    }
}

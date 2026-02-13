/*
 * P2P 前端启动器
 * 在前端桌面应用启动时自动初始化 P2P 服务
 */

#![allow(static_mut_refs)]

use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error};

use crate::comms::frontend::manager::P2PFrontendManager;

/// P2P 前端启动器
pub struct P2PFrontendStarter {
    manager: Arc<Mutex<Option<P2PFrontendManager>>>,
    is_initialized: std::sync::atomic::AtomicBool,
}

impl P2PFrontendStarter {
    /// 创建新的启动器
    pub fn new() -> Self {
        Self {
            manager: Arc::new(Mutex::new(None)),
            is_initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 初始化 P2P 服务
    pub async fn initialize(&self) -> Result<()> {
        if self.is_initialized.load(std::sync::atomic::Ordering::Relaxed) {
            warn!("P2P 服务已经初始化");
            return Ok(());
        }

        info!("🚀 启动 P2P 前端服务");

        // 创建 P2P 管理器
        let manager = P2PFrontendManager::new().await?;
        
        // 启动 P2P 服务
        let mut manager_mut = manager;
        manager_mut.start_p2p_service().await?;

        // 存储管理器
        {
            let mut guard = self.manager.lock().await;
            *guard = Some(manager_mut);
        }

        // 标记为已初始化
        self.is_initialized.store(true, std::sync::atomic::Ordering::Relaxed);

        info!("✅ P2P 前端服务启动成功");

        // 启动后台任务
        self.start_background_tasks().await?;

        Ok(())
    }

    /// 获取本地节点 ID
    pub async fn get_local_node_id(&self) -> Result<String> {
        let guard = self.manager.lock().await;
        if let Some(ref manager) = *guard {
            Ok(manager.local_node_id().to_string())
        } else {
            Err(anyhow!("P2P 管理器未初始化"))
        }
    }

    /// 获取前端状态
    pub async fn get_frontend_state(&self) -> Result<crate::comms::p2p_frontend_manager::FrontendState> {
        let guard = self.manager.lock().await;
        if let Some(ref manager) = *guard {
            manager.get_frontend_state().await
        } else {
            Err(anyhow!("P2P 管理器未初始化"))
        }
    }

    /// 添加远程节点
    pub async fn add_remote_node(&self, node_id: String, addresses: Vec<String>) -> Result<()> {
        let guard = self.manager.lock().await;
        if let Some(ref manager) = *guard {
            manager.add_remote_node(node_id, addresses).await
        } else {
            Err(anyhow!("P2P 管理器未初始化"))
        }
    }

    /// 移除节点
    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        let guard = self.manager.lock().await;
        if let Some(ref manager) = *guard {
            manager.remove_node(node_id).await
        } else {
            Err(anyhow!("P2P 管理器未初始化"))
        }
    }

    /// 复制节点 ID
    pub async fn copy_node_id(&self) -> Result<()> {
        let guard = self.manager.lock().await;
        if let Some(ref manager) = *guard {
            manager.copy_node_id().await
        } else {
            Err(anyhow!("P2P 管理器未初始化"))
        }
    }

    /// 从剪贴板添加节点
    pub async fn add_node_from_clipboard(&self) -> Result<()> {
        let guard = self.manager.lock().await;
        if let Some(ref manager) = *guard {
            manager.add_node_from_clipboard().await
        } else {
            Err(anyhow!("P2P 管理器未初始化"))
        }
    }

    /// 启动后台任务
    async fn start_background_tasks(&self) -> Result<()> {
        info!("🔄 启动 P2P 后台任务");

        // 模拟添加一些引导节点
        let manager = self.manager.clone();
        tokio::spawn(async move {
            // 等待一段时间后添加引导节点
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            let guard = manager.lock().await;
            if let Some(ref manager) = *guard {
                // 添加模拟的引导节点
                let bootstrap_nodes = vec![
                    ("12D3KooWBootstrapNode1".to_string(), vec!["/ip4/104.131.131.82/tcp/4001/p2p/12D3KooWBootstrapNode1".to_string()]),
                    ("12D3KooWBootstrapNode2".to_string(), vec!["/ip4/104.131.131.83/tcp/4001/p2p/12D3KooWBootstrapNode2".to_string()]),
                ];
                
                for (node_id, addresses) in bootstrap_nodes {
                    if let Err(e) = manager.add_remote_node(node_id.clone(), addresses).await {
                        error!("添加引导节点失败 {}: {}", node_id, e);
                    }
                }
                
                info!("🌐 引导节点添加完成");
            }
        });

        // 定期健康检查
        let manager = self.manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let guard = manager.lock().await;
                if let Some(ref manager) = *guard {
                    // 检查连接状态
                    if let Ok(nodes) = manager.get_connected_nodes().await {
                        let online_count = nodes.iter().filter(|n| {
                            matches!(n.status, crate::comms::p2p_frontend_manager::NodeStatus::Online)
                        }).count();
                        
                        info!("📊 连接状态: {}/{} 节点在线", online_count, nodes.len());
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止 P2P 服务
    pub async fn shutdown(&self) -> Result<()> {
        if !self.is_initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        info!("🛑 停止 P2P 前端服务");

        {
            let mut guard = self.manager.lock().await;
            if let Some(ref mut manager) = *guard {
                manager.stop_p2p_service().await?;
            }
        }

        self.is_initialized.store(false, std::sync::atomic::Ordering::Relaxed);

        info!("✅ P2P 前端服务已停止");
        Ok(())
    }
}

impl Drop for P2PFrontendStarter {
    fn drop(&mut self) {
        if self.is_initialized.load(std::sync::atomic::Ordering::Relaxed) {
            warn!("P2PFrontendStarter 被销毁但服务仍在运行，建议手动调用 shutdown()");
        }
    }
}

/// 全局 P2P 启动器实例
static mut GLOBAL_P2P_STARTER: Option<P2PFrontendStarter> = None;
static P2P_STARTER_INIT: std::sync::Once = std::sync::Once::new();

/// 获取全局 P2P 启动器
pub async fn get_global_p2p_starter() -> &'static P2PFrontendStarter {
    unsafe {
        P2P_STARTER_INIT.call_once(|| {
            let starter = P2PFrontendStarter::new();
            GLOBAL_P2P_STARTER = Some(starter);
        });
        
        GLOBAL_P2P_STARTER.as_ref().unwrap()
    }
}

/// 自动初始化 P2P 服务（在应用启动时调用）
pub async fn auto_initialize_p2p_service() -> Result<()> {
    let starter = get_global_p2p_starter().await;
    starter.initialize().await
}

/// FFI 函数：获取本地节点 ID（供前端调用）
#[no_mangle]
pub extern "C" fn p2p_get_local_node_id() -> *mut std::os::raw::c_char {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let starter = rt.block_on(get_global_p2p_starter());
    
    match rt.block_on(starter.get_local_node_id()) {
        Ok(node_id) => {
            std::ffi::CString::new(node_id).unwrap().into_raw()
        }
        Err(_) => {
            std::ptr::null_mut()
        }
    }
}

/// FFI 函数：复制节点 ID（供前端调用）
#[no_mangle]
pub extern "C" fn p2p_copy_node_id() -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let starter = rt.block_on(get_global_p2p_starter());
    rt.block_on(starter.copy_node_id()).is_ok()
}

/// FFI 函数：添加远程节点（供前端调用）
#[no_mangle]
pub extern "C" fn p2p_add_remote_node(node_id: *const std::os::raw::c_char) -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let starter = rt.block_on(get_global_p2p_starter());
    
    unsafe {
        let node_id_str = std::ffi::CStr::from_ptr(node_id).to_string_lossy().to_string();
        rt.block_on(starter.add_remote_node(node_id_str, vec![])).is_ok()
    }
}

/// FFI 函数：从剪贴板添加节点（供前端调用）
#[no_mangle]
pub extern "C" fn p2p_add_node_from_clipboard() -> bool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let starter = rt.block_on(get_global_p2p_starter());
    rt.block_on(starter.add_node_from_clipboard()).is_ok()
}

/// FFI 函数：获取前端状态 JSON（供前端调用）
#[no_mangle]
pub extern "C" fn p2p_get_frontend_state() -> *mut std::os::raw::c_char {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let starter = rt.block_on(get_global_p2p_starter());
    
    match rt.block_on(starter.get_frontend_state()) {
        Ok(state) => {
            match serde_json::to_string(&state) {
                Ok(json) => {
                    std::ffi::CString::new(json).unwrap().into_raw()
                }
                Err(_) => {
                    std::ptr::null_mut()
                }
            }
        }
        Err(_) => {
            std::ptr::null_mut()
        }
    }
}

/// FFI 函数：释放 C 字符串内存
#[no_mangle]
pub extern "C" fn p2p_free_string(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_frontend_starter() -> Result<()> {
        let starter = P2PFrontendStarter::new();
        
        // 测试初始化
        starter.initialize().await?;
        
        // 测试获取本地节点 ID
        let node_id = starter.get_local_node_id().await?;
        assert!(!node_id.is_empty());
        
        // 测试获取前端状态
        let state = starter.get_frontend_state().await?;
        assert_eq!(state.local_node.node_id, node_id);
        
        // 测试添加远程节点
        starter.add_remote_node(
            "test_node_id".to_string(),
            vec!["/ip4/127.0.0.1/tcp/9236".to_string()],
        ).await?;
        
        // 测试关闭
        starter.shutdown().await?;
        
        Ok(())
    }
}

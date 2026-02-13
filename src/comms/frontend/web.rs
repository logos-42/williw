/*
 * P2P 前端集成模块
 * 提供 WebAssembly 接口用于前端与 Rust 后端交互
 */

#![allow(static_mut_refs)]

use anyhow::Result;
use std::sync::Arc;

use crate::comms::frontend::manager::{P2PFrontendManager, P2PNodeInfo, P2PConnectionStats};

/// P2P 前端控制器
pub struct P2PWebController {
    manager: Arc<P2PFrontendManager>,
}

impl P2PWebController {
    /// 创建新的 Web 控制器
    pub async fn new() -> Result<Self> {
        let manager = Arc::new(P2PFrontendManager::new().await?);

        Ok(Self { manager })
    }

    /// 初始化前端界面
    pub async fn initialize(&self) -> Result<()> {
        println!("初始化 P2P Web 控制器");

        // 获取本地节点信息
        let local_node = self.manager.get_local_node_info().await?;
        
        // 更新前端显示（模拟）
        println!("本地节点 ID: {}", local_node.node_id);

        Ok(())
    }

    /// 获取本地节点信息
    pub async fn get_local_node_info(&self) -> Result<P2PNodeInfo> {
        self.manager.get_local_node_info().await
    }

    /// 复制节点 ID
    pub async fn copy_node_id(&self) -> Result<()> {
        self.manager.copy_node_id().await
    }

    /// 添加远程节点
    pub async fn add_remote_node(&self, node_id: String, addresses: Vec<String>) -> Result<()> {
        self.manager.add_remote_node(node_id, addresses).await
    }

    /// 移除节点
    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        self.manager.remove_node(node_id).await
    }

    /// 获取连接统计
    pub async fn get_connection_stats(&self) -> Result<P2PConnectionStats> {
        self.manager.get_connection_stats().await
    }
}

/// WebAssembly 接口（占位符）
pub struct P2PWebInterface {
    controller: Option<P2PWebController>,
}

impl P2PWebInterface {
    /// 创建新的 P2P Web 接口
    pub async fn new() -> Result<P2PWebInterface, String> {
        println!("创建 P2P Web 接口");
        
        match P2PWebController::new().await {
            Ok(controller) => Ok(P2PWebInterface {
                controller: Some(controller),
            }),
            Err(e) => Err(format!("创建 P2P Web 控制器失败: {}", e)),
        }
    }

    /// 初始化接口
    pub async fn initialize(&mut self) -> Result<(), String> {
        if let Some(ref controller) = self.controller {
            controller.initialize().await
                .map_err(|e| format!("初始化失败: {}", e))?;
        }
        Ok(())
    }

    /// 获取本地节点 ID
    pub async fn get_local_node_id(&self) -> Result<String, String> {
        if let Some(ref controller) = self.controller {
            let node_info = controller.get_local_node_info().await
                .map_err(|e| format!("获取本地节点信息失败: {}", e))?;
            Ok(node_info.node_id)
        } else {
            Err("控制器未初始化".to_string())
        }
    }

    /// 复制节点 ID
    pub async fn copy_node_id(&self) -> Result<(), String> {
        if let Some(ref controller) = self.controller {
            controller.copy_node_id().await
                .map_err(|e| format!("复制节点 ID 失败: {}", e))?;
        }
        Ok(())
    }

    /// 添加远程节点
    pub async fn add_remote_node(&self, node_id: &str, addresses: Vec<String>) -> Result<(), String> {
        if let Some(ref controller) = self.controller {
            controller.add_remote_node(node_id.to_string(), addresses).await
                .map_err(|e| format!("添加远程节点失败: {}", e))?;
        }
        Ok(())
    }

    /// 移除节点
    pub async fn remove_node(&self, node_id: &str) -> Result<(), String> {
        if let Some(ref controller) = self.controller {
            controller.remove_node(node_id).await
                .map_err(|e| format!("移除节点失败: {}", e))?;
        }
        Ok(())
    }

    /// 获取连接统计
    pub async fn get_connection_stats(&self) -> Result<P2PConnectionStats, String> {
        if let Some(ref controller) = self.controller {
            let stats = controller.get_connection_stats().await
                .map_err(|e| format!("获取连接统计失败: {}", e))?;
            Ok(stats)
        } else {
            Err("控制器未初始化".to_string())
        }
    }

    /// 启动 P2P 服务
    pub async fn start_service(&mut self) -> Result<(), String> {
        println!("启动 P2P 服务");
        Ok(())
    }

    /// 停止 P2P 服务
    pub async fn stop_service(&mut self) -> Result<(), String> {
        println!("停止 P2P 服务");
        Ok(())
    }
}

/// 全局实例（占位符）
static mut GLOBAL_WEB_INTERFACE: Option<P2PWebInterface> = None;
static WEB_INTERFACE_INIT: std::sync::Once = std::sync::Once::new();

/// 获取全局 Web 接口
pub fn get_global_p2p_interface() -> *const P2PWebInterface {
    unsafe {
        WEB_INTERFACE_INIT.call_once(|| {
            println!("初始化全局 P2P Web 接口");
        });
        
        GLOBAL_WEB_INTERFACE.as_ref().map(|interface| interface as *const P2PWebInterface).unwrap_or(std::ptr::null())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_web_controller() {
        // 这里需要模拟 Web 环境，在实际测试中需要 wasm-bindgen-test
        // 暂时跳过
    }
}

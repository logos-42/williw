/**
 * P2P传输事件系统
 * 提供统一的事件定义和处理机制
 */

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use anyhow::Result;

/// 传输事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferEvent {
    /// 传输开始
    TransferStarted {
        transfer_id: String,
        file_name: String,
        peer_id: String,
    },
    /// 传输进度更新
    ProgressUpdate {
        transfer_id: String,
        progress: f32,
        speed_bps: u64,
    },
    /// 传输完成
    TransferCompleted {
        transfer_id: String,
        file_size: u64,
        duration_secs: u64,
    },
    /// 传输失败
    TransferFailed {
        transfer_id: String,
        error: String,
    },
    /// 节点连接状态变化
    PeerConnectionChanged {
        peer_id: String,
        connected: bool,
    },
}

/// 事件管理器
pub struct EventManager {
    event_tx: mpsc::Sender<TransferEvent>,
    event_rx: mpsc::Receiver<TransferEvent>,
    listeners: Arc<RwLock<Vec<mpsc::Sender<TransferEvent>>>>,
}

impl EventManager {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel::<TransferEvent>(1000);
        let listeners = Arc::new(RwLock::new(Vec::new()));
        
        Self {
            event_tx,
            event_rx,
            listeners,
        }
    }
    
    /// 发送事件
    pub async fn send_event(&self, event: TransferEvent) -> Result<()> {
        // 发送到主通道
        let _ = self.event_tx.send(event.clone()).await;
        
        // 发送到所有监听器
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            let _ = listener.send(event.clone()).await;
        }
        
        Ok(())
    }
    
    /// 获取事件接收器（注意：Receiver不能clone，每次调用创建新的）
    pub fn get_receiver(&self) -> mpsc::Receiver<TransferEvent> {
        let (tx, rx) = mpsc::channel::<TransferEvent>(1000);
        {
            let mut listeners = self.listeners.blocking_write();
            listeners.push(tx);
        }
        rx
    }
    
    /// 获取主事件接收器
    pub fn get_main_receiver(&self) -> mpsc::Receiver<TransferEvent> {
        // 由于Receiver不能clone，这里创建新的监听器
        self.add_listener_blocking()
    }
    
    /// 添加监听器（阻塞版本）
    fn add_listener_blocking(&self) -> mpsc::Receiver<TransferEvent> {
        let (tx, rx) = mpsc::channel::<TransferEvent>(1000);
        let mut listeners = self.listeners.blocking_write();
        listeners.push(tx);
        rx
    }
    
    /// 添加事件监听器
    pub async fn add_listener(&self) -> mpsc::Receiver<TransferEvent> {
        let (tx, rx) = mpsc::channel::<TransferEvent>(100);
        let mut listeners = self.listeners.write().await;
        listeners.push(tx);
        rx
    }
    
    /// 移除事件监听器（通过关闭通道）
    pub async fn remove_listener(&self, _receiver: mpsc::Receiver<TransferEvent>) {
        // 实际实现中可以通过更复杂的方式管理监听器
        // 这里简化处理，当接收器被drop时会自动关闭
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局事件管理器实例
static mut GLOBAL_EVENT_MANAGER: Option<Arc<EventManager>> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// 获取全局事件管理器
pub fn get_global_event_manager() -> Arc<EventManager> {
    unsafe {
        INIT.call_once(|| {
            GLOBAL_EVENT_MANAGER = Some(Arc::new(EventManager::new()));
        });
        GLOBAL_EVENT_MANAGER.as_ref().unwrap().clone()
    }
}

/// 便捷函数：发送全局事件
pub async fn send_global_event(event: TransferEvent) -> Result<()> {
    get_global_event_manager().send_event(event).await
}

/// 便捷函数：获取全局事件接收器
pub fn get_global_receiver() -> mpsc::Receiver<TransferEvent> {
    get_global_event_manager().get_receiver()
}

/// 便捷函数：添加全局事件监听器
pub async fn add_global_listener() -> mpsc::Receiver<TransferEvent> {
    get_global_event_manager().add_listener().await
}

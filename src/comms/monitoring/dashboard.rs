/**
 * P2P传输监控仪表板
 * 提供实时监控和管理功能
 */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};
use tracing::{info, error};

use crate::comms::p2p::{TransferEvent, P2PModelDistributor, get_global_receiver};

/// 监控统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStats {
    /// 总传输次数
    pub total_transfers: u64,
    /// 成功传输次数
    pub successful_transfers: u64,
    /// 失败传输次数
    pub failed_transfers: u64,
    /// 总传输字节数
    pub total_bytes_transferred: u64,
    /// 平均传输速度 (bytes/sec)
    pub average_speed: f64,
    /// 活跃连接数
    pub active_connections: usize,
    /// 当前传输队列长度
    pub queue_length: usize,
    /// 系统运行时间
    pub uptime_seconds: u64,
}

impl Default for MonitoringStats {
    fn default() -> Self {
        Self {
            total_transfers: 0,
            successful_transfers: 0,
            failed_transfers: 0,
            total_bytes_transferred: 0,
            average_speed: 0.0,
            active_connections: 0,
            queue_length: 0,
            uptime_seconds: 0,
        }
    }
}

/// 传输历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferHistory {
    pub transfer_id: String,
    pub file_name: String,
    pub peer_id: String,
    pub file_size: u64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: TransferStatus,
    pub progress: f32,
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    Cancelled,
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub connected: bool,
    pub connection_time: Option<DateTime<Utc>>,
    pub total_transfers: u64,
    pub last_activity: Option<DateTime<Utc>>,
}

/// 监控仪表板
pub struct MonitoringDashboard {
    distributor: Arc<P2PModelDistributor>,
    stats: Arc<RwLock<MonitoringStats>>,
    transfer_history: Arc<RwLock<HashMap<String, TransferHistory>>>,
    peer_info: Arc<RwLock<HashMap<String, PeerInfo>>>,
    event_rx: mpsc::Receiver<TransferEvent>,
    start_time: DateTime<Utc>,
}

impl MonitoringDashboard {
    /// 创建新的监控仪表板
    pub async fn new(distributor: Arc<P2PModelDistributor>) -> Result<Self> {
        info!("初始化监控仪表板");
        
        let stats = Arc::new(RwLock::new(MonitoringStats::default()));
        let transfer_history = Arc::new(RwLock::new(HashMap::new()));
        let peer_info = Arc::new(RwLock::new(HashMap::new()));
        
        // 获取全局事件接收器（不能clone，需要重新获取）
        let event_rx = get_global_receiver();
        
        let dashboard = Self {
            distributor,
            stats,
            transfer_history,
            peer_info,
            event_rx,
            start_time: Utc::now(),
        };
        
        // 启动事件处理循环
        dashboard.start_event_processing().await?;
        
        info!("✅ 监控仪表板初始化完成");
        Ok(dashboard)
    }
    
    /// 启动事件处理循环
    async fn start_event_processing(&self) -> Result<()> {
        let stats = self.stats.clone();
        let history = self.transfer_history.clone();
        let peer_info = self.peer_info.clone();
        
        // 创建新的事件接收器
        let mut event_rx = get_global_receiver();
        
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    TransferEvent::TransferStarted { transfer_id, file_name, peer_id } => {
                        let _transfer_id_clone = transfer_id.clone();
                        let file_name_clone = file_name.clone();
                        let peer_id_clone = peer_id.clone();

                        // 创建传输记录
                        {
                            let mut h = history.write().await;
                            h.insert(_transfer_id_clone.clone(), TransferHistory {
                                transfer_id: _transfer_id_clone.clone(),
                                file_name: file_name_clone,
                                peer_id: peer_id_clone.clone(),
                                file_size: 0,
                                start_time: Utc::now(),
                                end_time: None,
                                status: TransferStatus::InProgress,
                                progress: 0.0,
                                speed_bps: 0,
                            });
                        }

                        // 更新节点信息
                        {
                            let mut p = peer_info.write().await;
                            let peer_id_for_entry = peer_id_clone.clone();
                            let peer = p.entry(peer_id_for_entry).or_insert_with(|| PeerInfo {
                                peer_id: peer_id_clone.clone(),
                                address: "unknown".to_string(),
                                connected: true,
                                connection_time: Some(Utc::now()),
                                total_transfers: 0,
                                last_activity: Some(Utc::now()),
                            });
                            peer.total_transfers += 1;
                            peer.last_activity = Some(Utc::now());
                        }

                        info!("📊 传输开始: {}", _transfer_id_clone);
                    }
                    
                    TransferEvent::ProgressUpdate { transfer_id, progress, speed_bps } => {
                        // 更新历史记录
                        {
                            let mut h = history.write().await;
                            if let Some(record) = h.get_mut(&transfer_id) {
                                record.progress = progress;
                                record.speed_bps = speed_bps;
                            }
                        }
                    }
                    
                    TransferEvent::TransferCompleted { transfer_id, file_size, duration_secs } => {
                        // 更新统计
                        {
                            let transfer_id_clone = transfer_id.clone();
                            let mut s = stats.write().await;
                            s.successful_transfers += 1;
                        }
                        
                        // 更新历史记录
                        {
                            let mut h = history.write().await;
                            if let Some(record) = h.get_mut(&transfer_id) {
                                record.status = TransferStatus::Completed;
                                record.end_time = Some(Utc::now());
                                record.file_size = file_size;
                            }
                        }
                        
                        // 更新统计
                        {
                            let mut s = stats.write().await;
                            s.total_bytes_transferred += file_size;
                            
                            // 更新平均速度
                            if s.successful_transfers > 0 {
                                s.average_speed = s.total_bytes_transferred as f64 / 
                                    (s.successful_transfers as f64 * duration_secs as f64);
                            }
                        }
                        
                        // 更新历史记录
                        {
                            let mut h = history.write().await;
                            if let Some(record) = h.get_mut(&transfer_id) {
                                record.status = TransferStatus::Completed;
                                record.end_time = Some(Utc::now());
                                record.file_size = file_size;
                            }
                        }
                        
                        info!("📊 传输完成: {} ({} bytes, {} sec)", transfer_id, file_size, duration_secs);
                    }
                    
                    TransferEvent::TransferFailed { transfer_id, error } => {
                        // 更新统计
                        {
                            let mut s = stats.write().await;
                            s.failed_transfers += 1;
                        }
                        
                        // 更新历史记录
                        {
                            let mut h = history.write().await;
                            if let Some(record) = h.get_mut(&transfer_id) {
                                record.status = TransferStatus::Failed(error.clone());
                                record.end_time = Some(Utc::now());
                            }
                        }
                        
                        error!("📊 传输失败: {} - {}", transfer_id, error);
                    }
                    
                    TransferEvent::PeerConnectionChanged { peer_id, connected } => {
                        // 更新节点信息
                        {
                            let mut p = peer_info.write().await;
                            let peer = p.entry(peer_id.clone()).or_insert_with(|| PeerInfo {
                                peer_id: peer_id.clone(),
                                address: "unknown".to_string(),
                                connected,
                                connection_time: if connected { Some(Utc::now()) } else { None },
                                total_transfers: 0,
                                last_activity: Some(Utc::now()),
                            });
                            peer.connected = connected;
                            peer.last_activity = Some(Utc::now());
                            
                            if !connected {
                                peer.connection_time = None;
                            }
                        }
                        
                        // 更新活跃连接数
                        {
                            let mut s = stats.write().await;
                            let peer_info_guard = peer_info.read().await;
                            s.active_connections = peer_info_guard.values()
                                .filter(|p| p.connected).count();
                        }
                        
                        info!("📊 节点连接状态变化: {} -> {}", peer_id, connected);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// 获取当前统计信息
    pub async fn get_stats(&self) -> MonitoringStats {
        let mut stats = self.stats.write().await;
        
        // 更新运行时间
        stats.uptime_seconds = (Utc::now() - self.start_time).num_seconds() as u64;
        
        // 更新队列长度
        stats.queue_length = self.distributor.get_active_transfers().await.len();
        
        stats.clone()
    }
    
    /// 获取传输历史
    pub async fn get_transfer_history(&self) -> Vec<TransferHistory> {
        let history = self.transfer_history.read().await;
        history.values().cloned().collect()
    }
    
    /// 获取节点信息
    pub async fn get_peer_info(&self) -> Vec<PeerInfo> {
        let peer_info = self.peer_info.read().await;
        peer_info.values().cloned().collect()
    }
    
    /// 获取活跃传输列表
    pub async fn get_active_transfers(&self) -> Vec<TransferHistory> {
        let history = self.transfer_history.read().await;
        history.values()
            .filter(|t| matches!(t.status, TransferStatus::InProgress))
            .cloned()
            .collect()
    }
    
    /// 生成监控报告
    pub async fn generate_report(&self) -> MonitoringReport {
        let stats = self.get_stats().await;
        let active_transfers = self.get_active_transfers().await;
        let peer_info = self.get_peer_info().await;
        
        MonitoringReport {
            timestamp: Utc::now(),
            stats,
            active_transfers,
            peer_info,
        }
    }
    
    /// 导出数据为JSON
    pub async fn export_data(&self) -> Result<String> {
        let report = self.generate_report().await;
        Ok(serde_json::to_string_pretty(&report)?)
    }
    
    /// 清理历史记录
    pub async fn cleanup_history(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        
        let mut history = self.transfer_history.write().await;
        history.retain(|_, record| {
            record.start_time > cutoff || 
            (record.end_time.is_none() || record.end_time.unwrap() > cutoff)
        });
        
        info!("📊 已清理 {} 小时前的历史记录", max_age_hours);
    }
}

/// 监控报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringReport {
    pub timestamp: DateTime<Utc>,
    pub stats: MonitoringStats,
    pub active_transfers: Vec<TransferHistory>,
    pub peer_info: Vec<PeerInfo>,
}

/// Web API处理器
pub struct WebApiHandler {
    dashboard: Arc<MonitoringDashboard>,
}

impl WebApiHandler {
    pub fn new(dashboard: Arc<MonitoringDashboard>) -> Self {
        Self { dashboard }
    }
    
    /// 获取统计信息API
    pub async fn get_stats(&self) -> Result<MonitoringStats> {
        Ok(self.dashboard.get_stats().await)
    }
    
    /// 获取传输历史API
    pub async fn get_history(&self) -> Result<Vec<TransferHistory>> {
        Ok(self.dashboard.get_transfer_history().await)
    }
    
    /// 获取节点信息API
    pub async fn get_peers(&self) -> Result<Vec<PeerInfo>> {
        Ok(self.dashboard.get_peer_info().await)
    }
    
    /// 获取监控报告API
    pub async fn get_report(&self) -> Result<MonitoringReport> {
        Ok(self.dashboard.generate_report().await)
    }
    
    /// 导出数据API
    pub async fn export_data(&self) -> Result<String> {
        self.dashboard.export_data().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_monitoring_stats() {
        let mut stats = MonitoringStats::default();
        stats.total_transfers = 10;
        stats.successful_transfers = 8;
        stats.failed_transfers = 2;
        
        assert_eq!(stats.total_transfers, 10);
        assert_eq!(stats.successful_transfers, 8);
        assert_eq!(stats.failed_transfers, 2);
    }
    
    #[tokio::test]
    async fn test_transfer_history() {
        let history = TransferHistory {
            transfer_id: "test".to_string(),
            file_name: "test.txt".to_string(),
            peer_id: "peer1".to_string(),
            file_size: 1024,
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            status: TransferStatus::Completed,
            progress: 100.0,
            speed_bps: 1024,
        };
        
        assert_eq!(history.transfer_id, "test");
        assert_eq!(history.file_name, "test.txt");
        assert!(matches!(history.status, TransferStatus::Completed));
    }
}

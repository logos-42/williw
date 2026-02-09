//! 统计数据管理模块
//! 
//! 管理训练统计数据和导出功能

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::Path;
use std::fs;
use anyhow::Result;

/// 训练统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStats {
    pub tick_count: u64,
    pub start_time: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connected_peers: u64,
    pub training_accuracy: f64,
    pub training_loss: f64,
    pub samples_processed: u64,
    pub custom_metrics: HashMap<String, f64>,
}

impl Default for TrainingStats {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            tick_count: 0,
            start_time: now,
            last_update: now,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connected_peers: 0,
            training_accuracy: 0.0,
            training_loss: 1.0,
            samples_processed: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

/// 统计数据管理器
#[derive(Debug)]
pub struct TrainingStatsManager {
    stats: TrainingStats,
}

impl TrainingStatsManager {
    /// 创建新的统计管理器
    pub fn new() -> Self {
        Self {
            stats: TrainingStats::default(),
        }
    }
    
    /// 创建带模型信息的统计管理器
    pub fn new_with_model(_model_hash: String, _model_version: u32) -> Self {
        Self::new()
    }
    
    /// 增加tick计数
    pub fn increment_tick(&mut self) {
        self.stats.tick_count += 1;
        self.stats.last_update = Utc::now();
    }
    
    /// 更新消息统计
    pub fn update_message_stats(&mut self, sent: u64, received: u64, bytes_sent: u64, bytes_received: u64) {
        self.stats.messages_sent += sent;
        self.stats.messages_received += received;
        self.stats.bytes_sent += bytes_sent;
        self.stats.bytes_received += bytes_received;
        self.stats.last_update = Utc::now();
    }
    
    /// 更新连接节点数
    pub fn update_connected_peers(&mut self, count: u64) {
        self.stats.connected_peers = count;
        self.stats.last_update = Utc::now();
    }
    
    /// 更新训练指标
    pub fn update_training_metrics(&mut self, accuracy: f64, loss: f64, samples: u64) {
        self.stats.training_accuracy = accuracy;
        self.stats.training_loss = loss;
        self.stats.samples_processed = samples;
        self.stats.last_update = Utc::now();
    }
    
    /// 添加自定义指标
    pub fn add_custom_metric(&mut self, name: String, value: f64) {
        self.stats.custom_metrics.insert(name, value);
        self.stats.last_update = Utc::now();
    }
    
    /// 获取统计数据引用
    pub fn get_stats(&self) -> &TrainingStats {
        &self.stats
    }
    
    /// 导出为JSON字符串
    pub fn export_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.stats)?)
    }
    
    /// 导出到文件
    pub fn export_json_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = self.export_json()?;
        fs::write(path, json)?;
        Ok(())
    }
    
    /// 重置统计数据
    pub fn reset(&mut self) {
        self.stats = TrainingStats::default();
    }
    
    /// 获取运行时间
    pub fn get_runtime(&self) -> chrono::Duration {
        Utc::now() - self.stats.start_time
    }
    
    /// 获取平均消息速率（消息/秒）
    pub fn get_message_rate(&self) -> f64 {
        let runtime_secs = self.get_runtime().num_seconds() as f64;
        if runtime_secs > 0.0 {
            (self.stats.messages_sent + self.stats.messages_received) as f64 / runtime_secs
        } else {
            0.0
        }
    }
    
    /// 获取网络吞吐量（字节/秒）
    pub fn get_throughput(&self) -> f64 {
        let runtime_secs = self.get_runtime().num_seconds() as f64;
        if runtime_secs > 0.0 {
            (self.stats.bytes_sent + self.stats.bytes_received) as f64 / runtime_secs
        } else {
            0.0
        }
    }
}

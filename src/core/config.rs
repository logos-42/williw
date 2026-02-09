//! 统一配置系统
//!
//! 提供简洁、统一的配置管理，支持多种配置源和动态更新。

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::network::NetworkConfig;
use crate::device::DeviceConfig;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// 网络配置
    pub network: NetworkConfig,
    /// 设备配置
    pub device: DeviceConfig,
    /// 训练配置
    pub training: TrainingConfig,
    /// 推理配置
    pub inference: InferenceConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

/// 训练配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// 模型路径
    pub model_path: String,
    /// 数据路径
    pub data_path: String,
    /// 批量大小
    pub batch_size: usize,
    /// 学习率
    pub learning_rate: f64,
    /// 训练轮数
    pub epochs: u32,
    /// 是否启用分布式训练
    pub enable_distributed: bool,
}

/// 推理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// 模型路径
    pub model_path: String,
    /// 批量大小
    pub batch_size: usize,
    /// 是否启用 GPU 加速
    pub enable_gpu: bool,
    /// 推理超时（秒）
    pub timeout_secs: u64,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: String,
    /// 日志文件路径
    pub file_path: Option<String>,
    /// 是否启用控制台输出
    pub enable_console: bool,
    /// 是否启用结构化日志
    pub enable_structured: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            device: DeviceConfig::default(),
            training: TrainingConfig::default(),
            inference: InferenceConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            model_path: "model.npy".to_string(),
            data_path: "data/".to_string(),
            batch_size: 32,
            learning_rate: 0.001,
            epochs: 10,
            enable_distributed: true,
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            model_path: "model.npy".to_string(),
            batch_size: 1,
            enable_gpu: true,
            timeout_secs: 30,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: Some("logs/app.log".to_string()),
            enable_console: true,
            enable_structured: false,
        }
    }
}

/// 配置管理器
pub struct ConfigManager {
    config: Arc<RwLock<AppConfig>>,
    config_path: Option<String>,
    watchers: Arc<RwLock<Vec<Box<dyn ConfigWatcher + Send + Sync>>>>,
}

/// 配置观察者接口
pub trait ConfigWatcher {
    /// 配置更新时调用
    fn on_config_updated(&self, config: &AppConfig);
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(AppConfig::default())),
            config_path: None,
            watchers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// 从文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = std::fs::read_to_string(&path)?;
        let config: AppConfig = toml::from_str(&content)?;
        
        *self.config.write() = config;
        self.config_path = Some(path.as_ref().to_string_lossy().to_string());
        
        self.notify_watchers();
        Ok(())
    }
    
    /// 从字符串加载配置
    pub fn load_from_str(&mut self, content: &str) -> Result<()> {
        let config: AppConfig = toml::from_str(content)?;
        *self.config.write() = config;
        self.notify_watchers();
        Ok(())
    }
    
    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let config = self.config.read();
        let content = toml::to_string_pretty(&*config)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// 获取当前配置
    pub fn get_config(&self) -> AppConfig {
        self.config.read().clone()
    }
    
    /// 更新配置
    pub fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write();
        updater(&mut config);
        self.notify_watchers();
        Ok(())
    }
    
    /// 添加配置观察者
    pub fn add_watcher(&self, watcher: Box<dyn ConfigWatcher + Send + Sync>) {
        self.watchers.write().push(watcher);
    }
    
    /// 通知观察者配置更新
    fn notify_watchers(&self) {
        let config = self.config.read();
        let watchers = self.watchers.read();
        
        for watcher in watchers.iter() {
            watcher.on_config_updated(&config);
        }
    }
    
    /// 获取配置路径
    pub fn config_path(&self) -> Option<&str> {
        self.config_path.as_deref()
    }
}

/// 配置构建器
pub struct ConfigBuilder {
    config: AppConfig,
}

impl ConfigBuilder {
    /// 创建新的配置构建器
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
        }
    }

    /// 设置网络配置
    pub fn network(mut self, network: NetworkConfig) -> Self {
        self.config.network = network;
        self
    }
    
    /// 设置设备配置
    pub fn device(mut self, device: DeviceConfig) -> Self {
        self.config.device = device;
        self
    }
    
    /// 设置训练配置
    pub fn training(mut self, training: TrainingConfig) -> Self {
        self.config.training = training;
        self
    }
    
    /// 设置推理配置
    pub fn inference(mut self, inference: InferenceConfig) -> Self {
        self.config.inference = inference;
        self
    }
    
    /// 设置日志配置
    pub fn logging(mut self, logging: LoggingConfig) -> Self {
        self.config.logging = logging;
        self
    }
    
    /// 构建配置
    pub fn build(self) -> AppConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

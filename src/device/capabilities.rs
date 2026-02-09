//! 设备能力结构
//! 
//! 定义设备能力的数据结构和相关功能。

use super::{DeviceType, GpuComputeApi, NetworkType};

/// 设备能力
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceCapabilities {
    /// 最大可用内存（MB）
    pub max_memory_mb: u64,
    /// CPU 核心数
    pub cpu_cores: u32,
    /// 是否有 GPU
    pub has_gpu: bool,
    /// CPU 架构
    pub cpu_architecture: String,
    /// 支持的 GPU 计算 API
    pub gpu_compute_apis: Vec<GpuComputeApi>,
    /// 是否有 TPU/NPU
    pub has_tpu: Option<bool>,
    /// 网络类型
    pub network_type: NetworkType,
    /// 电池电量（0-100），None 表示没有电池
    pub battery_level: Option<f32>,
    /// 是否正在充电
    pub is_charging: Option<bool>,
    /// 设备类型
    pub device_type: DeviceType,
}

impl DeviceCapabilities {
    /// 创建新的设备能力实例
    pub fn new(
        max_memory_mb: u64,
        cpu_cores: u32,
        has_gpu: bool,
        cpu_architecture: String,
        gpu_compute_apis: Vec<GpuComputeApi>,
        has_tpu: Option<bool>,
        network_type: NetworkType,
        battery_level: Option<f32>,
        is_charging: Option<bool>,
        device_type: DeviceType,
    ) -> Self {
        Self {
            max_memory_mb,
            cpu_cores,
            has_gpu,
            cpu_architecture,
            gpu_compute_apis,
            has_tpu,
            network_type,
            battery_level,
            is_charging,
            device_type,
        }
    }
    
    /// 获取性能评分（0-1）
    pub fn performance_score(&self) -> f64 {
        let mut score = 0.0;
        
        // CPU 核心数评分
        let cpu_score = (self.cpu_cores as f64).min(16.0) / 16.0;
        score += cpu_score * 0.3;
        
        // 内存评分
        let memory_score = (self.max_memory_mb as f64).min(16384.0) / 16384.0; // 16GB 为满分
        score += memory_score * 0.3;
        
        // GPU 评分
        let gpu_score = if self.has_gpu { 0.2 } else { 0.0 };
        score += gpu_score;
        
        // TPU 评分
        let tpu_score = if self.has_tpu.unwrap_or(false) { 0.1 } else { 0.0 };
        score += tpu_score;
        
        // 网络评分
        let network_score = match self.network_type {
            NetworkType::WiFi => 0.1,
            NetworkType::Cellular5G => 0.06,
            NetworkType::Cellular4G => 0.04,
            NetworkType::Unknown => 0.02,
        };
        score += network_score;
        
        score.min(1.0)
    }
    
    /// 检查是否支持特定 GPU API
    pub fn supports_gpu_api(&self, api: GpuComputeApi) -> bool {
        self.gpu_compute_apis.contains(&api)
    }
    
    /// 检查是否有足够的资源
    pub fn has_sufficient_resources(&self, required_memory_mb: u64, required_cores: u32) -> bool {
        self.max_memory_mb >= required_memory_mb && self.cpu_cores >= required_cores
    }
    
    /// 获取电池状态字符串
    pub fn battery_status(&self) -> String {
        match (self.battery_level, self.is_charging) {
            (Some(level), Some(true)) => format!("{}% (充电中)", level),
            (Some(level), Some(false)) => format!("{}% (使用电池)", level),
            (Some(level), None) => format!("{}%", level),
            (None, _) => "无电池".to_string(),
        }
    }
    
    /// 获取设备摘要
    pub fn summary(&self) -> String {
        format!(
            "{} ({}核心, {}MB内存, {}{}{})",
            self.device_type_str(),
            self.cpu_cores,
            self.max_memory_mb,
            if self.has_gpu { "有GPU" } else { "无GPU" },
            if self.has_tpu.unwrap_or(false) { ", 有TPU" } else { "" },
            if self.battery_level.is_some() { ", 电池" } else { "" }
        )
    }
    
    /// 获取设备类型字符串
    fn device_type_str(&self) -> &str {
        match self.device_type {
            DeviceType::Desktop => "桌面设备",
            DeviceType::Phone => "手机",
            DeviceType::Tablet => "平板",
            DeviceType::Unknown => "未知设备",
        }
    }
    
    /// 获取推荐的tick间隔（毫秒）
    pub fn recommended_tick_interval(&self) -> std::time::Duration {
        let millis = match self.device_type {
            DeviceType::Phone => 1000,      // 手机：1秒
            DeviceType::Tablet => 500,     // 平板：0.5秒
            DeviceType::Desktop => 100,     // 桌面：0.1秒
            DeviceType::Unknown => 1000,    // 未知：1秒
        };
        std::time::Duration::from_millis(millis)
    }
    
    /// 获取推荐的模型维度
    pub fn recommended_model_dim(&self) -> usize {
        // 基于内存和CPU核心数计算推荐模型维度
        if self.max_memory_mb >= 8192 {
            1024  // 8GB+
        } else if self.max_memory_mb >= 4096 {
            512   // 4GB+
        } else if self.max_memory_mb >= 2048 {
            256   // 2GB+
        } else {
            128   // <2GB
        }
    }
    
    /// 检查是否应该暂停训练（低电量保护）
    pub fn should_pause_training(&self) -> bool {
        if let Some(battery_level) = self.battery_level {
            // 电量低于20%且未充电时暂停训练
            battery_level < 0.2 && !self.is_charging.unwrap_or(true)
        } else {
            false
        }
    }
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            max_memory_mb: 8192, // 8GB
            cpu_cores: 4,
            has_gpu: false,
            cpu_architecture: "x86_64".to_string(),
            gpu_compute_apis: Vec::new(),
            has_tpu: None,
            network_type: NetworkType::Unknown,
            battery_level: None,
            is_charging: None,
            device_type: DeviceType::Desktop,
        }
    }
}
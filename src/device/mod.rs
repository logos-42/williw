//! 设备管理模块
//! 
//! 此模块提供了跨平台的设备能力检测功能，包括：
//! - CPU、GPU、NPU/TPU 检测
//! - 网络类型检测（WiFi、4G、5G）
//! - 电池状态检测
//! - 设备能力管理和运行时更新

pub mod detection;
pub mod capabilities;
pub mod manager;
pub mod platform;
pub mod types;

// 重新导出公共接口
pub use detection::*;
pub use capabilities::*;
pub use manager::*;
pub use types::*;
pub use platform::*;

/// 设备配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceConfig {
    /// 是否启用 GPU 加速
    pub enable_gpu_acceleration: bool,
    /// 是否启用 TPU/NPU 加速
    pub enable_tpu_acceleration: bool,
    /// 最大内存使用限制（MB）
    pub max_memory_mb: u64,
    /// 最大 CPU 核心使用数
    pub max_cpu_cores: u32,
    /// 是否启用电池优化
    pub enable_battery_optimization: bool,
    /// 是否启用网络感知
    pub enable_network_awareness: bool,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            enable_gpu_acceleration: true,
            enable_tpu_acceleration: true,
            max_memory_mb: 4096, // 4GB
            max_cpu_cores: 4,
            enable_battery_optimization: true,
            enable_network_awareness: true,
        }
    }
}

use crate::device::capabilities::DeviceCapabilities;
use crate::device::detection::DeviceDetector;
use crate::device::types::NetworkType;
use parking_lot::RwLock;
use std::sync::Arc;

/// 设备能力管理器（支持运行时更新）
pub struct DeviceManager {
    capabilities: Arc<RwLock<DeviceCapabilities>>,
}

impl Clone for DeviceManager {
    fn clone(&self) -> Self {
        // 仅克隆Arc指针，不复制内部数据
        Self {
            capabilities: Arc::clone(&self.capabilities),
        }
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        let caps = DeviceDetector::detect();
        Self {
            capabilities: Arc::new(RwLock::new(caps)),
        }
    }

    pub fn with_capabilities(capabilities: DeviceCapabilities) -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(capabilities)),
        }
    }

    pub fn get(&self) -> DeviceCapabilities {
        self.capabilities.read().clone()
    }

    /// 更新网络类型（用于 FFI 和运行时更新）
    pub fn update_network_type(&self, network_type: NetworkType) {
        self.capabilities.write().network_type = network_type;
    }

    /// 更新电池状态（用于 FFI 和运行时更新）
    pub fn update_battery(&self, level: Option<f32>, is_charging: bool) {
        let mut caps = self.capabilities.write();
        caps.battery_level = level;
        caps.is_charging = Some(is_charging);
    }
    
    /// 更新内存和 CPU 信息（用于 FFI 回调）
    pub fn update_hardware(&self, memory_mb: usize, cpu_cores: usize) {
        let mut caps = self.capabilities.write();
        caps.max_memory_mb = memory_mb as u64;
        caps.cpu_cores = cpu_cores as u32;
    }

    pub fn refresh(&self) {
        let mut caps = self.capabilities.write();
        *caps = DeviceDetector::detect();
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}


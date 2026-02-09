//! 设备检测模块
//! 
//! 提供跨平台的设备能力检测功能。

use crate::device::platform;
use crate::device::capabilities::DeviceCapabilities;
use crate::device::types::{DeviceType, GpuComputeApi, NetworkType, GpuUsageInfo};

/// 设备检测器
pub struct DeviceDetector;

impl DeviceDetector {
    /// 检测当前设备能力
    pub fn detect() -> DeviceCapabilities {
        // 检测 CPU 架构
        let cpu_architecture = Self::detect_cpu_architecture();
        
        // 检测 GPU API
        let gpu_compute_apis = Self::detect_gpu_apis();
        let has_gpu = !gpu_compute_apis.is_empty();
        
        // 检测 TPU/NPU
        let has_tpu = Self::detect_tpu();
        
        // 检测内存和 CPU 核心数
        let (max_memory_mb, cpu_cores) = Self::detect_memory_and_cpu();
        
        // 检测网络类型
        let network_type = Self::detect_network_type();
        
        // 检测电池状态
        let (battery_level, is_charging) = Self::detect_battery();
        
        // 判断设备类型
        let device_type = if battery_level.is_some() {
            if max_memory_mb >= 2048 {
                DeviceType::Tablet
            } else {
                DeviceType::Phone
            }
        } else {
            DeviceType::Desktop
        };
        
        DeviceCapabilities {
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
    
    /// 检测 CPU 架构
    fn detect_cpu_architecture() -> String {
        use sysinfo::System;
        
        let mut system = System::default();
        system.refresh_cpu_usage();
        
        // 尝试获取 CPU 品牌字符串
        if let Some(cpu) = system.cpus().first() {
            let brand = cpu.brand();
            if !brand.is_empty() {
                return format!("{} ({})", std::env::consts::ARCH, brand);
            }
        }
        
        // 回退到基本架构
        std::env::consts::ARCH.to_string()
    }
    
    /// 检测 GPU 计算 API
    fn detect_gpu_apis() -> Vec<GpuComputeApi> {
        crate::device::platform::detect_gpu_apis()
    }
    
    /// 检测 TPU/NPU 支持
    fn detect_tpu() -> Option<bool> {
        crate::device::platform::detect_tpu()
    }
    
    /// 检测内存和 CPU 核心数
    fn detect_memory_and_cpu() -> (u64, u32) {
        use sysinfo::System;
        
        let mut system = System::default();
        system.refresh_memory();
        system.refresh_cpu_usage();
        
        // system.total_memory() 返回字节，转换为 MB（除以 1024*1024）
        let max_memory_mb = system.total_memory() / (1024 * 1024);
        let cpu_cores = system.cpus().len() as u32;
        
        (max_memory_mb, cpu_cores)
    }
    
    /// 检测网络类型
    fn detect_network_type() -> NetworkType {
        crate::device::platform::detect_network_type()
    }
    
    /// 检测电池状态
    fn detect_battery() -> (Option<f32>, Option<bool>) {
        let (level, is_charging) = platform::detect_battery();
        (level, Some(is_charging))
    }

    /// 检测 GPU 使用率（平台特定）
    pub fn detect_gpu_usage() -> Vec<GpuUsageInfo> {
        crate::device::platform::detect_gpu_usage()
    }
}

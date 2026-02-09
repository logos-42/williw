#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

use crate::device::types::{GpuComputeApi, NetworkType, GpuUsageInfo};

// 统一的平台检测函数
pub fn detect_gpu_apis() -> Vec<GpuComputeApi> {
    #[cfg(target_os = "windows")]
    {
        windows::detect_gpu_apis()
    }
    #[cfg(target_os = "linux")]
    {
        linux::detect_gpu_apis()
    }
    #[cfg(target_os = "macos")]
    {
        macos::detect_gpu_apis()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // 其他平台的默认实现
        Vec::new()
    }
}

pub fn detect_tpu() -> Option<bool> {
    #[cfg(target_os = "windows")]
    {
        windows::detect_tpu()
    }
    #[cfg(target_os = "linux")]
    {
        linux::detect_tpu()
    }
    #[cfg(target_os = "macos")]
    {
        macos::detect_tpu()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // 其他平台的默认实现
        None
    }
}

pub fn detect_network_type() -> NetworkType {
    #[cfg(target_os = "windows")]
    {
        windows::detect_network_type()
    }
    #[cfg(target_os = "linux")]
    {
        linux::detect_network_type()
    }
    #[cfg(target_os = "macos")]
    {
        macos::detect_network_type()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // 其他平台的默认实现
        NetworkType::Unknown
    }
}

pub fn detect_battery() -> (Option<f32>, bool) {
    #[cfg(target_os = "windows")]
    {
        windows::detect_battery()
    }
    #[cfg(target_os = "linux")]
    {
        linux::detect_battery()
    }
    #[cfg(target_os = "macos")]
    {
        macos::detect_battery()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // 其他平台的默认实现
        (None, false)
    }
}

pub fn detect_gpu_usage() -> Vec<GpuUsageInfo> {
    #[cfg(target_os = "windows")]
    {
        windows::detect_gpu_usage()
    }
    #[cfg(target_os = "linux")]
    {
        linux::detect_gpu_usage()
    }
    #[cfg(target_os = "macos")]
    {
        macos::detect_gpu_usage()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // 其他平台的默认实现
        Vec::new()
    }
}

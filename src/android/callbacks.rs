//! Android JNI 回调实现
//! 
//! 实现 JNI 回调函数，用于从 Java 端获取设备信息

#[cfg(feature = "android")]
use crate::device::{NetworkType};
#[cfg(feature = "android")]
use crate::network::ffi::DeviceInfoCallback;
#[cfg(feature = "android")]
use std::ffi::{CStr, CString};
#[cfg(feature = "android")]
use std::os::raw::{c_char, c_int};

/// JNI 设备信息回调实现
#[cfg(feature = "android")]
pub struct JniDeviceInfoCallback;

#[cfg(feature = "android")]
impl JniDeviceInfoCallback {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "android")]
impl DeviceInfoCallback for JniDeviceInfoCallback {
    extern "C" fn call(
        &self,
        memory_mb: *mut u32,
        cpu_cores: *mut u32,
        network_type: *mut c_char,
        network_type_len: usize,
        battery_level: *mut f32,
        is_charging: *mut c_int,
    ) -> c_int {
        // 这里需要通过 JNI 调用 Java 端的 DeviceInfoProvider
        // 由于 JNI 调用比较复杂，这里先提供基本实现
        // 实际实现需要：
        // 1. 获取 JNIEnv
        // 2. 调用 DeviceInfoProvider.getDeviceInfo()
        // 3. 解析返回的设备信息
        // 4. 填充输出参数
        
        // 暂时返回默认值
        unsafe {
            *memory_mb = 2048; // 默认 2GB
            *cpu_cores = 4;    // 默认 4 核
            *battery_level = 0.8; // 默认 80% 电量
            *is_charging = 1;  // 默认充电中
            
            // 设置默认网络类型
            let wifi_str = CString::new("wifi").unwrap();
            let len = wifi_str.as_bytes().len().min(network_type_len - 1);
            std::ptr::copy_nonoverlapping(
                wifi_str.as_ptr(),
                network_type,
                len,
            );
            *network_type.add(len) = 0; // null 终止符
        }
        
        0 // 成功
    }
}

/// 从 Java 端获取设备信息的辅助函数
#[cfg(feature = "android")]
pub fn get_device_info_from_java() -> Result<JavaDeviceInfo, Box<dyn std::error::Error>> {
    // 这里需要实现完整的 JNI 调用逻辑
    // 暂时返回默认值
    Ok(JavaDeviceInfo {
        memory_mb: 2048,
        cpu_cores: 4,
        network_type: "wifi".to_string(),
        battery_level: Some(0.8),
        is_charging: true,
    })
}

/// Java 设备信息结构
#[cfg(feature = "android")]
#[derive(Debug, Clone)]
pub struct JavaDeviceInfo {
    pub memory_mb: u32,
    pub cpu_cores: u32,
    pub network_type: String,
    pub battery_level: Option<f32>,
    pub is_charging: bool,
}

impl JavaDeviceInfo {
    /// 转换为 NetworkType
    pub fn network_type(&self) -> NetworkType {
        match self.network_type.as_str() {
            "wifi" => NetworkType::WiFi,
            "5g" => NetworkType::Cellular5G,
            "4g" => NetworkType::Cellular4G,
            _ => NetworkType::Unknown,
        }
    }
}

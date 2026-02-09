//! FFI 接口模块，用于移动端（Android/iOS）集成
//!
//! 这个模块提供了 C 兼容的 FFI 接口，允许移动端应用调用 Rust 核心功能

use crate::device::{DeviceCapabilities, DeviceManager, NetworkType};
use crate::types::GgbMessage;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Arc;
use std::sync::Mutex;
use parking_lot::RwLock;

/// FFI 错误代码
#[repr(C)]
pub enum FfiError {
    Success = 0,
    InvalidArgument = 1,
    OutOfMemory = 2,
    NetworkError = 3,
    Unknown = 99,
}

/// 设备信息回调函数类型
/// 
/// 移动端可以通过此回调函数向 Rust 层提供真实的设备信息
/// 参数说明：
/// - memory_mb: 输出参数，设备内存（MB）
/// - cpu_cores: 输出参数，CPU 核心数
/// - network_type: 输出参数，网络类型字符串（"wifi", "4g", "5g", "unknown"）
/// - battery_level: 输出参数，电池电量（0.0-1.0），-1.0 表示无法检测
/// - is_charging: 输出参数，是否正在充电（0=false, 1=true）
/// 返回值：0 表示成功，非0表示失败
pub type DeviceInfoCallback = extern "C" fn(
    memory_mb: *mut u32,
    cpu_cores: *mut u32,
    network_type: *mut c_char,
    network_type_len: usize,
    battery_level: *mut f32,
    is_charging: *mut c_int,
) -> c_int;

/// 节点句柄（不透明指针）
pub struct NodeHandle {
    // 这里可以存储实际的 Node 实例
    // 为了简化，暂时只存储设备管理器
    device_manager: DeviceManager,
    // 设备信息回调函数（可选）
    device_callback: Arc<RwLock<Option<DeviceInfoCallback>>>,
}

/// 创建新的节点实例
///
/// # Safety
/// 返回的指针必须通过 `williw_node_destroy` 释放
#[no_mangle]
pub unsafe extern "C" fn williw_node_create() -> *mut NodeHandle {
    let device_manager = DeviceManager::new();
    let handle = Box::new(NodeHandle {
        device_manager,
        device_callback: Arc::new(RwLock::new(None)),
    });
    Box::into_raw(handle)
}

/// 设置设备信息回调函数
///
/// 移动端可以通过此函数注册一个回调，用于向 Rust 层提供真实的设备信息
/// 当 DeviceManager 需要刷新设备信息时，会调用此回调
///
/// # Safety
/// ptr 必须是有效的节点句柄
/// callback 必须是有效的函数指针，或者 NULL（表示清除回调）
#[no_mangle]
pub unsafe extern "C" fn williw_node_set_device_callback(
    ptr: *mut NodeHandle,
    callback: Option<DeviceInfoCallback>,
) -> c_int {
    if ptr.is_null() {
        return FfiError::InvalidArgument as c_int;
    }
    
    let handle = &mut *ptr;
    *handle.device_callback.write() = callback;
    FfiError::Success as c_int
}

/// 从回调函数获取设备信息并更新 DeviceManager
///
/// # Safety
/// ptr 必须是有效的节点句柄
unsafe fn update_device_from_callback(handle: &mut NodeHandle) -> c_int {
    let callback_opt = handle.device_callback.read().clone();
    
    if let Some(callback) = callback_opt {
        let mut memory_mb: u32 = 0;
        let mut cpu_cores: u32 = 0;
        let mut network_type_buf = vec![0u8; 32];
        let mut battery_level: f32 = -1.0;
        let mut is_charging: c_int = 0;
        
        let result = callback(
            &mut memory_mb,
            &mut cpu_cores,
            network_type_buf.as_mut_ptr() as *mut c_char,
            network_type_buf.len(),
            &mut battery_level,
            &mut is_charging,
        );
        
        if result != 0 {
            return result; // 回调返回错误
        }
        
        // 解析网络类型
        let network_type = if let Ok(net_str) = CStr::from_ptr(network_type_buf.as_ptr() as *const c_char).to_str() {
            match net_str {
                "wifi" => NetworkType::WiFi,
                "5g" => NetworkType::Cellular5G,
                "4g" => NetworkType::Cellular4G,
                _ => NetworkType::Unknown,
            }
        } else {
            NetworkType::Unknown
        };
        
        // 更新设备管理器
        handle.device_manager.update_network_type(network_type);
        handle.device_manager.update_hardware(memory_mb as usize, cpu_cores as usize);
        
        let battery_level_opt = if battery_level >= 0.0 && battery_level <= 1.0 {
            Some(battery_level)
        } else {
            None
        };
        
        handle.device_manager.update_battery(battery_level_opt, is_charging != 0);
        
        FfiError::Success as c_int
    } else {
        // 没有回调，使用默认检测
        FfiError::Success as c_int
    }
}

/// 销毁节点实例
///
/// # Safety
/// ptr 必须是通过 `williw_node_create` 创建的有效指针
#[no_mangle]
pub unsafe extern "C" fn williw_node_destroy(ptr: *mut NodeHandle) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// 获取设备能力信息（JSON 格式）
///
/// # Safety
/// ptr 必须是有效的节点句柄
/// 返回的字符串必须通过 `williw_string_free` 释放
#[no_mangle]
pub unsafe extern "C" fn williw_node_get_capabilities(
    ptr: *const NodeHandle,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    
    let handle = &*ptr;
    let caps = handle.device_manager.get();
    
    // 序列化为 JSON
    match serde_json::to_string(&caps) {
        Ok(json) => {
            match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// 更新网络类型
///
/// # Safety
/// ptr 必须是有效的节点句柄
/// network_type_str 必须是有效的 C 字符串
#[no_mangle]
pub unsafe extern "C" fn williw_node_update_network_type(
    ptr: *mut NodeHandle,
    network_type_str: *const c_char,
) -> c_int {
    if ptr.is_null() || network_type_str.is_null() {
        return FfiError::InvalidArgument as c_int;
    }
    
    let handle = &mut *ptr;
    let network_type = match CStr::from_ptr(network_type_str).to_str() {
        Ok(s) => match s {
            "wifi" => NetworkType::WiFi,
            "5g" => NetworkType::Cellular5G,
            "4g" => NetworkType::Cellular4G,
            _ => NetworkType::Unknown,
        },
        Err(_) => return FfiError::InvalidArgument as c_int,
    };
    
    handle.device_manager.update_network_type(network_type);
    FfiError::Success as c_int
}

/// 刷新设备信息（从回调函数获取）
///
/// 如果已设置设备信息回调，会调用回调获取最新设备信息并更新
///
/// # Safety
/// ptr 必须是有效的节点句柄
#[no_mangle]
pub unsafe extern "C" fn williw_node_refresh_device_info(ptr: *mut NodeHandle) -> c_int {
    if ptr.is_null() {
        return FfiError::InvalidArgument as c_int;
    }
    
    let handle = &mut *ptr;
    update_device_from_callback(handle)
}

/// 更新电池状态
///
/// # Safety
/// ptr 必须是有效的节点句柄
#[no_mangle]
pub unsafe extern "C" fn williw_node_update_battery(
    ptr: *mut NodeHandle,
    level: f32,      // 0.0-1.0
    is_charging: c_int, // 0 = false, 1 = true
) -> c_int {
    if ptr.is_null() {
        return FfiError::InvalidArgument as c_int;
    }
    
    let handle = &mut *ptr;
    let level_opt = if level >= 0.0 && level <= 1.0 {
        Some(level)
    } else {
        None
    };
    let charging = is_charging != 0;
    
    handle.device_manager.update_battery(level_opt, charging);
    FfiError::Success as c_int
}

/// 释放由 FFI 函数返回的字符串
///
/// # Safety
/// ptr 必须是通过 FFI 函数返回的有效字符串指针
#[no_mangle]
pub unsafe extern "C" fn williw_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// 获取推荐的模型维度
///
/// # Safety
/// ptr 必须是有效的节点句柄
#[no_mangle]
pub unsafe extern "C" fn williw_node_recommended_model_dim(
    ptr: *const NodeHandle,
) -> usize {
    if ptr.is_null() {
        return 256; // 默认值
    }
    
    let handle = &*ptr;
    let caps = handle.device_manager.get();
    caps.recommended_model_dim()
}

/// 获取推荐的训练间隔（秒）
///
/// # Safety
/// ptr 必须是有效的节点句柄
#[no_mangle]
pub unsafe extern "C" fn williw_node_recommended_tick_interval(
    ptr: *const NodeHandle,
) -> u64 {
    if ptr.is_null() {
        return 10; // 默认 10 秒
    }
    
    let handle = &*ptr;
    let caps = handle.device_manager.get();
    caps.recommended_tick_interval().as_secs()
}

/// 检查是否应该暂停训练
///
/// # Safety
/// ptr 必须是有效的节点句柄
#[no_mangle]
pub unsafe extern "C" fn williw_node_should_pause_training(
    ptr: *const NodeHandle,
) -> c_int {
    if ptr.is_null() {
        return 0; // false
    }
    
    let handle = &*ptr;
    let caps = handle.device_manager.get();
    if caps.should_pause_training() {
        1 // true
    } else {
        0 // false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_create_destroy() {
        unsafe {
            let ptr = williw_node_create();
            assert!(!ptr.is_null());
            williw_node_destroy(ptr);
        }
    }

    #[test]
    fn test_get_capabilities() {
        unsafe {
            let ptr = williw_node_create();
            let json_ptr = williw_node_get_capabilities(ptr);
            assert!(!json_ptr.is_null());
            
            let json = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert!(json.contains("max_memory_mb"));
            
            williw_string_free(json_ptr);
            williw_node_destroy(ptr);
        }
    }

    #[test]
    fn test_update_network_type() {
        unsafe {
            let ptr = williw_node_create();
            let wifi = CString::new("wifi").unwrap();
            let result = williw_node_update_network_type(ptr, wifi.as_ptr());
            assert_eq!(result, FfiError::Success as c_int);
            williw_node_destroy(ptr);
        }
    }

    #[test]
    fn test_update_battery() {
        unsafe {
            let ptr = williw_node_create();
            let result = williw_node_update_battery(ptr, 0.75, 1);
            assert_eq!(result, FfiError::Success as c_int);
            williw_node_destroy(ptr);
        }
    }

    #[test]
    fn test_set_device_callback() {
        unsafe {
            let ptr = williw_node_create();
            
            // 测试设置回调
            extern "C" fn test_callback(
                memory_mb: *mut u32,
                cpu_cores: *mut u32,
                network_type: *mut c_char,
                _network_type_len: usize,
                battery_level: *mut f32,
                is_charging: *mut c_int,
            ) -> c_int {
                *memory_mb = 2048;
                *cpu_cores = 8;
                let wifi_str = CString::new("wifi").unwrap();
                std::ptr::copy_nonoverlapping(
                    wifi_str.as_ptr(),
                    network_type,
                    wifi_str.as_bytes().len().min(31),
                );
                *battery_level = 0.85;
                *is_charging = 1;
                FfiError::Success as c_int
            }
            
            let result = williw_node_set_device_callback(ptr, Some(test_callback));
            assert_eq!(result, FfiError::Success as c_int);
            
            // 测试刷新设备信息
            let refresh_result = williw_node_refresh_device_info(ptr);
            assert_eq!(refresh_result, FfiError::Success as c_int);
            
            // 验证设备信息已更新
            let caps_json = williw_node_get_capabilities(ptr);
            assert!(!caps_json.is_null());
            let json_str = CStr::from_ptr(caps_json).to_str().unwrap();
            assert!(json_str.contains("\"network_type\":\"WiFi\""));
            williw_string_free(caps_json);
            
            // 测试清除回调
            let clear_result = williw_node_set_device_callback(ptr, None);
            assert_eq!(clear_result, FfiError::Success as c_int);
            
            williw_node_destroy(ptr);
        }
    }
}


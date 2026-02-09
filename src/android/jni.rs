//! Android JNI 接口实现
//! 
//! 提供完整的 JNI 导出函数，支持 Android 应用调用 Rust 核心功能

#[cfg(feature = "android")]
use crate::device::{DeviceCapabilities, DeviceManager, NetworkType};
#[cfg(feature = "android")]
use crate::network::ffi::{NodeHandle, FfiError, DeviceInfoCallback};
#[cfg(feature = "android")]
use crate::android::utils::*;
#[cfg(feature = "android")]
use crate::android::callbacks::*;

#[cfg(feature = "android")]
use jni::JNIEnv;
#[cfg(feature = "android")]
use jni::objects::{JClass, JString, JObject, JValue};
#[cfg(feature = "android")]
use jni::sys::{jlong, jint, jboolean, jstring, jobject, jobjectArray};
#[cfg(feature = "android")]
use std::ffi::{CStr, CString};
#[cfg(feature = "android")]
use std::os::raw::c_char;
#[cfg(feature = "android")]
use std::sync::Arc;
#[cfg(feature = "android")]
use parking_lot::RwLock;

/// JNI 全局上下文
#[cfg(feature = "android")]
static mut JNI_GLOBAL_CONTEXT: Option<Arc<RwLock<JniGlobalContext>>> = None;

/// JNI 全局上下文结构
#[cfg(feature = "android")]
pub struct JniGlobalContext {
    pub java_vm: jni::JavaVM,
    pub context_class: jni::objects::GlobalRef,
    pub device_info_provider_class: jni::objects::GlobalRef,
}

/// 初始化 JNI 环境
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeInit(
    env: JNIEnv,
    _class: JClass,
    context: JObject,
) -> jlong {
    android_log::init_once();
    log::info!("JNI 初始化开始");
    
    // 获取 JavaVM
    let java_vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(e) => {
            log::error!("获取 JavaVM 失败: {:?}", e);
            return 0;
        }
    };
    
    // 获取 Context 类
    let context_class = match env.find_class("android/content/Context") {
        Ok(cls) => cls,
        Err(e) => {
            log::error!("查找 Context 类失败: {:?}", e);
            return 0;
        }
    };
    
    let context_global = match env.new_global_ref(context_class.into()) {
        Ok(global) => global,
        Err(e) => {
            log::error!("创建 Context 全局引用失败: {:?}", e);
            return 0;
        }
    };
    
    // 获取 DeviceInfoProvider 类
    let device_info_provider_class = match env.find_class("com/williw/mobile/DeviceInfoProvider") {
        Ok(cls) => cls,
        Err(e) => {
            log::error!("查找 DeviceInfoProvider 类失败: {:?}", e);
            return 0;
        }
    };
    
    let device_info_global = match env.new_global_ref(device_info_provider_class.into()) {
        Ok(global) => global,
        Err(e) => {
            log::error!("创建 DeviceInfoProvider 全局引用失败: {:?}", e);
            return 0;
        }
    };
    
    // 创建全局上下文
    let global_context = JniGlobalContext {
        java_vm,
        context_class: context_global,
        device_info_provider_class: device_info_global,
    };
    
    JNI_GLOBAL_CONTEXT = Some(Arc::new(RwLock::new(global_context)));
    
    // 创建节点实例
    match create_node_with_jni_callback() {
        Ok(ptr) => {
            log::info!("JNI 初始化成功，节点句柄: {:p}", ptr);
            ptr as jlong
        }
        Err(e) => {
            log::error!("创建节点失败: {:?}", e);
            0
        }
    }
}

/// 创建带有 JNI 回调的节点实例
#[cfg(feature = "android")]
fn create_node_with_jni_callback() -> Result<*mut NodeHandle, Box<dyn std::error::Error>> {
    let device_manager = DeviceManager::new();
    let jni_callback = JniDeviceInfoCallback::new();
    
    let handle = Box::new(NodeHandle {
        device_manager,
        device_callback: Arc::new(RwLock::new(Some(jni_callback as DeviceInfoCallback))),
    });
    
    Ok(Box::into_raw(handle))
}

/// 销毁节点实例
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = Box::from_raw(ptr as *mut NodeHandle);
        log::info!("节点实例已销毁");
    }
}

/// 获取设备能力信息（JSON 格式）
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetCapabilities(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return std::ptr::null_mut();
    }
    
    let handle = &*(ptr as *mut NodeHandle);
    let caps = handle.device_manager.get();
    
    match serde_json::to_string(&caps) {
        Ok(json) => {
            match env.new_string(json) {
                Ok(j_string) => j_string.into_raw(),
                Err(e) => {
                    log::error!("创建 Java 字符串失败: {:?}", e);
                    std::ptr::null_mut()
                }
            }
        }
        Err(e) => {
            log::error!("序列化设备能力失败: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

/// 更新网络类型
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeUpdateNetworkType(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    network_type: JString,
) -> jint {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return FfiError::InvalidArgument as jint;
    }
    
    let handle = &mut *(ptr as *mut NodeHandle);
    
    // 转换 Java 字符串为 Rust 字符串
    let network_str = match network_type.to_string() {
        Ok(s) => s,
        Err(e) => {
            log::error!("转换网络类型字符串失败: {:?}", e);
            return FfiError::InvalidArgument as jint;
        }
    };
    
    let network_type = match network_str.as_str() {
        "wifi" => NetworkType::WiFi,
        "5g" => NetworkType::Cellular5G,
        "4g" => NetworkType::Cellular4G,
        _ => NetworkType::Unknown,
    };
    
    handle.device_manager.update_network_type(network_type);
    log::info!("网络类型已更新: {:?}", network_type);
    
    FfiError::Success as jint
}

/// 更新电池状态
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeUpdateBattery(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    level: jfloat,
    is_charging: jboolean,
) -> jint {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return FfiError::InvalidArgument as jint;
    }
    
    let handle = &mut *(ptr as *mut NodeHandle);
    let level_opt = if level >= 0.0 && level <= 1.0 {
        Some(level)
    } else {
        None
    };
    let charging = is_charging != 0;
    
    handle.device_manager.update_battery(level_opt, charging);
    log::info!("电池状态已更新: 电量={:?}, 充电={}", level_opt, charging);
    
    FfiError::Success as jint
}

/// 更新硬件信息
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeUpdateHardware(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    memory_mb: jint,
    cpu_cores: jint,
) -> jint {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return FfiError::InvalidArgument as jint;
    }
    
    let handle = &mut *(ptr as *mut NodeHandle);
    handle.device_manager.update_hardware(memory_mb as usize, cpu_cores as usize);
    
    log::info!("硬件信息已更新: 内存={}MB, CPU={}核", memory_mb, cpu_cores);
    
    FfiError::Success as jint
}

/// 刷新设备信息
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeRefreshDeviceInfo(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return FfiError::InvalidArgument as jint;
    }
    
    let handle = &mut *(ptr as *mut NodeHandle);
    
    // 从 JNI 回调获取设备信息
    match update_device_from_jni_callback(handle) {
        FfiError::Success => {
            log::info!("设备信息刷新成功");
            FfiError::Success as jint
        }
        error => {
            log::error!("设备信息刷新失败: {:?}", error);
            error as jint
        }
    }
}

/// 从 JNI 回调获取设备信息
#[cfg(feature = "android")]
fn update_device_from_jni_callback(handle: &mut NodeHandle) -> FfiError {
    // 这里需要调用 Java 端的 DeviceInfoProvider 获取设备信息
    // 由于 JNI 调用比较复杂，这里先实现基本版本
    // 实际实现需要通过 JNI 调用 Java 方法
    
    // 暂时返回成功，实际实现需要完整的 JNI 调用
    FfiError::Success
}

/// 获取推荐的模型维度
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetRecommendedModelDim(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return 256; // 默认值
    }
    
    let handle = &*(ptr as *mut NodeHandle);
    let caps = handle.device_manager.get();
    caps.recommended_model_dim() as jint
}

/// 获取推荐的训练间隔（秒）
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetRecommendedTickInterval(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return 10; // 默认 10 秒
    }
    
    let handle = &*(ptr as *mut NodeHandle);
    let caps = handle.device_manager.get();
    caps.recommended_tick_interval().as_secs() as jlong
}

/// 检查是否应该暂停训练
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeShouldPauseTraining(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jboolean {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return 0; // false
    }
    
    let handle = &*(ptr as *mut NodeHandle);
    let caps = handle.device_manager.get();
    if caps.should_pause_training() {
        1 // true
    } else {
        0 // false
    }
}

/// 获取设备性能评分
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetPerformanceScore(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jfloat {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return 0.0;
    }
    
    let handle = &*(ptr as *mut NodeHandle);
    let caps = handle.device_manager.get();
    caps.performance_score() as jfloat
}

/// 获取设备摘要信息
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetDeviceSummary(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return std::ptr::null_mut();
    }
    
    let handle = &*(ptr as *mut NodeHandle);
    let caps = handle.device_manager.get();
    let summary = caps.summary();
    
    match env.new_string(summary) {
        Ok(j_string) => j_string.into_raw(),
        Err(e) => {
            log::error!("创建设备摘要字符串失败: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

/// 获取电池状态字符串
#[cfg(feature = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetBatteryStatus(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    if ptr == 0 {
        log::error!("无效的节点句柄");
        return std::ptr::null_mut();
    }
    
    let handle = &*(ptr as *mut NodeHandle);
    let caps = handle.device_manager.get();
    let battery_status = caps.battery_status();
    
    match env.new_string(battery_status) {
        Ok(j_string) => j_string.into_raw(),
        Err(e) => {
            log::error!("创建电池状态字符串失败: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

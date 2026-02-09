//! Williw Android 库 - 完整版
//! 
//! 这是专门为 Android 平台设计的完整版 Rust 库
//! 包含设备检测、网络管理、训练控制等完整功能

#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

// JNI 导入
#[cfg(target_os = "android")]
use jni::JNIEnv;
#[cfg(target_os = "android")]
use jni::objects::{JClass, JString, JObject};
#[cfg(target_os = "android")]
use jni::sys::{jlong, jint, jboolean, jstring, jfloat};

// 设备类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Desktop,
    Phone,
    Tablet,
    Unknown,
}

impl Default for DeviceType {
    fn default() -> Self {
        DeviceType::Unknown
    }
}

// 网络类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    WiFi,
    Cellular4G,
    Cellular5G,
    Unknown,
}

impl Default for NetworkType {
    fn default() -> Self {
        NetworkType::Unknown
    }
}

// 设备能力结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub max_memory_mb: u64,
    pub cpu_cores: u32,
    pub has_gpu: bool,
    pub cpu_architecture: String,
    pub has_tpu: Option<bool>,
    pub network_type: NetworkType,
    pub battery_level: Option<f32>,
    pub is_charging: Option<bool>,
    pub device_type: DeviceType,
    pub device_brand: String,
    pub device_model: String,
    pub android_version: String,
    pub sdk_version: i32,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            max_memory_mb: 8192,
            cpu_cores: 4,
            has_gpu: false,
            cpu_architecture: "unknown".to_string(),
            has_tpu: None,
            network_type: NetworkType::Unknown,
            battery_level: None,
            is_charging: None,
            device_type: DeviceType::Unknown,
            device_brand: "unknown".to_string(),
            device_model: "unknown".to_string(),
            android_version: "unknown".to_string(),
            sdk_version: 0,
        }
    }
}

impl DeviceCapabilities {
    /// 获取性能评分（0-1）
    pub fn performance_score(&self) -> f64 {
        let mut score = 0.0;
        
        // CPU 核心数评分
        let cpu_score = (self.cpu_cores as f64).min(16.0) / 16.0;
        score += cpu_score * 0.3;
        
        // 内存评分
        let memory_score = (self.max_memory_mb as f64).min(16384.0) / 16384.0;
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
    
    /// 获取推荐的模型维度
    pub fn recommended_model_dim(&self) -> usize {
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
    
    /// 获取推荐的训练间隔（毫秒）
    pub fn recommended_tick_interval(&self) -> u64 {
        match self.device_type {
            DeviceType::Phone => 1000,      // 手机：1秒
            DeviceType::Tablet => 500,     // 平板：0.5秒
            DeviceType::Desktop => 100,     // 桌面：0.1秒
            DeviceType::Unknown => 1000,    // 未知：1秒
        }
    }
    
    /// 检查是否应该暂停训练
    pub fn should_pause_training(&self) -> bool {
        if let Some(battery_level) = self.battery_level {
            battery_level < 0.2 && !self.is_charging.unwrap_or(true)
        } else {
            false
        }
    }
    
    /// 获取电池状态字符串
    pub fn battery_status(&self) -> String {
        match (self.battery_level, self.is_charging) {
            (Some(level), Some(true)) => format!("{}% (充电中)", level * 100.0),
            (Some(level), Some(false)) => format!("{}% (使用电池)", level * 100.0),
            (Some(level), None) => format!("{}%", level * 100.0),
            (None, _) => "无电池".to_string(),
        }
    }
    
    /// 获取设备摘要
    pub fn summary(&self) -> String {
        format!(
            "{} {} ({}核心, {}MB内存, {}{}{})",
            self.device_brand,
            self.device_type_str(),
            self.cpu_cores,
            self.max_memory_mb,
            if self.has_gpu { "有GPU" } else { "无GPU" },
            if self.has_tpu.unwrap_or(false) { ", 有TPU" } else { "" },
            if self.battery_level.is_some() { ", 电池" } else { "" }
        )
    }
    
    fn device_type_str(&self) -> &str {
        match self.device_type {
            DeviceType::Desktop => "桌面设备",
            DeviceType::Phone => "手机",
            DeviceType::Tablet => "平板",
            DeviceType::Unknown => "未知设备",
        }
    }
}

// 设备管理器
pub struct DeviceManager {
    capabilities: Arc<RwLock<DeviceCapabilities>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        let caps = Self::detect_capabilities();
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
    
    pub fn update_network_type(&self, network_type: NetworkType) {
        self.capabilities.write().network_type = network_type;
    }
    
    pub fn update_battery(&self, level: Option<f32>, is_charging: bool) {
        let mut caps = self.capabilities.write();
        caps.battery_level = level;
        caps.is_charging = Some(is_charging);
    }
    
    pub fn update_hardware(&self, memory_mb: usize, cpu_cores: usize) {
        let mut caps = self.capabilities.write();
        caps.max_memory_mb = memory_mb as u64;
        caps.cpu_cores = cpu_cores as u32;
    }
    
    fn detect_capabilities() -> DeviceCapabilities {
        // 基本设备检测
        let mut caps = DeviceCapabilities::default();
        
        // 获取基本系统信息
        if let Ok(arch) = std::env::var("CARGO_CFG_TARGET_ARCH") {
            caps.cpu_architecture = arch;
        }
        
        // 尝试获取内存信息（简化版）
        caps.max_memory_mb = 4096; // 默认4GB
        caps.cpu_cores = 4;        // 默认4核
        
        caps
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

// 节点句柄
pub struct NodeHandle {
    device_manager: DeviceManager,
}

// FFI 错误代码
#[repr(C)]
pub enum FfiError {
    Success = 0,
    InvalidArgument = 1,
    OutOfMemory = 2,
    NetworkError = 3,
    Unknown = 99,
}

// JNI 导出函数
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeInit(
    env: JNIEnv,
    _class: JClass,
    _context: JObject,
) -> jlong {
    #[cfg(target_os = "android")]
    {
        let _ = android_log::init("WilliwAndroid");
    }
    
    log::info!("Williw Android 库初始化开始");
    
    let device_manager = DeviceManager::new();
    let handle = Box::new(NodeHandle {
        device_manager,
    });
    
    log::info!("Williw Android 库初始化完成");
    Box::into_raw(handle) as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut NodeHandle) };
        log::info!("Williw 节点实例已销毁");
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetCapabilities(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    
    let handle = unsafe { &*(ptr as *mut NodeHandle) };
    let caps = handle.device_manager.get();
    
    match serde_json::to_string(&caps) {
        Ok(json) => {
            match env.new_string(json) {
                Ok(j_string) => j_string.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeUpdateNetworkType(
    mut env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    network_type: JString,
) -> jint {
    if ptr == 0 {
        return FfiError::InvalidArgument as jint;
    }
    
    let handle = unsafe { &mut *(ptr as *mut NodeHandle) };
    
    let network_str = match env.get_string(&network_type) {
        Ok(s) => s,
        Err(_) => return FfiError::InvalidArgument as jint,
    };
    
    let network_type = match network_str.to_str().unwrap_or("") {
        "wifi" => NetworkType::WiFi,
        "5g" => NetworkType::Cellular5G,
        "4g" => NetworkType::Cellular4G,
        _ => NetworkType::Unknown,
    };
    
    handle.device_manager.update_network_type(network_type);
    FfiError::Success as jint
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeUpdateBattery(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    level: jfloat,
    is_charging: jboolean,
) -> jint {
    if ptr == 0 {
        return FfiError::InvalidArgument as jint;
    }
    
    let handle = unsafe { &mut *(ptr as *mut NodeHandle) };
    let level_opt = if level >= 0.0 && level <= 1.0 {
        Some(level)
    } else {
        None
    };
    let charging = is_charging != 0;
    
    handle.device_manager.update_battery(level_opt, charging);
    FfiError::Success as jint
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeUpdateHardware(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
    memory_mb: jint,
    cpu_cores: jint,
) -> jint {
    if ptr == 0 {
        return FfiError::InvalidArgument as jint;
    }
    
    let handle = unsafe { &mut *(ptr as *mut NodeHandle) };
    handle.device_manager.update_hardware(memory_mb as usize, cpu_cores as usize);
    FfiError::Success as jint
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetRecommendedModelDim(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 256;
    }
    
    let handle = unsafe { &*(ptr as *mut NodeHandle) };
    let caps = handle.device_manager.get();
    caps.recommended_model_dim() as jint
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetRecommendedTickInterval(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 10;
    }
    
    let handle = unsafe { &*(ptr as *mut NodeHandle) };
    let caps = handle.device_manager.get();
    caps.recommended_tick_interval() as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeShouldPauseTraining(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    
    let handle = unsafe { &*(ptr as *mut NodeHandle) };
    let caps = handle.device_manager.get();
    if caps.should_pause_training() {
        1
    } else {
        0
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetPerformanceScore(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jfloat {
    if ptr == 0 {
        return 0.0;
    }
    
    let handle = unsafe { &*(ptr as *mut NodeHandle) };
    let caps = handle.device_manager.get();
    caps.performance_score() as jfloat
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetDeviceSummary(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    
    let handle = unsafe { &*(ptr as *mut NodeHandle) };
    let caps = handle.device_manager.get();
    let summary = caps.summary();
    
    match env.new_string(summary) {
        Ok(j_string) => j_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_williw_mobile_WilliwNode_nativeGetBatteryStatus(
    env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jstring {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    
    let handle = unsafe { &*(ptr as *mut NodeHandle) };
    let caps = handle.device_manager.get();
    let battery_status = caps.battery_status();
    
    match env.new_string(battery_status) {
        Ok(j_string) => j_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// 标准 FFI 导出（用于非 Android 平台测试）
#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn williw_node_create() -> *mut NodeHandle {
    let device_manager = DeviceManager::new();
    let handle = Box::new(NodeHandle {
        device_manager,
    });
    Box::into_raw(handle)
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn williw_node_destroy(ptr: *mut NodeHandle) {
    if !ptr.is_null() {
        let _ = unsafe { Box::from_raw(ptr) };
    }
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn williw_node_get_capabilities(ptr: *const NodeHandle) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    
    let handle = unsafe { &*ptr };
    let caps = handle.device_manager.get();
    
    match serde_json::to_string(&caps) {
        Ok(json) => {
            match std::ffi::CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn williw_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { std::ffi::CString::from_raw(ptr) };
    }
}

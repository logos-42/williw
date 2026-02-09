//! Android JNI 模块
//! 
//! 此模块提供了完整的 Android JNI 接口，将 Rust 核心功能暴露给 Android 应用
//! 包含设备检测、网络管理、训练控制等完整功能

#[cfg(feature = "android")]
pub mod jni;

#[cfg(feature = "android")]
pub mod callbacks;

#[cfg(feature = "android")]
pub mod utils;

// 重新导出公共接口
#[cfg(feature = "android")]
pub use jni::*;
#[cfg(feature = "android")]
pub use callbacks::*;
#[cfg(feature = "android")]
pub use utils::*;

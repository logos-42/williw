/**
 * 核心通信模块
 * 包含配置、句柄、路由等基础功能
 */

pub mod config;
pub mod handle;
pub mod routing;

// 重新导出常用类型
pub use config::{CommsConfig, BandwidthBudgetConfig};
pub use handle::{CommsHandle, IrohEvent, Topic};

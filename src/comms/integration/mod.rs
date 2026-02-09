/**
 * 应用集成模块
 * 包含应用集成等功能
 */

pub mod app;

// 重新导出常用类型
pub use app::{P2PAppFactory, P2PAppIntegration, P2PEnabledApp};

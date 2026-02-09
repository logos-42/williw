/**
 * 监控模块
 * 包含传输监控、仪表板等功能
 */

pub mod dashboard;

// 重新导出常用类型
pub use dashboard::{MonitoringDashboard, MonitoringStats, TransferHistory, WebApiHandler};

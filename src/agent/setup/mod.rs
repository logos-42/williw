//! AI 配置助手模块
//!
//! 提供 AI 驱动的自动化配置功能，包括：
//! - 系统环境检测
//! - AI 决策配置步骤
//! - 自动依赖安装
//! - GPU 服务启动
//! - 去中心化网络加入

pub mod ai_setup_assistant;
pub mod setup_workflow;
pub mod setup_tasks;

pub use ai_setup_assistant::*;
pub use setup_workflow::*;
pub use setup_tasks::*;

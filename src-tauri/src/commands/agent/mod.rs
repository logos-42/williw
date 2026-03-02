//! Agent 模块
//!
//! AI Agent 相关的命令和工具定义
//! 遵循人月神话原则，拆分为以下子模块：
//! - tools: 工具定义和实现
//! - setup: AI 设置流程
//! - chat: 聊天功能

pub mod tools;
pub mod setup;
pub mod chat;

// Re-export 主要类型
pub use tools::*;
pub use setup::*;
pub use chat::*;

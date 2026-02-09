/**
 * 前端集成模块
 * 包含前端管理、Web集成等功能
 */

pub mod manager;
pub mod starter;
pub mod web;
pub mod agent_integration;

// 重新导出常用类型
pub use manager::P2PFrontendManager;
pub use starter::P2PFrontendStarter;
pub use agent_integration::{AgentSession, AgentSessionConfig, AgentMode, ChatMessage, MessageType, AgentFrontendManager};

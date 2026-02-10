//! Ralph Loop 模块
//!
//! Ralph Loop 闭环循环流的核心模块，包含AI自动决策、智能重试、文档调研等功能
//!
//! # 文档驱动的自主工作流
//!
//! 新增功能：让AI通过阅读文档自主完成任务
//!
//! ## 使用方式
//!
//! ```rust
//! use williw::agent::workflow::ralph_loop::{DocumentDrivenConfig, use_default_docs};
//!
//! let config = DocumentDrivenConfig {
//!     use_embedded_docs: true,  // 使用内嵌文档
//!     ..Default::default()
//! };
//!
//! executor.run_document_driven_workflow(
//!     "execution_id".to_string(),
//!     config,
//!     api_key,
//!     ralph_config,
//! ).await?;
//! ```

pub mod core;
pub mod ai_decision;
pub mod research;
pub mod learning;
pub mod retry_strategy;
pub mod auto_environment;
pub mod document_driven;
pub mod docs;

// 重新导出主要类型和功能
pub use core::*;
pub use ai_decision::*;
pub use research::*;
pub use learning::*;
pub use retry_strategy::*;
pub use auto_environment::*;
pub use document_driven::*;
pub use docs::*;

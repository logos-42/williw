//! Task 模块
//!
//! 支持三种执行模式：sequential（顺序）、parallel（并行）、swarm（多智能体协作）

pub mod manifest;
pub mod executor;
pub mod skills_loader;
pub mod commands;

pub use manifest::*;
pub use executor::*;
pub use skills_loader::*;
pub use commands::*;

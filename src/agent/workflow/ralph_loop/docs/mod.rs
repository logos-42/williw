//! 内嵌文档模块
//!
//! 包含AI身份、任务和工具使用说明文档

/// 默认身份文档 - 去中心化算力专家
pub const IDENTITY_COMPUTE_EXPERT: &str = include_str!("agents/compute_expert.md");

/// 系统配置专家身份文档
pub const IDENTITY_SYSTEM_CONFIG_EXPERT: &str = include_str!("agents/system_config_expert.md");

/// 默认任务文档 - 模型切分
pub const TASK_SPLIT_MODEL_EXAMPLE: &str = include_str!("tasks/split_model_example.md");

/// 系统配置任务文档
pub const TASK_SYSTEM_CONFIG: &str = include_str!("tasks/system_config.md");

/// 工具文档
pub const TOOL_DECENTRALIZED_MODEL: &str = include_str!("tools/DecentralizedModel.md");

/// 默认文档路径
pub const DEFAULT_IDENTITY_PATH: &str =
    "src/agent/workflow/ralph_loop/docs/agents/compute_expert.md";
pub const DEFAULT_TASK_PATH: &str =
    "src/agent/workflow/ralph_loop/docs/tasks/split_model_example.md";

//! Task 清单定义
//!
//! 支持三种执行模式：sequential（顺序）、parallel（并行）、swarm（多智能体协作）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task 执行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskExecutionMode {
    /// 顺序执行
    Sequential,
    /// 并行执行
    Parallel,
    /// Agent Swarm（多智能体协作）
    Swarm,
}

/// Task 清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    /// Task ID
    pub id: String,
    /// 显示名称
    pub display_name: String,
    /// Task 描述
    pub description: String,
    /// 执行模式
    pub execution_mode: TaskExecutionMode,
    /// 版本号
    pub version: String,
    /// 输入参数模式
    pub input_schema: serde_json::Value,
    /// 输出结果模式
    pub output_schema: serde_json::Value,
    /// 标签
    pub tags: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// Task 步骤（用于 sequential/parallel）
    pub steps: Vec<TaskStep>,
    /// Swarm 配置（用于 swarm 模式）
    pub swarm_config: Option<SwarmConfig>,
}

/// Task 步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// 步骤ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 使用的工具
    pub tool: String,
    /// 使用的 Skill
    pub skill: Option<String>,
    /// 依赖步骤
    pub depends_on: Vec<String>,
    /// 是否可并行
    pub parallelizable: bool,
    /// 超时时间（秒）
    pub timeout: u64,
}

/// Swarm 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Agent 数量
    pub agent_count: usize,
    /// 协作策略
    pub strategy: SwarmStrategy,
    /// Leader Agent 提示词
    pub leader_prompt: Option<String>,
    /// Worker Agent 提示词
    pub worker_prompt: Option<String>,
    /// 角色分配
    pub roles: HashMap<String, String>,
}

/// Swarm 协作策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmStrategy {
    /// 广播：所有 Agent 收到相同任务
    Broadcast,
    /// 分片：将任务拆分给不同 Agent
    Shard,
    /// 投票：Agent 投票决策
    Vote,
    /// 层级：Leader + Workers
    Hierarchical,
}

/// Task 执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 等待中
    Pending,
    /// 运行中
    Running,
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 取消
    Cancelled,
}

/// Task 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// 执行状态
    pub status: TaskStatus,
    /// 开始时间
    pub started_at: i64,
    /// 结束时间
    pub finished_at: Option<i64>,
    /// 输出结果
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 子任务结果（用于 parallel/swarm）
    pub subtask_results: Vec<SubTaskResult>,
}

/// 子任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskResult {
    /// 子任务ID
    pub subtask_id: String,
    /// Agent ID（用于 swarm）
    pub agent_id: Option<String>,
    /// 执行状态
    pub status: TaskStatus,
    /// 输出
    pub output: Option<serde_json::Value>,
    /// 错误
    pub error: Option<String>,
}

impl TaskManifest {
    /// 创建简单的顺序任务
    pub fn sequential(id: &str, display_name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            description: description.to_string(),
            execution_mode: TaskExecutionMode::Sequential,
            version: "1.0.0".to_string(),
            input_schema: serde_json::json!({}),
            output_schema: serde_json::json!({}),
            tags: vec![],
            enabled: true,
            steps: vec![],
            swarm_config: None,
        }
    }

    /// 创建并行任务
    pub fn parallel(id: &str, display_name: &str, description: &str) -> Self {
        Self {
            execution_mode: TaskExecutionMode::Parallel,
            ..Self::sequential(id, display_name, description)
        }
    }

    /// 创建 Swarm 任务
    pub fn swarm(id: &str, display_name: &str, description: &str, agent_count: usize) -> Self {
        Self {
            execution_mode: TaskExecutionMode::Swarm,
            swarm_config: Some(SwarmConfig {
                agent_count,
                strategy: SwarmStrategy::Hierarchical,
                leader_prompt: None,
                worker_prompt: None,
                roles: HashMap::new(),
            }),
            ..Self::sequential(id, display_name, description)
        }
    }
}

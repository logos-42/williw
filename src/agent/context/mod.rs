//! Agent 上下文管理模块
//!
//! 管理 Agent 的执行上下文、状态和环境

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;

/// 上下文条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    /// 条目ID
    pub id: String,
    /// 条目类型
    pub entry_type: ContextType,
    /// 内容
    pub content: String,
    /// 重要性评分 (0-10)
    pub importance: u8,
    /// 时间戳
    pub timestamp: i64,
    /// 关联的任务ID
    pub task_id: Option<String>,
}

/// 上下文类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    /// 输入
    Input,
    /// 输出
    Output,
    /// 工具调用
    ToolCall,
    /// 工具结果
    ToolResult,
    /// 错误
    Error,
    /// 状态更新
    StatusUpdate,
    /// 决策
    Decision,
    /// 学习总结
    LearningSummary,
}

impl ContextType {
    /// 获取上下文类型的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextType::Input => "INPUT",
            ContextType::Output => "OUTPUT",
            ContextType::ToolCall => "TOOL_CALL",
            ContextType::ToolResult => "TOOL_RESULT",
            ContextType::Error => "ERROR",
            ContextType::StatusUpdate => "STATUS_UPDATE",
            ContextType::Decision => "DECISION",
            ContextType::LearningSummary => "LEARNING_SUMMARY",
        }
    }
}

/// Agent 执行上下文
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// 会话ID
    pub session_id: String,
    /// Agent 名称
    pub agent_name: String,
    /// 执行环境
    pub environment: HashMap<String, String>,
    /// 工作目录
    pub working_dir: String,
    /// 配置参数
    pub config: AgentConfig,
    /// 状态存储
    pub state: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// 工具访问权限
    pub tool_permissions: Vec<String>,
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 最大执行时间（秒）
    pub max_execution_time: u64,
    /// 内存限制（MB）
    pub memory_limit_mb: usize,
    /// 并发限制
    pub max_concurrent_tasks: usize,
    /// 日志级别
    pub log_level: String,
    /// 安全模式
    pub secure_mode: bool,
    /// 调试模式
    pub debug_mode: bool,
    /// 缓存设置
    pub cache_settings: CacheSettings,
}

/// 缓存设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    /// 启用缓存
    pub enabled: bool,
    /// 缓存大小限制（MB）
    pub size_limit_mb: usize,
    /// 缓存过期时间（秒）
    pub ttl_seconds: u64,
    /// 缓存目录
    pub cache_dir: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_execution_time: 300, // 5分钟
            memory_limit_mb: 1024,   // 1GB
            max_concurrent_tasks: 5,
            log_level: "info".to_string(),
            secure_mode: true,
            debug_mode: false,
            cache_settings: CacheSettings::default(),
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            size_limit_mb: 100,
            ttl_seconds: 3600, // 1小时
            cache_dir: ".alou/cache".to_string(),
        }
    }
}

impl AgentContext {
    /// 创建新的 Agent 上下文
    pub fn new(session_id: String, agent_name: String) -> Self {
        Self {
            session_id,
            agent_name,
            environment: std::env::vars().collect(),
            working_dir: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            config: AgentConfig::default(),
            state: Arc::new(RwLock::new(HashMap::new())),
            tool_permissions: vec!["read".to_string(), "write".to_string(), "execute".to_string()],
        }
    }

    /// 获取状态值
    pub async fn get_state(&self, key: &str) -> Option<serde_json::Value> {
        let state = self.state.read().await;
        state.get(key).cloned()
    }

    /// 设置状态值
    pub async fn set_state(&self, key: String, value: serde_json::Value) {
        let mut state = self.state.write().await;
        state.insert(key, value);
    }

    /// 更新环境变量
    pub fn update_environment(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }

    /// 检查工具权限
    pub fn has_permission(&self, permission: &str) -> bool {
        self.tool_permissions.iter().any(|p| p == permission)
    }
}

/// 上下文管理器
pub struct ContextManager {
    contexts: Arc<RwLock<HashMap<String, AgentContext>>>,
}

impl ContextManager {
    /// 创建新的上下文管理器
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建新的上下文
    pub async fn create_context(&self, session_id: String, agent_name: String) -> AgentContext {
        let context = AgentContext::new(session_id.clone(), agent_name);
        let mut contexts = self.contexts.write().await;
        contexts.insert(session_id, context.clone());
        context
    }

    /// 获取上下文
    pub async fn get_context(&self, session_id: &str) -> Option<AgentContext> {
        let contexts = self.contexts.read().await;
        contexts.get(session_id).cloned()
    }

    /// 删除上下文
    pub async fn remove_context(&self, session_id: &str) -> Option<AgentContext> {
        let mut contexts = self.contexts.write().await;
        contexts.remove(session_id)
    }

    /// 获取所有上下文ID
    pub async fn get_all_context_ids(&self) -> Vec<String> {
        let contexts = self.contexts.read().await;
        contexts.keys().cloned().collect()
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
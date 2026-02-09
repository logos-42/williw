//! Agent 提示词管理模块
//!
//! 管理 Agent 的提示词模板、对话历史和上下文构建

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod layered_prompts;
pub use layered_prompts::*;

mod ai_workflow_prompts;
pub use ai_workflow_prompts::*;

/// 提示词模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 模板名称
    pub name: String,
    /// 模板内容
    pub template: String,
    /// 描述
    pub description: String,
    /// 参数列表
    pub parameters: Vec<String>,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 提示词管理器
pub struct PromptManager {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptManager {
    /// 创建新的提示词管理器
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// 注册提示词模板
    pub fn register_template(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// 获取提示词模板
    pub fn get_template(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// 渲染提示词模板
    pub fn render_template(&self, name: &str, params: &HashMap<String, String>) -> Option<String> {
        if let Some(template) = self.get_template(name) {
            let mut result = template.template.clone();
            
            for (param, value) in params {
                let placeholder = format!("{{{}}}", param);
                result = result.replace(&placeholder, value);
            }
            
            Some(result)
        } else {
            None
        }
    }

    /// 获取所有模板名称
    pub fn get_template_names(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

// 全局提示词管理器实例
use std::sync::Once;
use tokio::sync::RwLock;
use std::sync::Arc;

static mut PROMPT_MANAGER_INSTANCE: Option<Arc<RwLock<PromptManager>>> = None;
static PROMPT_MANAGER_ONCE: Once = Once::new();

pub fn get_prompt_manager() -> Arc<RwLock<PromptManager>> {
    PROMPT_MANAGER_ONCE.call_once(|| unsafe {
        PROMPT_MANAGER_INSTANCE = Some(Arc::new(RwLock::new(PromptManager::default().with_defaults())));
    });
    unsafe {
        PROMPT_MANAGER_INSTANCE.as_ref().unwrap().clone()
    }
}

// 为兼容性提供别名
pub use get_prompt_manager as PROMPT_MANAGER;

// 预定义的提示词模板
impl PromptManager {
    /// 添加默认的提示词模板
    pub fn with_defaults(mut self) -> Self {
        // Agent 技能执行提示词
        self.register_template(PromptTemplate {
            name: "skill_execution".to_string(),
            template: r#"你是一个技能执行助手。你的任务是根据提供的技能定义执行特定任务。
            
技能名称: {skill_name}
技能描述: {skill_description}
输入参数: {input_params}

请按照技能定义执行相应操作，并返回结果。"#.to_string(),
            description: "技能执行提示词模板".to_string(),
            parameters: vec!["skill_name".to_string(), "skill_description".to_string(), "input_params".to_string()],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        });

        // 工作流执行提示词
        self.register_template(PromptTemplate {
            name: "workflow_execution".to_string(),
            template: r#"你是一个工作流执行助手。你的任务是协调和执行工作流中的各个步骤。
            
工作流名称: {workflow_name}
工作流描述: {workflow_description}
当前步骤: {current_step}
上下文信息: {context_info}

请执行当前步骤并准备下一步。"#.to_string(),
            description: "工作流执行提示词模板".to_string(),
            parameters: vec!["workflow_name".to_string(), "workflow_description".to_string(), 
                            "current_step".to_string(), "context_info".to_string()],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        });

        // 依赖安装提示词
        self.register_template(PromptTemplate {
            name: "dependency_installation".to_string(),
            template: r#"你是一个依赖管理助手。你的任务是安装和配置项目所需的依赖项。
            
项目类型: {project_type}
依赖列表: {dependencies}
平台信息: {platform_info}
配置要求: {configuration_requirements}

请安装必要的依赖项并配置环境。"#.to_string(),
            description: "依赖安装提示词模板".to_string(),
            parameters: vec!["project_type".to_string(), "dependencies".to_string(), 
                            "platform_info".to_string(), "configuration_requirements".to_string()],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        });

        // 系统稳定性检查提示词
        self.register_template(PromptTemplate {
            name: "stability_check".to_string(),
            template: r#"你是一个系统稳定性检查助手。你的任务是监控和维护系统的稳定性。
            
检查类型: {check_type}
监控指标: {metrics}
异常情况: {anomalies}
恢复策略: {recovery_strategies}

请执行稳定性检查并采取必要措施。"#.to_string(),
            description: "系统稳定性检查提示词模板".to_string(),
            parameters: vec!["check_type".to_string(), "metrics".to_string(), 
                            "anomalies".to_string(), "recovery_strategies".to_string()],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        });

        self
    }
}

/// 对话历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    /// 会话ID
    pub session_id: String,
    /// 历史记录
    pub messages: Vec<ConversationMessage>,
    /// 上下文摘要
    pub context_summary: String,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// 消息ID
    pub id: String,
    /// 发送者
    pub sender: String,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: i64,
    /// 消息类型
    pub message_type: MessageType,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// 用户输入
    UserInput,
    /// 系统消息
    SystemMessage,
    /// 工具输出
    ToolOutput,
    /// 错误消息
    ErrorMessage,
    /// 状态更新
    StatusUpdate,
}

impl ConversationHistory {
    /// 创建新的对话历史
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            messages: Vec::new(),
            context_summary: String::new(),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, message: ConversationMessage) {
        self.messages.push(message);
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// 获取最后几条消息
    pub fn get_recent_messages(&self, count: usize) -> Vec<&ConversationMessage> {
        self.messages.iter().rev().take(count).rev().collect()
    }

    /// 更新上下文摘要
    pub fn update_context_summary(&mut self, summary: String) {
        self.context_summary = summary;
        self.updated_at = chrono::Utc::now().timestamp();
    }
}
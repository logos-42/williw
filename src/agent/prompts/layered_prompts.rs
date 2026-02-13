//! 分层提示词工程模块
//!
//! 实现分层提示词系统，避免上下文腐烂，支持循环执行直到任务完成

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use chrono::Utc;

/// 提示词层级枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptLayer {
    /// 系统层 - 定义基本角色和行为准则
    System,
    /// 任务层 - 定义当前任务和目标
    Task,
    /// 上下文层 - 提供当前上下文信息
    Context,
    /// 工具层 - 提供可用工具信息
    Tools,
    /// 历史层 - 提供关键历史信息摘要
    History,
    /// 输出层 - 定义期望的输出格式
    Output,
}

/// 分层提示词结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredPrompt {
    /// 提示词ID
    pub id: String,
    /// 层级
    pub layer: PromptLayer,
    /// 提示词内容
    pub content: String,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
    /// 有效期（秒）
    pub ttl: Option<u64>,
    /// 优先级
    pub priority: u8,
}

/// 分层提示词管理器
pub struct LayeredPromptManager {
    layers: HashMap<String, Vec<LayeredPrompt>>,
    global_context: Arc<RwLock<GlobalContext>>,
}

/// 全局上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalContext {
    /// 当前任务ID
    pub current_task_id: Option<String>,
    /// 任务历史摘要
    pub task_history_summary: String,
    /// 当前上下文窗口
    pub context_window: Vec<ContextEntry>,
    /// 上下文窗口最大大小
    pub max_context_size: usize,
    /// 上下文压缩阈值
    pub compression_threshold: usize,
    /// 创建时间
    pub created_at: i64,
}

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

impl GlobalContext {
    /// 创建新的全局上下文
    pub fn new() -> Self {
        Self {
            current_task_id: None,
            task_history_summary: String::new(),
            context_window: Vec::new(),
            max_context_size: 50, // 默认最大50个条目
            compression_threshold: 30, // 30个条目开始压缩
            created_at: Utc::now().timestamp(),
        }
    }

    /// 添加上下文条目
    pub fn add_entry(&mut self, entry: ContextEntry) {
        self.context_window.push(entry);
        
        // 如果超过最大大小，移除最不重要的条目
        if self.context_window.len() > self.max_context_size {
            self.compress_context();
        }
        
        // 更新时间戳
        self.created_at = Utc::now().timestamp();
    }

    /// 压缩上下文
    fn compress_context(&mut self) {
        // 按重要性排序，保留最重要的条目
        self.context_window.sort_by(|a, b| b.importance.cmp(&a.importance));
        
        // 移除最不重要的条目，保留前max_context_size个
        if self.context_window.len() > self.max_context_size {
            self.context_window.truncate(self.max_context_size);
        }
    }

    /// 获取上下文摘要
    pub fn get_context_summary(&self) -> String {
        let mut summary = String::new();
        
        // 只包含重要性大于5的条目
        for entry in &self.context_window {
            if entry.importance > 5 {
                summary.push_str(&format!("[{}] {}: {}\n", 
                    entry.entry_type.as_str(), 
                    entry.timestamp, 
                    entry.content
                ));
            }
        }
        
        summary
    }
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

impl LayeredPromptManager {
    /// 创建新的分层提示词管理器
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
            global_context: Arc::new(RwLock::new(GlobalContext::new())),
        }
    }

    /// 添加提示词到指定层级
    pub fn add_prompt_to_layer(&mut self, layer: PromptLayer, prompt: LayeredPrompt) {
        let layer_key = format!("{:?}", layer);
        self.layers.entry(layer_key).or_insert_with(Vec::new).push(prompt);
    }

    /// 获取指定层级的所有提示词
    pub async fn get_prompts_for_layer(&self, layer: PromptLayer) -> Vec<LayeredPrompt> {
        let layer_key = format!("{:?}", layer);
        self.layers.get(&layer_key)
            .map(|prompts| prompts.clone())
            .unwrap_or_default()
    }

    /// 构建完整的分层提示词
    pub async fn build_layered_prompt(&self, task_description: &str) -> String {
        let mut prompt = String::new();
        
        // 按优先级顺序添加各层提示词
        let layers = [
            PromptLayer::System,
            PromptLayer::Task,
            PromptLayer::Context,
            PromptLayer::Tools,
            PromptLayer::History,
            PromptLayer::Output,
        ];
        
        for layer in &layers {
            let layer_prompts = self.get_prompts_for_layer(layer.clone()).await;
            
            if !layer_prompts.is_empty() {
                prompt.push_str(&format!("\n=== {:?} LAYER ===\n", layer));
                
                for p in &layer_prompts {
                    if let Some(expired) = p.ttl {
                        if Utc::now().timestamp() - p.created_at > expired as i64 {
                            continue; // 跳过过期的提示词
                        }
                    }
                    prompt.push_str(&p.content);
                    prompt.push('\n');
                }
            }
        }
        
        // 添加当前任务描述
        prompt.push_str(&format!("\n=== CURRENT TASK ===\n{}\n", task_description));
        
        // 添加当前上下文
        {
            let context = self.global_context.read().await;
            let context_summary = context.get_context_summary();
            if !context_summary.is_empty() {
                prompt.push_str(&format!("\n=== RELEVANT CONTEXT ===\n{}\n", context_summary));
            }
        }
        
        prompt
    }

    /// 更新全局上下文
    pub async fn update_global_context(&self, entry: ContextEntry) {
        let mut context = self.global_context.write().await;
        context.add_entry(entry);
    }

    /// 获取全局上下文的克隆
    pub async fn get_global_context(&self) -> GlobalContext {
        self.global_context.read().await.clone()
    }

    /// 设置当前任务ID
    pub async fn set_current_task_id(&self, task_id: String) {
        let mut context = self.global_context.write().await;
        context.current_task_id = Some(task_id);
    }

    /// 获取当前任务ID
    pub async fn get_current_task_id(&self) -> Option<String> {
        let context = self.global_context.read().await;
        context.current_task_id.clone()
    }
}

impl Default for LayeredPromptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 预定义的分层提示词模板
impl LayeredPromptManager {
    /// 添加默认的分层提示词
    pub fn with_defaults(mut self) -> Self {
        // 系统层 - 定义AI助手的基本角色
        self.add_prompt_to_layer(PromptLayer::System, LayeredPrompt {
            id: "system-role-definition".to_string(),
            layer: PromptLayer::System,
            content: r#"你是Williw AI助手，一个专业的去中心化训练系统助手。
你的职责：
1. 帮助用户管理去中心化训练任务
2. 协调各种工具和资源
3. 确保任务高效完成
4. 提供准确的技术建议
5. 维护系统稳定性和安全性

行为准则：
- 始终保持专业和礼貌
- 提供准确和可靠的信息
- 优先考虑系统安全
- 在不确定时寻求澄清
- 逐步解决问题"#.to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            ttl: None,
            priority: 10,
        });

        // 任务层 - 定义任务执行框架
        self.add_prompt_to_layer(PromptLayer::Task, LayeredPrompt {
            id: "task-execution-framework".to_string(),
            layer: PromptLayer::Task,
            content: r#"任务执行框架：
1. 分析任务需求和约束
2. 规划执行步骤
3. 选择合适工具
4. 执行并监控进度
5. 验证结果
6. 如未完成则循环执行
7. 记录学习和改进点

循环执行规则：
- 每次迭代后评估完成度
- 根据反馈调整策略
- 达到完成条件时停止
- 遇到错误时应用重试策略"#.to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            ttl: None,
            priority: 9,
        });

        // 工具层 - 提供可用工具信息
        self.add_prompt_to_layer(PromptLayer::Tools, LayeredPrompt {
            id: "available-tools-info".to_string(),
            layer: PromptLayer::Tools,
            content: r#"可用工具列表：
- 文件系统工具：读写文件、目录操作
- 网络工具：HTTP请求、DNS查询、ping
- 系统工具：硬件检测、进程管理
- Bash工具：命令执行
- 计划工具：任务规划和管理
- 待办工具：任务列表管理
- Agent技能工具：AI技能执行
- Agent协作工具：多Agent协作
- 工具创建工具：动态创建新工具
- Iroh通讯工具：P2P网络通讯

使用工具时：
1. 选择最适合的工具
2. 提供必要的参数
3. 处理工具返回结果
4. 根据结果决定下一步行动"#.to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            ttl: Some(3600), // 1小时后过期
            priority: 8,
        });

        // 输出层 - 定义输出格式
        self.add_prompt_to_layer(PromptLayer::Output, LayeredPrompt {
            id: "output-format-specification".to_string(),
            layer: PromptLayer::Output,
            content: r#"输出格式要求：
1. 结构化JSON响应
2. 包含状态、结果和错误信息
3. 提供执行时间和性能指标
4. 包含上下文和警告信息

JSON格式：
{
  "status": "success|error|partial",
  "result": {...},
  "error": "错误信息（如果有）",
  "execution_time_ms": 123,
  "context": {...},
  "warnings": [...]
}"#.to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            ttl: None,
            priority: 7,
        });

        self
    }
}

/// 分层提示词执行器
pub struct LayeredPromptExecutor {
    manager: Arc<RwLock<LayeredPromptManager>>,
}

impl LayeredPromptExecutor {
    /// 创建新的分层提示词执行器
    pub fn new(manager: Arc<RwLock<LayeredPromptManager>>) -> Self {
        Self { manager }
    }

    /// 执行分层提示词循环直到任务完成
    pub async fn execute_until_complete(
        &self,
        task_description: &str,
        completion_condition: impl Fn(&serde_json::Value) -> bool + Send + Sync + 'static,
        max_iterations: usize,
        iteration_delay_ms: u64,
    ) -> Result<serde_json::Value, String> {
        let mut iteration = 0;
        
        while iteration < max_iterations {
            iteration += 1;
            
            println!("🔄 [LAYERED-PROMPT] Iteration {} for task: {}", 
                     iteration, 
                     task_description.chars().take(50).collect::<String>());
            
            // 构建当前迭代的提示词
            let prompt = self.manager.read().await.build_layered_prompt(task_description).await;
            
            // 执行提示词（这里应该调用实际的AI模型，但现在我们模拟）
            let result = self.execute_prompt_iteration(&prompt, iteration).await?;
            
            // 检查完成条件
            if completion_condition(&result) {
                println!("✅ [LAYERED-PROMPT] Task completed at iteration {}", iteration);
                
                // 记录完成信息到上下文
                self.manager.write().await.update_global_context(ContextEntry {
                    id: format!("completion-{}", Utc::now().timestamp()),
                    entry_type: ContextType::StatusUpdate,
                    content: format!("Task completed at iteration {}", iteration),
                    importance: 10,
                    timestamp: Utc::now().timestamp(),
                    task_id: self.manager.read().await.get_current_task_id().await,
                }).await;
                
                return Ok(result);
            }
            
            // 记录迭代结果到上下文
            self.manager.write().await.update_global_context(ContextEntry {
                id: format!("iteration-result-{}", iteration),
                entry_type: ContextType::Output,
                content: format!("Iteration {} result: {:?}", iteration, result),
                importance: 7,
                timestamp: Utc::now().timestamp(),
                task_id: self.manager.read().await.get_current_task_id().await,
            }).await;
            
            // 如果不是最后一次迭代，等待一段时间
            if iteration < max_iterations {
                tokio::time::sleep(tokio::time::Duration::from_millis(iteration_delay_ms)).await;
            }
        }
        
        // 达到最大迭代次数仍未完成
        let _final_result = self.execute_prompt_iteration(
            &format!("{}\n\n注意：已达到最大迭代次数({})仍未完成任务，请提供当前进度和建议", 
                    task_description, max_iterations),
            iteration
        ).await?;
        
        // 记录未完成信息
        self.manager.write().await.update_global_context(ContextEntry {
            id: format!("max-iterations-reached-{}", Utc::now().timestamp()),
            entry_type: ContextType::StatusUpdate,
            content: format!("Reached max iterations ({}) without completing task", max_iterations),
            importance: 8,
            timestamp: Utc::now().timestamp(),
            task_id: self.manager.read().await.get_current_task_id().await,
        }).await;
        
        Err(format!("Max iterations ({}) reached without completing task", max_iterations))
    }

    /// 执行单次提示词迭代
    async fn execute_prompt_iteration(&self, prompt: &str, iteration: usize) -> Result<serde_json::Value, String> {
        // 这里应该实际调用AI模型，但现在我们返回模拟结果
        // 在实际实现中，这里会调用OpenAI API或其他LLM服务
        
        println!("📝 [LAYERED-PROMPT] Executing iteration {} with prompt length: {} chars", 
                 iteration, prompt.len());
        
        // 模拟AI响应
        Ok(serde_json::json!({
            "iteration": iteration,
            "status": "processing",
            "message": format!("Processed iteration {} of layered prompt", iteration),
            "prompt_length": prompt.len(),
            "timestamp": Utc::now().timestamp(),
            "context_snapshot": self.manager.read().await.get_global_context().await.get_context_summary()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_layered_prompt_manager() {
        let mut manager = LayeredPromptManager::new().with_defaults();
        
        // 测试添加自定义提示词
        manager.add_prompt_to_layer(PromptLayer::Context, LayeredPrompt {
            id: "test-context".to_string(),
            layer: PromptLayer::Context,
            content: "Test context layer".to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            ttl: None,
            priority: 5,
        });
        
        // 测试获取提示词
        let context_prompts = manager.get_prompts_for_layer(PromptLayer::Context).await;
        assert_eq!(context_prompts.len(), 1);
        assert_eq!(context_prompts[0].content, "Test context layer");
        
        // 测试构建完整提示词
        let full_prompt = manager.build_layered_prompt("Test task").await;
        assert!(full_prompt.contains("Test task"));
        assert!(full_prompt.contains("SYSTEM LAYER"));
        assert!(full_prompt.contains("TASK LAYER"));
    }

    #[tokio::test]
    async fn test_global_context_management() {
        let manager = LayeredPromptManager::new();
        
        // 添加上下文条目
        manager.update_global_context(ContextEntry {
            id: "test-entry".to_string(),
            entry_type: ContextType::Input,
            content: "Test input".to_string(),
            importance: 8,
            timestamp: Utc::now().timestamp(),
            task_id: Some("test-task".to_string()),
        }).await;
        
        let context = manager.get_global_context().await;
        assert_eq!(context.context_window.len(), 1);
        assert!(context.get_context_summary().contains("Test input"));
    }
}
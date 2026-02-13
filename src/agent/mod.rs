// Agent 模块
// 负责大模型配置、依赖管理和系统稳定性维护

#![allow(ambiguous_glob_reexports)]

pub mod tools;
pub mod workflow;
pub mod memory_manager;
pub mod utils;
pub mod context;
pub mod prompts;
pub mod skills;
pub mod bridges;
pub mod compute;
pub mod setup;

// 导出主要类型和函数
pub use tools::*;
pub use workflow::*;
pub use memory_manager::*;
pub use utils::*;
pub use context::*;
pub use prompts::*;
pub use skills::*;
pub use setup::*;

/// Agent 状态管理器
pub struct AgentManager {
    /// 工作流管理器
    pub workflow_manager: crate::agent::workflow::WorkflowState,
    /// 工具管理器
    pub tool_manager: crate::agent::tools::ToolRegistry,
    /// 内存管理器
    pub memory_manager: crate::agent::memory_manager::MemoryManager,
    /// 上下文管理器
    pub context_manager: crate::agent::context::ContextManager,
    /// 提示词管理器
    pub prompt_manager: crate::agent::prompts::PromptManager,
}

impl AgentManager {
    /// 创建新的 Agent 管理器
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            workflow_manager: crate::agent::workflow::WorkflowState::default(),
            tool_manager: crate::agent::tools::ToolRegistry::new(),
            memory_manager: crate::agent::memory_manager::MemoryManager::new(1000, 50), // 1000 items, 50MB
            context_manager: crate::agent::context::ContextManager::new(),
            prompt_manager: crate::agent::prompts::PromptManager::default().with_defaults(),
        })
    }

    /// 初始化 Agent 系统
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing Agent Manager...");

        // 初始化工作流管理器
        println!("✓ Workflow manager initialized");

        // 初始化工具管理器 - 使用 initialize_tools 函数
        let _ = crate::agent::tools::initialize_tools().await?;
        println!("✓ Tool manager initialized");

        // 初始化内存管理器
        self.memory_manager.initialize()?;
        println!("✓ Memory manager initialized");

        // 初始化上下文管理器
        println!("✓ Context manager initialized");

        // 初始化提示词管理器
        println!("✓ Prompt manager initialized");

        println!("Agent Manager initialization complete!");
        Ok(())
    }

    /// 运行 Agent 系统
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting Agent Manager...");

        // 这里可以启动后台任务，如监控、调度等

        Ok(())
    }
}
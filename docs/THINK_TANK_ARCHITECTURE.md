# 智囊团切分工具架构设计

## 1. 概述

本文档描述智囊团(Think Tank)切分工具的完整架构设计，实现AI自动化工作流、去中心化算力共享和智能体分工协作。

### 核心目标

- **自我复制**: Agent能够根据任务需求分裂为多个子Agent
- **自动配置**: Agent自己配置运行环境、安装依赖
- **去中心化**: 使用Iroh进行节点间P2P通讯和算力共享
- **动态调度**: 根据任务负载动态分配计算资源

## 2. 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      Think Tank Coordinator                      │
│                    (Ralph Loop Controller)                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Agent       │  │ Agent       │  │ Agent       │             │
│  │ Splitter    │  │ Configurator│  │ Scheduler   │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Prompt      │  │ Workflow    │  │ Iroh P2P    │             │
│  │ Generator   │  │ Executor    │  │ Network     │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
├─────────────────────────────────────────────────────────────────┤
│                      Shared State Layer                          │
│              (Context, Memory, Resources)                        │
└─────────────────────────────────────────────────────────────────┘
```

## 3. 核心组件设计

### 3.1 Agent分裂器 (Agent Splitter)

```rust
// src/agent/think_tank/agent_splitter.rs

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use chrono::Utc;

/// 智囊团任务类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThinkTankTaskType {
    /// 训练任务
    Training,
    /// 推理任务
    Inference,
    /// 数据处理
    DataProcessing,
    /// 模型分发
    ModelDistribution,
    /// 混合任务
    Hybrid,
}

/// 子智囊团配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubThinkTankConfig {
    pub id: String,
    pub parent_id: String,
    pub task_type: ThinkTankTaskType,
    pub role: String,
    pub capabilities: Vec<String>,
    pub resources: ResourceQuota,
    pub priority: u8,
    pub iroh_node_id: String,
    pub lifecycle: LifecycleConfig,
}

/// 资源配额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_memory_mb: u64,
    pub max_cpu_cores: u64,
    pub max_gpu_memory_mb: Option<u64>,
    pub max_storage_mb: u64,
    pub network_bandwidth_mbps: Option<u64>,
}

/// 生命周期配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    pub auto_start: bool,
    pub auto_stop: bool,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: Option<u64>,
}

/// Agent分裂器
pub struct AgentSplitter {
    /// 子智囊团注册表
    sub_agents: Arc<RwLock<HashMap<String, SubThinkTankConfig>>>,
    /// 任务分发器
    task_dispatcher: Arc<Mutex<TaskDispatcher>>,
    /// Iroh网络管理器
    iroh_manager: Arc<Mutex<Option<IrohConnectionManager>>>,
    /// 当前活跃节点数
    active_node_count: Arc<Mutex<u32>>,
}

impl AgentSplitter {
    /// 创建新的分裂器
    pub async fn new(iroh_manager: Arc<Mutex<Option<IrohConnectionManager>>>) -> Self {
        Self {
            sub_agents: Arc::new(RwLock::new(HashMap::new())),
            task_dispatcher: Arc::new(Mutex::new(TaskDispatcher::new())),
            iroh_manager,
            active_node_count: Arc::new(Mutex::new(1)),
        }
    }

    /// 根据任务需求分裂智囊团
    pub async fn split_think_tank(
        &self,
        task_requirements: TaskRequirements,
        parent_agent: &AgentContext,
    ) -> Result<Vec<SubThinkTankConfig>, SplitError> {
        println!("🔀 [AGENT-SPLITTER] 开始分裂智囊团");

        // 1. 分析任务需求，确定分裂策略
        let split_strategy = self.analyze_task_requirements(&task_requirements).await?;

        // 2. 根据策略生成分裂计划
        let split_plan = self.generate_split_plan(&split_strategy, parent_agent).await?;

        // 3. 创建子智囊团配置
        let mut sub_agents = Vec::new();
        for (index, agent_spec) in split_plan.agents.into_iter().enumerate() {
            let sub_config = SubThinkTankConfig {
                id: format!("{}_sub_{}", parent_agent.id, index),
                parent_id: parent_agent.id.clone(),
                task_type: agent_spec.task_type,
                role: agent_spec.role,
                capabilities: agent_spec.capabilities,
                resources: agent_spec.resources,
                priority: agent_spec.priority,
                iroh_node_id: self.get_or_create_node_id().await?,
                lifecycle: LifecycleConfig {
                    auto_start: true,
                    auto_stop: true,
                    idle_timeout_seconds: 300,
                    max_lifetime_seconds: None,
                },
            };
            sub_agents.push(sub_config);
        }

        // 4. 注册子智囊团
        let mut registry = self.sub_agents.write().await;
        for sub_agent in &sub_agents {
            registry.insert(sub_agent.id.clone(), sub_agent.clone());
        }

        println!("✅ [AGENT-SPLITTER] 成功创建 {} 个子智囊团", sub_agents.len());
        Ok(sub_agents)
    }

    /// 分析任务需求
    async fn analyze_task_requirements(
        &self,
        requirements: &TaskRequirements,
    ) -> Result<SplitStrategy, SplitError> {
        // 使用AI决策来确定最佳分裂策略
        let analysis_prompt = format!(
            r#"
分析以下任务需求，确定智囊团最佳分裂策略：

任务类型: {:?}", requirements.task_type
任务规模: {:?}", requirements.scale
优先级: {}
截止时间: {:?}", requirements.priority, requirements.deadline

可用资源:
- CPU核心数: {}
- GPU: {:?}", requirements.available_resources.cpu_cores,
            requirements.available_resources.gpu_info

请返回JSON格式的分裂策略：
{{
    "num_agents": 整数,
    "agent_specs": [{{"role": "角色名", "task_type": "任务类型", "resources": {{...}}}}]
}}
"#,
            requirements.task_type,
            requirements.scale,
            requirements.priority,
            requirements.deadline,
            requirements.available_resources.cpu_cores,
            requirements.available_resources.gpu_info
        );

        // 调用AI分析...
        unimplemented!()
    }
}
```

### 3.2 AI自动化环境配置器

```rust
// src/agent/think_tank/environment_configurator.rs

/// 环境配置器 - Agent自己配置运行环境
pub struct EnvironmentConfigurator {
    /// 系统信息
    system_info: SystemInfo,
    /// 环境检测器
    detectors: Vec<Box<dyn EnvironmentDetector>>,
    /// 配置模板库
    config_templates: ConfigTemplateLibrary,
}

impl EnvironmentConfigurator {
    /// 自动检测和配置环境
    pub async fn auto_configure(&mut self, task_requirements: &TaskRequirements) -> Result<EnvironmentConfig, ConfigError> {
        println!("🔧 [ENV-CONFIG] 开始自动环境配置");

        // 1. 检测当前系统环境
        let system_detection = self.detect_system_environment().await?;

        // 2. 分析任务依赖
        let dependencies = self.analyze_dependencies(task_requirements).await?;

        // 3. 生成配置计划
        let config_plan = self.generate_config_plan(&system_detection, &dependencies)?;

        // 4. 执行配置
        let execution_result = self.execute_config_plan(&config_plan).await?;

        // 5. 验证配置
        self.verify_configuration(&execution_result).await?;

        Ok(execution_result)
    }

    /// 检测系统环境
    async fn detect_system_environment(&self) -> Result<SystemDetection, ConfigError> {
        Ok(SystemDetection {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            available_memory: self.get_available_memory()?,
            available_cpus: num_cpus::get(),
            gpu_info: self.detect_gpu().await?,
            cuda_available: self.check_cuda().await?,
            rocm_available: self.check_rocm().await?,
            disk_space: self.get_disk_space()?,
        })
    }

    /// 检测GPU
    async fn detect_gpu(&self) -> Result<Vec<GPUInfo>, ConfigError> {
        let mut gpus = Vec::new();

        // 检测CUDA GPU
        if self.check_cuda().await? {
            // 使用nvidia-smi获取GPU信息
            if let Ok(output) = Command::new("nvidia-smi")
                .args(&["--query-gpu=name,memory.total,driver_version", "--format=csv,noheader,nounits"])

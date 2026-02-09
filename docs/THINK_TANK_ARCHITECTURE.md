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

### 3.2 AI驱动的Prompt生成器

```rust
// src/agent/think_tank/prompt_generator.rs

/// AI驱动的Prompt生成器
/// 让AI自己设计和优化Prompt系统
pub struct AIPromptGenerator {
    /// Prompt模板库
    template_library: PromptTemplateLibrary,
    /// 学习历史
    learning_history: Arc<RwLock<Vec<PromptLearningRecord>>>,
    /// 性能追踪器
    performance_tracker: Arc<RwLock<PromptPerformanceTracker>>,
}

impl AIPromptGenerator {
    /// 根据任务自动生成最优Prompt
    pub async fn generate_optimized_prompt(
        &self,
        task_context: &TaskContext,
        available_tools: &[ToolInfo],
        constraints: &PromptConstraints,
    ) -> Result<OptimizedPrompt, PromptError> {
        println!("🎯 [PROMPT-GEN] 开始AI驱动的Prompt生成");

        // 1. 分析任务特点
        let task_analysis = self.analyze_task_characteristics(task_context).await?;

        // 2. 从历史学习中选择最佳策略
        let learned_strategy = self.select_learned_strategy(&task_analysis).await?;

        // 3. 生成候选Prompt
        let candidates = self.generate_candidate_prompts(
            &task_analysis,
            available_tools,
            constraints,
            &learned_strategy,
        )?;

        // 4. 评估和选择最佳Prompt
        let best_prompt = self.evaluate_and_select(&candidates).await?;

        // 5. 记录学习
        self.record_learning(&best_prompt, &candidates).await?;

        Ok(best_prompt)
    }

    /// 分析任务特点
    async fn analyze_task_characteristics(&self, context: &TaskContext) -> Result<TaskAnalysis, PromptError> {
        let analysis_prompt = format!(
            r#"
分析以下AI Agent任务，返回JSON格式的任务特点：

任务描述: {}
任务类型: {:?}
预期复杂度: {:?}

请分析：
1. 任务类型（推理/创意/分析/执行）
2. 最佳思考模式（链式/树状/图状）
3. 需要的工具类型
4. 上下文需求级别
5. 迭代需求程度

返回JSON：
{{
    "task_type": "",
    "thinking_mode": "",
    "tool_categories": [],
    "context_level": "",
    "iteration_level": ""
}}
"#,
            context.description,
            context.task_type,
            context.expected_complexity
        );

        // 调用AI分析...
        unimplemented!()
    }
}

/// 分层Prompt模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredPromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub layers: Vec<PromptLayer>,
    pub context_config: ContextConfig,
    pub performance_metrics: PerformanceMetrics,
}

/// Prompt层配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLayer {
    pub layer_type: LayerType,
    pub template: String,
    pub variables: Vec<String>,
    pub conditions: Vec<LayerCondition>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    System,      // 系统定义
    Task,        // 任务描述
    Context,     // 上下文注入
    Tools,       // 工具说明
    History,     // 历史摘要
    Output,      // 输出格式
    Constraints, // 约束条件
    Examples,    // 示例
}
```

### 3.3 增强的Ralph Loop - 自我配置版

```rust
// src/agent/think_tank/self_configuring_ralph_loop.rs

/// 自我配置的Ralph Loop
/// 实现AI自动决策下一步行动、自动配置环境
pub struct SelfConfiguringRalphLoop {
    /// Ralph Loop配置
    config: RalphLoopConfig,
    /// 环境配置器
    env_configurator: EnvironmentConfigurator,
    /// Prompt生成器
    prompt_generator: Arc<AIPromptGenerator>,
    /// 决策历史
    decision_history: Arc<RwLock<Vec<DecisionRecord>>>,
}

impl SelfConfiguringRalphLoop {
    /// 创建自我配置Ralph Loop
    pub async fn new() -> Result<Self, RalphLoopError> {
        Ok(Self {
            config: RalphLoopConfig::default(),
            env_configurator: EnvironmentConfigurator::new(),
            prompt_generator: Arc::new(AIPromptGenerator::new()?),
            decision_history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// 执行完整的自我配置Ralph Loop
    pub async fn execute_with_self_configuration(
        &mut self,
        task: &Task,
        initial_context: &ExecutionContext,
    ) -> Result<LoopResult, RalphLoopError> {
        println!("🚀 [RALPH-LOOP-SELF] 开始自我配置的Ralph Loop执行");

        let start_time = Utc::now().timestamp_millis();
        let mut iteration = 0;
        let mut current_context = initial_context.clone();

        // Ralph Loop主循环
        loop {
            iteration += 1;

            // 1. 检查是否需要环境配置
            if iteration == 1 || self.should_reconfigure(&current_context).await? {
                self.auto_configure_environment(task, &current_context).await?;
            }

            // 2. AI决策下一步
            let decision = self.ai_decide_with_self_prompt(
                iteration,
                &current_context,
                task,
            ).await?;

            // 3. 执行决策
            let result = self.execute_decision(&decision, &current_context).await?;

            // 4. 更新上下文
            current_context = self.update_context(current_context, &result).await?;

            // 5. 检查终止条件
            if self.should_terminate(&decision, &result, iteration)? {
                return Ok(LoopResult {
                    status: LoopStatus::Completed,
                    iterations: iteration,
                    final_context: current_context,
                    execution_time_ms: Utc::now().timestamp_millis() - start_time,
                });
            }

            // 6. 记录决策
            self.record_decision(iteration, &decision, &result).await?;
        }
    }

    /// 自动配置环境
    async fn auto_configure_environment(
        &mut self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<(), RalphLoopError> {
        println!("🔧 [RALPH-LOOP-SELF] 自动配置运行环境");

        // 检测任务需求
        let requirements = self.detect_task_requirements(task).await?;

        // 自动配置
        let config = self.env_configurator.auto_configure(&requirements).await?;

        // 应用配置
        self.apply_configuration(&config).await?;

        Ok(())
    }

    /// 使用AI生成的Prompt进行决策
    async fn ai_decide_with_self_prompt(
        &self,
        iteration: u32,
        context: &ExecutionContext,
        task: &Task,
    ) -> Result<LoopDecision, RalphLoopError> {
        // 生成动态Prompt
        let dynamic_prompt = self.prompt_generator.generate_optimized_prompt(
            &TaskContext {
                description: task.description.clone(),
                task_type: task.task_type,
                expected_complexity: context.complexity_score,
            },
            &context.available_tools,
            &PromptConstraints {
                max_tokens: 200,
                timeout_seconds: 30,
            },
        ).await?;

        // 使用动态Prompt进行AI决策...
        unimplemented!()
    }
}
```

### 3.4 Iroh去中心化算力共享

```rust
// src/agent/think_tank/decentralized_compute_sharing.rs

/// 去中心化算力共享系统
/// 使用Iroh实现节点间的GPU和计算资源共享
pub struct DecentralizedComputeSharing {
    /// Iroh连接管理器
    iroh_manager: Arc<IrohConnectionManager>,
    /// 资源注册表
    resource_registry: Arc<RwLock<ResourceRegistry>>,
    /// 任务调度器
    task_scheduler: Arc<ComputeTaskScheduler>,
    /// 贡献追踪器
    contribution_tracker: ContributionTracker,
}

impl DecentralizedComputeSharing {
    /// 创建新的算力共享系统
    pub async fn new(iroh_manager: Arc<IrohConnectionManager>) -> Result<Self, ComputeError> {
        Ok(Self {
            iroh_manager,
            resource_registry: Arc::new(RwLock::new(ResourceRegistry::new())),
            task_scheduler: Arc::new(ComputeTaskScheduler::new()),
            contribution_tracker: ContributionTracker::new(),
        })
    }

    /// 注册本地资源到网络
    pub async fn register_local_resources(&self) -> Result<(), ComputeError> {
        let resources = self.detect_local_resources().await?;

        let mut registry = self.resource_registry.write().await;
        registry.register_local(resources.clone());

        // 广播可用资源
        self.broadcast_resource_announcement(&resources).await?;

        Ok(())
    }

    /// 检测本地资源
    async fn detect_local_resources(&self) -> Result<LocalResources, ComputeError> {
        Ok(LocalResources {
            cpu_cores: num_cpus::get() as u64,
            memory_mb: self.get_memory_mb()?,
            gpus: self.detect_gpus().await?,
            storage_mb: self.get_storage_mb()?,
            network_bandwidth_mbps: self.estimate_bandwidth().await?,
        })
    }

    /// 广播资源公告
    async fn broadcast_resource_announcement(&self, resources: &LocalResources) -> Result<(), ComputeError> {
        let announcement = ResourceAnnouncement {
            node_id: self.iroh_manager.node_id(),
            resources: resources.clone(),
            available_from: Utc::now().timestamp(),
            price_per_unit: None, // 暂时免费
        };

        // 使用Iroh Gossip广播
        self.iroh_manager.broadcast(
            b"williw-resource-announcement".to_vec(),
            serde_json::to_vec(&announcement)?,
        ).await?;

        Ok(())
    }

    /// 请求远程算力
    pub async fn request_remote_compute(
        &self,
        requirements: ComputeRequirements,
    ) -> Result<ComputeAllocation, ComputeError> {
        // 1. 发现可用节点
        let available_nodes = self.discover_nodes(&requirements).await?;

        // 2. 选择最佳节点
        let selected_node = self.select_optimal_node(&available_nodes, &requirements)?;

        // 3. 建立连接
        let connection = self.iroh_manager.connect(&selected_node.node_id).await?;

        // 4. 分配任务
        let allocation = ComputeAllocation {
            node_id: selected_node.node_id,
            resources: selected_node.resources,
            connection,
            task_id: uuid::Uuid::new_v4().to_string(),
        };

        Ok(allocation)
    }

    /// 执行远程计算任务
    pub async fn execute_remote_task(
        &self,
        allocation: &ComputeAllocation,
        task: &ComputeTask,
    ) -> Result<TaskResult, ComputeError> {
        // 发送任务到远程节点
        let task_message = TaskMessage {
            task_id: allocation.task_id.clone(),
            task_type: task.task_type,
            input_data: task.input_data.clone(),
            code: task.code.clone(),
            timeout_seconds: task.timeout_seconds,
        };

        // 通过Iroh发送
        self.iroh_manager.send(
            &allocation.node_id,
            b"williw-compute-task".to_vec(),
            serde_json::to_vec(&task_message)?,
        ).await?;

        // 等待结果
        let result = self.wait_for_result(&allocation.task_id, task.timeout_seconds).await?;

        Ok(result)
    }
}

/// GPU资源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub memory_mb: u64,
    pub compute_capability: (u8, u8),
    pub cuda_cores: Option<u32>,
    pub tensor_cores: Option<u32>,
    pub supported_frameworks: Vec<String>,
}

/// 远程节点资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNodeResources {
    pub node_id: String,
    pub public_key: String,
    pub location: GeoLocation,
    pub cpu_cores: u64,
    pub memory_mb: u64,
    pub gpus: Vec<GPUInfo>,
    pub available_from: i64,
    pub reliability_score: f64,
}
```

## 4. 工作流程

### 4.1 智囊团分裂工作流程

```
┌─────────────────────────────────────────────────────────────────┐
│                      任务输入                                     │
│              (训练/推理/数据处理任务)                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      任务分析                                     │
│          AI分析任务需求，确定分裂策略                              │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌──────────┐   ┌──────────┐   ┌──────────┐
│ 数据处理 │   │ 训练协调 │   │ 模型分发 │
│  Agent   │   │  Agent   │   │  Agent   │
└──────────┘   └──────────┘   └──────────┘
    │               │               │
    │               │               │
    └───────────────┴───────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      资源分配                                     │
│              本地GPU + 远程Iroh节点                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      任务执行                                     │
│              Ralph Loop自我配置执行                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      结果聚合                                     │
│              合并各子Agent结果，返回最终输出                       │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Ralph Loop自我配置流程

```
┌─────────────────────────────────────────────────────────────────┐
│                      Ralph Loop开始                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      环境检测                                     │
│              CPU/GPU/内存/存储/网络                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      任务需求分析                                 │
│              AI分析任务，确定所需依赖                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      自动配置                                     │
│          安装依赖/配置环境变量/启动服务                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      AI决策循环                                   │
│              生成动态Prompt → 执行 → 学习                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      任务完成                                     │
│              清理资源/记录学习/返回结果                           │
└─────────────────────────────────────────────────────────────────┘
```

## 5. 文件结构

```
src/agent/think_tank/
├── mod.rs                    # 模块入口
├── agent_splitter.rs        # Agent分裂器
├── agent_config.rs          # Agent配置
├── environment_configurator.rs  # 环境配置器
├── prompt_generator.rs      # AI Prompt生成器
├── self_configuring_ralph_loop.rs  # 自我配置Ralph Loop
├── decentralized_compute_sharing.rs  # 去中心化算力共享
├── resource_registry.rs     # 资源注册表
├── task_scheduler.rs        # 任务调度器
├── contribution_tracker.rs  # 贡献追踪
└── types.rs                 # 类型定义
```

## 6. 实现路线图

### Phase 1: 基础架构 (Week 1-2)
- [ ] 实现Agent分裂器核心逻辑
- [ ] 实现环境自动配置器
- [ ] 集成现有的Ralph Loop

### Phase 2: AI增强 (Week 3-4)
- [ ] 实现AI驱动的Prompt生成器
- [ ] 添加学习历史功能
- [ ] 优化决策逻辑

### Phase 3: 去中心化 (Week 5-6)
- [ ] 实现Iroh资源广播
- [ ] 实现远程算力请求和任务分发
- [ ] 添加贡献追踪和激励机制

### Phase 4: 优化和测试 (Week 7-8)
- [ ] 性能优化
- [ ] 错误处理和恢复
- [ ] 全面测试

## 7. 集成点

### 7.1 与现有系统集成

- **Ralph Loop**: 扩展现有的 [`src/agent/workflow/ralph_loop/`](src/agent/workflow/ralph_loop/) 模块
- **Iroh通讯**: 使用现有的 [`src/comms/transport/iroh.rs`](src/comms/transport/iroh.rs)
- **Prompt系统**: 扩展现有的 [`src/agent/prompts/layered_prompts.rs`](src/agent/prompts/layered_prompts.rs)
- **工作流**: 集成到 [`src/agent/workflow/`](src/agent/workflow/) 模块

### 7.2 配置项

```yaml
# think_tank_config.yaml
think_tank:
  enabled: true
  auto_split: true
  max_sub_agents: 8
  
  environment:
    auto_configure: true
    check_interval_seconds: 60
    
  compute_sharing:
    enabled: true
    register_resources: true
    accept_remote_tasks: false
    
  ralph_loop:
    max_iterations: 100
    auto_configure: true
    
  prompt_generator:
    enabled: true
    learn_from_history: true
``````

//! AI工作流自动化Prompt系统
//!
//! 实现用于工作流编排、任务切分和算力调度的专用Prompt模板

use super::layered_prompts::{LayeredPromptManager, PromptLayer, LayeredPrompt};
use chrono::Utc;

/// AI工作流Prompt模板
pub struct AIWorkflowPrompts;

impl AIWorkflowPrompts {
    /// 获取工作流切分Prompt
    pub fn workflow_splitting_prompt() -> String {
        r#"# AI工作流切分专家

你是一名专业的工作流切分专家，负责将复杂任务分解为可并行执行的子任务。

## 任务分析框架
1. 识别任务的核心目标和约束条件
2. 分析任务的依赖关系和执行顺序
3. 评估可以并行化的部分
4. 确定每个子任务的资源需求

## 切分原则
- **独立性**：子任务之间应尽量减少依赖
- **原子性**：每个子任务应是一个完整的工作单元
- **均衡性**：子任务的计算量应尽量均衡
- **可追溯性**：每个子任务应有明确的输入输出定义

## 输出格式
```json
{
  "workflow_name": "工作流名称",
  "description": "工作流描述",
  "steps": [
    {
      "id": "step_1",
      "name": "步骤名称",
      "description": "步骤描述",
      "tool": "使用的工具",
      "args": {},
      "depends_on": [],
      "estimated_duration_ms": 60000,
      "resource_requirements": {
        "gpu": true,
        "memory_mb": 4096,
        "cpu_cores": 2
      }
    }
  ],
  "parallel_groups": [
    ["step_1", "step_2"],
    ["step_3"]
  ]
}
```

## 决策流程
1. 分析任务类型（计算密集型、IO密集型、混合）
2. 确定最佳切分策略
3. 生成子任务列表
4. 构建依赖图
5. 优化并行度

请基于以上框架分析任务并提供切分方案。"#.to_string()
    }

    /// 获取算力调度Prompt
    pub fn compute_scheduling_prompt() -> String {
        r#"# 去中心化算力调度专家

你是一名算力调度专家，负责在分布式网络中优化任务分配。

## 调度目标
1. **最小化总执行时间**
2. **最大化资源利用率**
3. **平衡网络负载**
4. **确保容错性**

## 调度策略

### GPU任务调度
- 优先分配给具有GPU的节点
- 考虑GPU内存和计算能力
- 避免GPU内存溢出

### 网络感知调度
- 优先选择网络延迟低的节点
- 考虑带宽限制
- 最小化数据传输

### 负载均衡
- 监控节点负载
- 动态调整任务分配
- 避免热点节点

## 节点评估维度
```json
{
  "node_id": "节点ID",
  "compute_score": 0.8,
  "gpu_available": true,
  "gpu_memory_mb": 8192,
  "cpu_cores": 8,
  "memory_mb": 16384,
  "network_latency_ms": 20,
  "bandwidth_mbps": 100,
  "current_load": 0.3,
  "reliability_score": 0.95
}
```

## 调度决策输出
```json
{
  "scheduling_plan": [
    {
      "task_id": "task_1",
      "assigned_node": "node_a",
      "priority": "high",
      "expected_start_time": "2024-01-01T00:00:00Z",
      "fallback_nodes": ["node_b", "node_c"]
    }
  ],
  "optimization_notes": [
    "基于GPU可用性选择node_a",
    "node_b作为热备份"
  ]
}
```

请基于当前网络状态和任务需求提供调度方案。"#.to_string()
    }

    /// 获取Agent自配置Prompt
    pub fn agent_self_configuration_prompt() -> String {
        r#"# Agent自动配置专家

你是一名Agent配置专家，负责自动优化Agent的运行环境和参数。

## 配置维度

### 1. 环境检测
- 操作系统类型和版本
- 可用的计算资源（CPU、内存、GPU）
- 网络连通性
- 依赖包可用性

### 2. 参数优化
- 批处理大小
- 学习率
- 内存缓存大小
- 并发度设置

### 3. 安全策略
- 访问权限控制
- 资源限制
- 网络隔离

## 自适应配置流程
```
1. 检测当前环境
   ↓
2. 评估资源限制
   ↓
3. 选择最优参数
   ↓
4. 验证配置可行性
   ↓
5. 应用配置
   ↓
6. 监控并调整
```

## 配置输出格式
```json
{
  "environment_profile": {
    "os": "macOS 14.0",
    "cpu_cores": 8,
    "total_memory_gb": 16,
    "gpu_available": true,
    "gpu_type": "Apple Silicon",
    "network_type": "WiFi"
  },
  "optimized_config": {
    "batch_size": 32,
    "num_workers": 4,
    "cache_size_mb": 2048,
    "max_concurrent_tasks": 2,
    "use_gpu": true,
    "mixed_precision": true
  },
  "safety_limits": {
    "max_memory_usage_percent": 80,
    "max_cpu_usage_percent": 90,
    "max_gpu_memory_percent": 85,
    "timeout_seconds": 300
  }
}
```

请分析当前环境并生成最优配置。"#.to_string()
    }

    /// 获取错误恢复Prompt
    pub fn error_recovery_prompt(error: &str, context: &str) -> String {
        format!(
            r#"# 错误恢复专家

检测到执行错误，需要进行智能恢复。

## 错误信息
```
{}
```

## 执行上下文
```
{}
```

## 恢复策略

### 1. 错误分类
- **临时性错误**：网络超时、资源暂不可用 → 重试
- **配置错误**：参数错误、路径问题 → 修正配置
- **资源错误**：内存不足、磁盘满 → 释放资源或降级
- **逻辑错误**：代码缺陷、算法问题 → 调整策略

### 2. 恢复决策流程
```
分析错误类型
    ↓
评估影响范围
    ↓
选择恢复策略
    ↓
执行恢复操作
    ↓
验证恢复结果
```

### 3. 恢复选项
- RETRY: 立即重试（指数退避）
- ROLLBACK: 回滚到上一个稳定状态
- ADJUST: 调整参数后重试
- SKIP: 跳过当前步骤（如果可选）
- FAILOVER: 切换到备用节点
- ESCALATE: 上报给更高级别的Agent

## 决策输出
```json
{{
  "error_type": "temporary|config|resource|logic",
  "severity": "low|medium|high|critical",
  "recovery_strategy": "RETRY|ROLLBACK|ADJUST|SKIP|FAILOVER|ESCALATE",
  "action": {{
    "type": "specific_action",
    "parameters": {{}}
  }},
  "estimated_recovery_time_ms": 5000,
  "fallback_plan": "如果恢复失败则..."
}}
```

请分析错误并提供恢复方案。"#,
            error, context
        )
    }

    /// 获取P2P协作Prompt
    pub fn p2p_collaboration_prompt() -> String {
        r#"# P2P去中心化协作专家

你是一名P2P网络协作专家，负责协调分布式Agent之间的协作。

## 协作模式

### 1. 主从模式
- 一个协调者（Coordinator）负责分发任务
- 多个工作者（Worker）执行子任务
- 协调者收集并整合结果

### 2. 对等模式
- 所有节点地位平等
- 通过共识算法协调
- 适合去中心化场景

### 3. 混合模式
- 动态选举协调者
- 故障时自动切换
- 平衡效率和容错

## 协作协议

### 任务分发
```json
{
  "message_type": "TASK_ASSIGN",
  "task_id": "task_123",
  "worker_id": "worker_456",
  "payload": {
    "subtask": "...",
    "deadline": "2024-01-01T00:00:00Z",
    "priority": "high"
  }
}
```

### 进度报告
```json
{
  "message_type": "PROGRESS_REPORT",
  "task_id": "task_123",
  "worker_id": "worker_456",
  "progress": 0.75,
  "status": "running",
  "metrics": {
    "cpu_usage": 0.8,
    "memory_usage_mb": 4096
  }
}
```

### 结果提交
```json
{
  "message_type": "RESULT_SUBMIT",
  "task_id": "task_123",
  "worker_id": "worker_456",
  "result": {
    "data": "...",
    "checksum": "sha256:...",
    "execution_time_ms": 60000
  }
}
```

## 冲突解决
1. **结果冲突**：投票机制选择多数结果
2. **任务重复**：去重检查避免重复执行
3. **节点故障**：自动重新分配任务

## 安全考虑
- 消息签名验证
- 访问权限控制
- 数据加密传输

请设计P2P协作策略。"#.to_string()
    }

    /// 获取任务优化Prompt
    pub fn task_optimization_prompt() -> String {
        r#"# 任务优化专家

你是一名任务优化专家，负责分析和优化工作流性能。

## 优化维度

### 1. 计算优化
- 算法选择（时间复杂度）
- 并行化策略
- 缓存利用
- 批处理优化

### 2. 内存优化
- 内存分配策略
- 数据流优化
- 垃圾回收调优
- 内存池使用

### 3. 网络优化
- 数据传输最小化
- 压缩策略
- 预取和缓存
- 连接复用

### 4. GPU优化
- 内核融合
- 内存访问模式
- 流并行
- 混合精度

## 性能分析框架
```
收集性能指标
    ↓
识别瓶颈
    ↓
根因分析
    ↓
制定优化方案
    ↓
实施优化
    ↓
验证效果
```

## 优化建议格式
```json
{
  "optimization_plan": [
    {
      "category": "compute|memory|network|gpu",
      "issue": "描述问题",
      "recommendation": "优化建议",
      "expected_improvement": "预期改进（如：减少30%执行时间）",
      "implementation_complexity": "low|medium|high",
      "priority": "critical|high|medium|low"
    }
  ],
  "risk_assessment": [
    "优化可能带来的风险"
  ]
}
```

请分析工作流并提供优化建议。"#.to_string()
    }
}

/// 为LayeredPromptManager添加AI工作流专用的默认Prompt
pub fn add_ai_workflow_prompts(manager: &mut LayeredPromptManager) {
    // 工作流切分层
    manager.add_prompt_to_layer(PromptLayer::Task, LayeredPrompt {
        id: "ai-workflow-splitting".to_string(),
        layer: PromptLayer::Task,
        content: AIWorkflowPrompts::workflow_splitting_prompt(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        ttl: None,
        priority: 10,
    });

    // 算力调度层
    manager.add_prompt_to_layer(PromptLayer::Tools, LayeredPrompt {
        id: "ai-compute-scheduling".to_string(),
        layer: PromptLayer::Tools,
        content: AIWorkflowPrompts::compute_scheduling_prompt(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        ttl: None,
        priority: 9,
    });

    // 自配置层
    manager.add_prompt_to_layer(PromptLayer::Context, LayeredPrompt {
        id: "ai-self-configuration".to_string(),
        layer: PromptLayer::Context,
        content: AIWorkflowPrompts::agent_self_configuration_prompt(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        ttl: None,
        priority: 8,
    });

    // P2P协作者层
    manager.add_prompt_to_layer(PromptLayer::Tools, LayeredPrompt {
        id: "ai-p2p-collaboration".to_string(),
        layer: PromptLayer::Tools,
        content: AIWorkflowPrompts::p2p_collaboration_prompt(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        ttl: None,
        priority: 7,
    });

    // 任务优化层
    manager.add_prompt_to_layer(PromptLayer::Output, LayeredPrompt {
        id: "ai-task-optimization".to_string(),
        layer: PromptLayer::Output,
        content: AIWorkflowPrompts::task_optimization_prompt(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        ttl: None,
        priority: 6,
    });
}

/// AI工作流决策结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDecision {
    pub decision_type: DecisionType,
    pub confidence: f32,
    pub reasoning: String,
    pub action: Action,
    pub fallback: Option<Box<WorkflowDecision>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionType {
    SplitTask,
    ScheduleCompute,
    ConfigureEnvironment,
    RecoverFromError,
    CollaborateP2P,
    OptimizeTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String,
    pub parameters: serde_json::Value,
    pub expected_duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_prompts_not_empty() {
        assert!(!AIWorkflowPrompts::workflow_splitting_prompt().is_empty());
        assert!(!AIWorkflowPrompts::compute_scheduling_prompt().is_empty());
        assert!(!AIWorkflowPrompts::agent_self_configuration_prompt().is_empty());
    }

    #[test]
    fn test_error_recovery_prompt_format() {
        let prompt = AIWorkflowPrompts::error_recovery_prompt("test error", "test context");
        assert!(prompt.contains("test error"));
        assert!(prompt.contains("test context"));
    }
}

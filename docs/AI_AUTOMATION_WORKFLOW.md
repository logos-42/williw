# AI自动化工作流系统

## 概述

AI自动化工作流系统是一个完整的、智能的Agent切分和执行框架，集成了以下核心功能：

- **Ralph Loop**: 自循环工作流执行引擎
- **自动环境配置**: AI驱动的环境检测和配置
- **分层Prompt系统**: 结构化、可扩展的Prompt管理
- **去中心化算力共享**: 基于Iroh的P2P计算网络
- **GPU节点管理**: 智能GPU资源调度和管理

## 架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                      AI自动化工作流系统                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Ralph Loop    │  │  分层Prompt系统  │  │  AI决策引擎     │  │
│  │  ┌───────────┐  │  │  ┌───────────┐  │  │  ┌───────────┐  │  │
│  │  │ 环境配置   │  │  │  │ System层  │  │  │  │ 任务切分   │  │  │
│  │  │ 任务执行   │  │  │  │ Task层    │  │  │  │ 错误恢复   │  │  │
│  │  │ 结果聚合   │  │  │  │ Context层 │  │  │  │ 资源调度   │  │  │
│  │  └───────────┘  │  │  │ Tools层   │  │  │  └───────────┘  │  │
│  └─────────────────┘  │  └───────────┘  │  └─────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              去中心化算力共享网络 (DCN)                    │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │ GPU管理器    │  │ 任务调度器   │  │ 结果聚合器       │   │  │
│  │  │ ·节点发现    │  │ ·负载均衡    │  │ ·多节点结果合并  │   │  │
│  │  │ ·资源监控    │  │ ·任务分发    │  │ ·一致性检查     │   │  │
│  │  │ ·心跳检测    │  │ ·故障转移    │  │ ·质量评估       │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                        Iroh P2P网络层                           │
└─────────────────────────────────────────────────────────────────┘
```

## 核心模块

### 1. Ralph Loop (自动环境配置)

**文件**: `src/agent/workflow/ralph_loop/auto_environment.rs`

功能:
- 自动检测系统资源 (CPU、内存、GPU)
- 检测Python环境和依赖包
- 配置Iroh P2P网络连接
- AI决策是否自动安装缺失依赖
- 发现网络中的对等节点

使用方法:
```rust
let executor = AsyncWorkflowExecutor::new()?;
let env_config = executor.auto_configure_environment(
    "execution_id",
    "api_key"
).await?;
```

### 2. 分层Prompt系统

**文件**: `src/agent/prompts/ai_workflow_prompts.rs`

功能:
- **工作流切分**: 智能任务分解和并行化
- **算力调度**: 基于网络状态的资源分配
- **Agent自配置**: 自动优化运行参数
- **错误恢复**: 智能错误分类和恢复策略
- **P2P协作**: 去中心化节点协调
- **任务优化**: 性能分析和优化建议

### 3. GPU管理器

**文件**: `src/agent/compute/gpu_manager.rs`

功能:
- GPU节点注册和发现
- 设备能力检测 (CUDA、Metal、Vulkan)
- 任务队列管理
- 智能任务调度
- 节点心跳监控
- 性能指标收集

关键结构:
```rust
pub struct GpuNode {
    pub node_id: String,
    pub device_info: DeviceCapabilities,
    pub gpu_info: Vec<GpuDevice>,
    pub status: NodeStatus,
    pub performance_metrics: PerformanceMetrics,
}
```

### 4. 去中心化计算网络 (DCN)

**文件**: `src/agent/compute/decentralized_compute.rs`

功能:
- P2P节点发现和加入
- 任务切分和分发
- 结果聚合和验证
- 激励机制和声誉系统
- 容错和故障转移

消息类型:
```rust
pub enum NetworkMessage {
    TaskRequest(ComputeTask),
    TaskResponse(TaskResult),
    Heartbeat { node_id, timestamp, load },
    NodeJoin { node_id, capabilities },
    NodeLeave { node_id },
    StatusUpdate { node_id, status },
}
```

## 使用示例

### 完整演示

```rust
use williw::agent::workflow::AIAutomationDemo;

#[tokio::main]
async fn main() -> Result<(), String> {
    // 运行完整演示
    AIAutomationDemo::run_full_demo().await?;
    Ok(())
}
```

### 简化演示

```rust
// 仅运行环境自动配置
AIAutomationDemo::run_simple_demo().await?;
```

### 手动配置工作流

```rust
use williw::agent::workflow::*;
use williw::agent::compute::*;
use williw::agent::prompts::*;

#[tokio::main]
async fn main() -> Result<(), String> {
    // 1. 创建执行器
    let executor = AsyncWorkflowExecutor::new()?;
    
    // 2. 自动配置环境
    let env_config = executor.auto_configure_environment(
        "exec_001",
        "your_api_key"
    ).await?;
    
    // 3. 初始化计算网络
    let node_id = env_config.node_id.unwrap();
    let compute_manager = ComputeResourceManager::new(node_id).await?;
    compute_manager.initialize().await?;
    
    // 4. 创建并提交任务
    let task = ComputeTask {
        task_id: "task_001".to_string(),
        task_type: ComputeTaskType::ModelTraining,
        // ... 其他配置
    };
    
    if let Some(network) = compute_manager.get_network().await {
        let task_id = network.submit_compute_task(task).await?;
        println!("任务已提交: {}", task_id);
    }
    
    Ok(())
}
```

## 配置选项

### 环境变量

```bash
# 种子节点配置
export WILLIW_SEED_NODES="node1.z32,node2.z32,node3.z32"
export WILLIW_GPU_SEED_NODES="gpu_node1.z32,gpu_node2.z32"
export WILLIW_COMPUTE_SEEDS="compute1.z32,compute2.z32"

# API密钥
export ANTHROPIC_API_KEY="your_api_key"
```

### Ralph Loop配置

```rust
let ralph_config = RalphLoopConfig {
    enabled: true,
    max_iterations: 50,
    iteration_delay_ms: 500,
    completion_checker: Some("auto".to_string()),
    max_total_time_ms: Some(1800000), // 30分钟
    iteration_timeout_ms: 120000, // 2分钟
    max_cost: Some(10.0),
    enable_history: true,
    smart_retry: SmartRetryStrategy {
        enabled: true,
        adaptive_retry: true,
        max_consecutive_failures: 5,
        learning_period: 3,
    },
};
```

## 任务类型支持

- **ModelTraining**: 模型训练任务
- **ModelInference**: 模型推理任务
- **DataProcessing**: 数据处理任务
- **FederatedLearning**: 联邦学习任务
- **DistributedTraining**: 分布式训练任务
- **Custom(String)**: 自定义任务类型

## 调度策略

### 负载均衡权重
- `load_balance_weight`: 0.3 - 负载均衡
- `locality_weight`: 0.25 - 数据本地性
- `cost_weight`: 0.2 - 成本优化
- `reliability_weight`: 0.25 - 可靠性

### 调度算法
1. **最佳节点选择**: 基于综合评分选择最优节点
2. **任务优先级**: Critical > High > Normal > Low
3. **动态调整**: 根据网络状态实时调整调度策略
4. **故障转移**: 自动切换到备用节点

## 安全特性

- **消息签名**: 所有P2P消息都经过签名验证
- **访问控制**: 基于声誉的权限管理
- **数据加密**: 传输层加密保护
- **沙箱执行**: 任务在隔离环境中运行

## 性能优化

- **批处理**: 支持任务批量处理提高效率
- **缓存**: 智能缓存常用数据和模型
- **预取**: 预测性数据预取减少等待时间
- **流控制**: 自适应流量控制避免网络拥塞

## 监控和调试

### 日志级别
```rust
// 设置日志级别
export RUST_LOG=williw=debug,agent=trace
```

### 指标收集
- 任务执行时间
- GPU利用率
- 网络延迟
- 节点健康状态
- 任务成功率

## 故障排除

### 常见问题

1. **环境配置失败**
   - 检查系统权限
   - 验证Python安装
   - 确认网络连接

2. **节点发现失败**
   - 检查种子节点配置
   - 验证防火墙设置
   - 确认Iroh网络可用

3. **任务调度失败**
   - 检查节点资源是否充足
   - 验证任务资源需求
   - 查看调度日志

### 调试命令

```bash
# 检查环境配置
cargo run --bin williw-bin -- check-env

# 查看节点状态
cargo run --bin williw-bin -- node-status

# 测试网络连接
cargo run --bin williw-bin -- network-test
```

## 贡献指南

1. Fork 项目
2. 创建功能分支
3. 提交变更
4. 确保测试通过
5. 提交Pull Request

## 许可证

MIT License - 详见 LICENSE 文件

//! Task 执行器
//!
//! 支持顺序、并行、Swarm 三种执行模式

use super::manifest::{
    TaskExecutionMode, TaskManifest, TaskResult, TaskStatus, SubTaskResult, SwarmConfig, SwarmStrategy,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde_json::Value;

/// Task 执行器
pub struct TaskExecutor {
    /// 运行中的任务
    running_tasks: RwLock<HashMap<String, Arc<RwLock<TaskResult>>>>,
    /// 任务清单缓存
    manifests: RwLock<HashMap<String, TaskManifest>>,
}

/// Task 执行上下文
pub struct ExecutionContext {
    /// 任务ID
    pub task_id: String,
    /// 输入参数
    pub input: Value,
    /// Agent 池
    pub agent_pool: Option<AgentPool>,
    /// 回调函数
    pub on_progress: Option<Box<dyn Fn(ProgressEvent) + Send + Sync>>,
}

/// Agent 池
pub struct AgentPool {
    /// 可用 Agent 数量
    pub size: usize,
    /// Agent 列表
    pub agents: Vec<AgentInfo>,
}

/// Agent 信息
pub struct AgentInfo {
    /// Agent ID
    pub id: String,
    /// Agent 名称
    pub name: String,
    /// 状态
    pub status: AgentStatus,
    /// 当前任务
    pub current_task: Option<String>,
}

/// Agent 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// 空闲
    Idle,
    /// 忙碌
    Busy,
    /// 离线
    Offline,
    /// 错误
    Error,
}

/// 进度事件
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    /// 任务ID
    pub task_id: String,
    /// 步骤ID
    pub step_id: Option<String>,
    /// Agent ID
    pub agent_id: Option<String>,
    /// 事件类型
    pub event_type: ProgressEventType,
    /// 消息
    pub message: String,
    /// 进度 (0-100)
    pub progress: u8,
}

/// 进度事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEventType {
    /// 开始
    Started,
    /// 步骤开始
    StepStarted,
    /// 步骤完成
    StepCompleted,
    /// 步骤失败
    StepFailed,
    /// 完成
    Completed,
    /// 失败
    Failed,
    /// 取消
    Cancelled,
}

impl TaskExecutor {
    /// 创建新的执行器
    pub fn new() -> Self {
        Self {
            running_tasks: RwLock::new(HashMap::new()),
            manifests: RwLock::new(HashMap::new()),
        }
    }

    /// 注册任务清单
    pub async fn register(&self, manifest: TaskManifest) {
        let mut manifests = self.manifests.write().await;
        manifests.insert(manifest.id.clone(), manifest);
    }

    /// 获取任务清单
    pub async fn get_manifest(&self, task_id: &str) -> Option<TaskManifest> {
        let manifests = self.manifests.read().await;
        manifests.get(task_id).cloned()
    }

    /// 列出所有任务
    pub async fn list_tasks(&self) -> Vec<TaskManifest> {
        let manifests = self.manifests.read().await;
        manifests.values().cloned().collect()
    }

    /// 执行任务
    pub async fn execute(&self, task_id: &str, input: Value) -> TaskResult {
        let manifest = match self.get_manifest(task_id).await {
            Some(m) => m,
            None => {
                return TaskResult {
                    task_id: task_id.to_string(),
                    status: TaskStatus::Failed,
                    started_at: chrono::Utc::now().timestamp(),
                    finished_at: Some(chrono::Utc::now().timestamp()),
                    output: None,
                    error: Some(format!("Task {} not found", task_id)),
                    subtask_results: vec![],
                };
            }
        };

        let started_at = chrono::Utc::now().timestamp();
        
        let result = match manifest.execution_mode {
            TaskExecutionMode::Sequential => {
                self.execute_sequential(&manifest, input).await
            }
            TaskExecutionMode::Parallel => {
                self.execute_parallel(&manifest, input).await
            }
            TaskExecutionMode::Swarm => {
                self.execute_swarm(&manifest, input).await
            }
        };

        TaskResult {
            task_id: task_id.to_string(),
            status: result.0,
            started_at,
            finished_at: Some(chrono::Utc::now().timestamp()),
            output: result.1,
            error: result.2,
            subtask_results: result.3,
        }
    }

    /// 顺序执行
    async fn execute_sequential(&self, manifest: &TaskManifest, input: Value) -> (TaskStatus, Option<Value>, Option<String>, Vec<SubTaskResult>) {
        let mut current_input = input;
        let mut subtask_results = vec![];
        
        for step in &manifest.steps {
            // 检查依赖
            let deps_satisfied = step.depends_on.iter().all(|dep| {
                subtask_results.iter().any(|r: &SubTaskResult| r.subtask_id == *dep && r.status == TaskStatus::Success)
            });
            
            if !deps_satisfied {
                return (
                    TaskStatus::Failed,
                    None,
                    Some(format!("Dependencies not satisfied for step {}", step.id)),
                    subtask_results,
                );
            }

            // 执行步骤
            let result = Self::execute_step_static(step, current_input.clone()).await;
            
            subtask_results.push(result.clone());
            
            match result.status {
                TaskStatus::Success => {
                    current_input = result.output.unwrap_or(current_input);
                }
                _ => {
                    return (
                        TaskStatus::Failed,
                        None,
                        result.error,
                        subtask_results,
                    );
                }
            }
        }

        (TaskStatus::Success, Some(current_input), None, subtask_results)
    }

    /// 并行执行
    async fn execute_parallel(&self, manifest: &TaskManifest, input: Value) -> (TaskStatus, Option<Value>, Option<String>, Vec<SubTaskResult>) {
        // 找出所有可并行的步骤
        let parallel_steps: Vec<_> = manifest.steps.iter()
            .filter(|s| s.parallelizable)
            .collect();

        if parallel_steps.is_empty() {
            return self.execute_sequential(manifest, input).await;
        }

        // 并行执行
        let mut handles = vec![];
        for step in parallel_steps {
            let step_clone = step.clone();
            let input_clone = input.clone();
            let handle = tokio::spawn(async move {
                Self::execute_step_static(&step_clone, input_clone).await
            });
            handles.push((step.id.clone(), handle));
        }

        let mut subtask_results = vec![];
        let mut all_success = true;

        for (step_id, handle) in handles {
            match handle.await {
                Ok(result) => {
                    if result.status != TaskStatus::Success {
                        all_success = false;
                    }
                    subtask_results.push(result);
                }
                Err(e) => {
                    subtask_results.push(SubTaskResult {
                        subtask_id: step_id,
                        agent_id: None,
                        status: TaskStatus::Failed,
                        output: None,
                        error: Some(format!("Task join error: {}", e)),
                    });
                    all_success = false;
                }
            }
        }

        if all_success {
            (TaskStatus::Success, Some(input), None, subtask_results)
        } else {
            (TaskStatus::Failed, None, Some("Some parallel tasks failed".to_string()), subtask_results)
        }
    }

    /// Swarm 执行
    async fn execute_swarm(&self, manifest: &TaskManifest, input: Value) -> (TaskStatus, Option<Value>, Option<String>, Vec<SubTaskResult>) {
        let swarm_config = match &manifest.swarm_config {
            Some(c) => c,
            None => {
                return (
                    TaskStatus::Failed,
                    None,
                    Some("Swarm config not found".to_string()),
                    vec![],
                );
            }
        };

        // 创建 Agent 池
        let agent_pool = AgentPool {
            size: swarm_config.agent_count,
            agents: (0..swarm_config.agent_count).map(|i| AgentInfo {
                id: format!("agent_{}", i),
                name: format!("Worker Agent {}", i),
                status: AgentStatus::Idle,
                current_task: None,
            }).collect(),
        };

        // 根据策略执行
        match swarm_config.strategy {
            SwarmStrategy::Broadcast => {
                self.execute_swarm_broadcast(&agent_pool, input).await
            }
            SwarmStrategy::Shard => {
                self.execute_swarm_shard(&agent_pool, input).await
            }
            SwarmStrategy::Vote => {
                self.execute_swarm_vote(&agent_pool, input).await
            }
            SwarmStrategy::Hierarchical => {
                self.execute_swarm_hierarchical(&agent_pool, input).await
            }
        }
    }

    /// Broadcast 策略：所有 Agent 收到相同任务
    async fn execute_swarm_broadcast(&self, pool: &AgentPool, input: Value) -> (TaskStatus, Option<Value>, Option<String>, Vec<SubTaskResult>) {
        let mut subtask_results = vec![];
        
        // 所有 Agent 并行执行相同任务
        let mut handles = vec![];
        for agent in &pool.agents {
            let input_clone = input.clone();
            let agent_id = agent.id.clone();
            let handle = tokio::spawn(async move {
                // 模拟 Agent 执行
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                SubTaskResult {
                    subtask_id: format!("{}_task", agent_id),
                    agent_id: Some(agent_id),
                    status: TaskStatus::Success,
                    output: Some(input_clone),
                    error: None,
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            if let Ok(result) = handle.await {
                subtask_results.push(result);
            }
        }

        // 聚合结果
        let output = serde_json::json!({
            "strategy": "broadcast",
            "results": subtask_results.iter().map(|r| r.output.clone()).collect::<Vec<_>>()
        });

        (TaskStatus::Success, Some(output), None, subtask_results)
    }

    /// Shard 策略：任务拆分给不同 Agent
    async fn execute_swarm_shard(&self, pool: &AgentPool, input: Value) -> (TaskStatus, Option<Value>, Option<String>, Vec<SubTaskResult>) {
        // 将输入拆分成多个分片
        let shard_count = pool.agents.len();
        let mut subtask_results = vec![];

        for (i, agent) in pool.agents.iter().enumerate() {
            let shard = serde_json::json!({
                "shard_id": i,
                "total_shards": shard_count,
                "data": input
            });

            subtask_results.push(SubTaskResult {
                subtask_id: format!("{}_shard_{}", agent.id, i),
                agent_id: Some(agent.id.clone()),
                status: TaskStatus::Success,
                output: Some(shard),
                error: None,
            });
        }

        let output = serde_json::json!({
            "strategy": "shard",
            "shard_count": shard_count
        });

        (TaskStatus::Success, Some(output), None, subtask_results)
    }

    /// Vote 策略：Agent 投票决策
    async fn execute_swarm_vote(&self, pool: &AgentPool, input: Value) -> (TaskStatus, Option<Value>, Option<String>, Vec<SubTaskResult>) {
        // 模拟投票过程
        let votes = pool.agents.len();
        
        let output = serde_json::json!({
            "strategy": "vote",
            "votes": votes,
            "result": "majority_agreement"
        });

        (TaskStatus::Success, Some(output), None, vec![])
    }

    /// Hierarchical 策略：Leader + Workers
    async fn execute_swarm_hierarchical(&self, pool: &AgentPool, input: Value) -> (TaskStatus, Option<Value>, Option<String>, Vec<SubTaskResult>) {
        if pool.agents.is_empty() {
            return (TaskStatus::Failed, None, Some("No agents available".to_string()), vec![]);
        }

        let leader = &pool.agents[0];
        let workers = &pool.agents[1..];

        let mut subtask_results = vec![];

        // Leader 分发任务
        subtask_results.push(SubTaskResult {
            subtask_id: format!("{}_distribute", leader.id),
            agent_id: Some(leader.id.clone()),
            status: TaskStatus::Success,
            output: Some(serde_json::json!({"action": "task_distributed", "worker_count": workers.len()})),
            error: None,
        });

        // Workers 执行
        for worker in workers {
            subtask_results.push(SubTaskResult {
                subtask_id: format!("{}_execute", worker.id),
                agent_id: Some(worker.id.clone()),
                status: TaskStatus::Success,
                output: Some(input.clone()),
                error: None,
            });
        }

        // Leader 聚合结果
        let output = serde_json::json!({
            "strategy": "hierarchical",
            "leader": leader.id,
            "workers": workers.iter().map(|w| &w.id).collect::<Vec<_>>(),
            "aggregated": true
        });

        (TaskStatus::Success, Some(output), None, subtask_results)
    }

    /// 执行单个步骤
    async fn execute_step(step: &super::manifest::TaskStep, input: Value) -> (SubTaskResult, Value) {
        // 简化实现：实际应该调用相应的工具或 Skill
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let output = serde_json::json!({
            "step_id": step.id,
            "executed": true,
            "input": input
        });

        (
            SubTaskResult {
                subtask_id: step.id.clone(),
                agent_id: None,
                status: TaskStatus::Success,
                output: Some(output.clone()),
                error: None,
            },
            output,
        )
    }

    async fn execute_step_static(step: &super::manifest::TaskStep, input: Value) -> SubTaskResult {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        SubTaskResult {
            subtask_id: step.id.clone(),
            agent_id: None,
            status: TaskStatus::Success,
            output: Some(input),
            error: None,
        }
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

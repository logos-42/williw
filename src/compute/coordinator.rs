//! 分布式推理协调器
//!
//! 负责协调多节点共同执行模型推理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{timeout, Duration};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::protocol::{InferenceMessage, ShardInfo, ShardStatus, ShardTable, ExecutionMetrics, InferenceRequest, InferenceResult, InferenceConfig, PartialResult, AggregationMethod};
use super::cache::{IntermediateCache, CachedResult};
use crate::compute::ResultAggregator;
use crate::ai_decision::{AIDecisionEngine, ExecutionContext, AcceptanceCriterion};
use crate::comms::transport::iroh::IrohConnectionManager;

/// 推理任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceTaskState {
    /// 任务 ID
    pub task_id: String,
    /// 模型 ID
    pub model_id: String,
    /// 输入数据
    pub input_data: Vec<u8>,
    /// 配置
    pub config: InferenceConfig,
    /// 当前执行的分片索引
    pub current_shard_index: usize,
    /// 分片执行顺序
    pub shard_order: Vec<String>,
    /// 中间结果
    pub intermediate_results: HashMap<String, Vec<u8>>,
    /// 状态
    pub status: InferenceStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 错误信息
    pub error: Option<String>,
    /// 重试次数
    pub retry_count: u32,
}

impl InferenceTaskState {
    /// 创建新的任务状态
    pub fn new(request: InferenceRequest) -> Self {
        Self {
            task_id: request.task_id,
            model_id: request.model_id,
            input_data: request.input_data,
            config: request.config,
            current_shard_index: 0,
            shard_order: Vec::new(),
            intermediate_results: HashMap::new(),
            status: InferenceStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
            retry_count: 0,
        }
    }
    
    /// 计算进度
    pub fn progress(&self) -> f32 {
        if self.shard_order.is_empty() {
            return 0.0;
        }
        self.current_shard_index as f32 / self.shard_order.len() as f32
    }
}

/// 推理状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InferenceStatus {
    /// 等待中
    Pending,
    /// 正在执行
    Running {
        /// 当前执行的节点
        current_node: String,
        /// 当前分片
        current_shard: String,
    },
    /// 等待中间结果
    WaitingForIntermediate {
        /// 来源节点
        from_node: String,
        /// 分片 ID
        shard_id: String,
    },
    /// 已完成
    Completed,
    /// 失败
    Failed(String),
    /// 已取消
    Cancelled,
}

/// 协调器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// 是否启用分布式推理
    pub enabled: bool,
    /// 最大并行任务数
    pub max_parallel_tasks: usize,
    /// 中间结果缓存大小（MB）
    pub cache_size_mb: u64,
    /// 节点超时时间（秒）
    pub node_timeout_secs: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否启用 AI 优化调度
    pub enable_ai_scheduling: bool,
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_parallel_tasks: 4,
            cache_size_mb: 1024,
            node_timeout_secs: 60,
            max_retries: 3,
            enable_ai_scheduling: true,
            heartbeat_interval_secs: 30,
        }
    }
}

/// 分布式推理协调器
pub struct DistributedInferenceCoordinator {
    /// 本地节点 ID
    node_id: String,
    /// 配置
    config: CoordinatorConfig,
    /// 分片表
    shard_table: Arc<RwLock<ShardTable>>,
    /// 任务状态表
    tasks: Arc<RwLock<HashMap<String, InferenceTaskState>>>,
    /// 中间结果缓存
    cache: Arc<IntermediateCache>,
    /// 节点状态
    node_status: Arc<RwLock<HashMap<String, NodeStatus>>>,
    /// 连接管理器
    connection_manager: Arc<Mutex<Option<Arc<IrohConnectionManager>>>>,
    /// AI 决策引擎
    ai_decision: Option<Arc<AIDecisionEngine>>,
    /// 运行状态
    is_running: Arc<RwLock<bool>>,
    /// 任务完成通知
    completion_tx: mpsc::Sender<TaskCompletion>,
    /// 任务完成接收器
    completion_rx: Mutex<mpsc::Receiver<TaskCompletion>>,
    /// 结果聚合器
    aggregator: Arc<RwLock<ResultAggregator>>,
}

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    /// 节点 ID
    pub node_id: String,
    /// 是否在线
    pub online: bool,
    /// 负载 (0.0-1.0)
    pub load: f32,
    /// 可用显存 (MB)
    pub available_memory_mb: u64,
    /// 最后心跳时间
    pub last_heartbeat: DateTime<Utc>,
    /// 分片列表
    pub shards: Vec<String>,
}

/// 任务完成通知
#[derive(Debug, Clone)]
pub struct TaskCompletion {
    pub task_id: String,
    pub result: InferenceResult,
}

impl DistributedInferenceCoordinator {
    /// 创建新的协调器
    pub fn new(node_id: String, config: CoordinatorConfig) -> Self {
        let (completion_tx, completion_rx) = mpsc::channel(100);
        
        Self {
            node_id,
            config,
            shard_table: Arc::new(RwLock::new(ShardTable::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(IntermediateCache::new(1024)),
            node_status: Arc::new(RwLock::new(HashMap::new())),
            connection_manager: Arc::new(Mutex::new(None)),
            ai_decision: None,
            is_running: Arc::new(RwLock::new(false)),
            completion_tx,
            completion_rx: Mutex::new(completion_rx),
            aggregator: Arc::new(RwLock::new(ResultAggregator::new())),
        }
    }
    
    /// 设置 AI 决策引擎
    pub fn with_ai_decision(mut self, ai_decision: Arc<AIDecisionEngine>) -> Self {
        self.ai_decision = Some(ai_decision);
        self
    }
    
    /// 设置连接管理器
    pub async fn set_connection_manager(&self, manager: Arc<IrohConnectionManager>) {
        let mut cm = self.connection_manager.lock().await;
        *cm = Some(manager);
    }
    
    /// 启动协调器
    pub async fn start(&self) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }
        
        log::info!("🚀 [Coordinator] Starting Distributed Inference Coordinator...");
        
        // 设置运行状态
        *self.is_running.write().await = true;
        
        // 启动心跳服务
        self.start_heartbeat_service().await;
        
        // 启动任务调度器
        self.start_task_scheduler().await;
        
        log::info!("✅ [Coordinator] Coordinator started successfully");
        Ok(())
    }
    
    /// 停止协调器
    pub async fn stop(&self) {
        *self.is_running.write().await = false;
        log::info!("🛑 [Coordinator] Coordinator stopped");
    }
    
    /// 注册模型分片
    pub async fn register_model_shards(
        &self,
        model_id: &str,
        shards: Vec<ShardInfo>,
    ) -> Result<(), String> {
        let mut table = self.shard_table.write().await;
        
        // 更新节点状态
        let mut node_status = self.node_status.write().await;
        for shard in &shards {
            let status = node_status.entry(shard.node_id.clone()).or_insert(NodeStatus {
                node_id: shard.node_id.clone(),
                online: true,
                load: 0.0,
                available_memory_mb: 0,
                last_heartbeat: Utc::now(),
                shards: Vec::new(),
            });
            status.shards.push(shard.shard_id.clone());
        }
        
        // 注册分片
        table.register_shards(model_id, shards);
        
        log::info!("📋 [Coordinator] Registered shards for model: {}", model_id);
        Ok(())
    }
    
    /// 提交推理任务
    pub async fn submit_task(&self, request: InferenceRequest) -> Result<String, String> {
        let task_id = request.task_id.clone();
        
        // 创建任务状态
        let mut state = InferenceTaskState::new(request);
        
        // 获取分片执行顺序
        let shard_table = self.shard_table.read().await;
        if let Some(shards) = shard_table.get_shards(&state.model_id) {
            // 按层范围排序
            let mut sorted_shards: Vec<_> = shards.iter().collect();
            sorted_shards.sort_by_key(|s| s.layer_range.0);
            state.shard_order = sorted_shards.iter().map(|s| s.shard_id.clone()).collect();
        } else {
            return Err(format!("Model {} not found in shard table", state.model_id));
        }
        drop(shard_table);
        
        // 添加到任务队列
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), state);
        
        log::info!("📥 [Coordinator] Task {} submitted", task_id);
        Ok(task_id)
    }
    
    /// 执行推理任务
    pub async fn execute_inference(&self, task_id: &str) -> Result<InferenceResult, String> {
        let start_time = Utc::now();
        
        // 获取任务状态
        let mut tasks = self.tasks.write().await;
        let task_state = tasks.get_mut(task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;
        
        task_state.status = InferenceStatus::Running {
            current_node: self.node_id.clone(),
            current_shard: String::new(),
        };
        task_state.started_at = Some(start_time);
        
        let shard_order = task_state.shard_order.clone();
        let mut current_data = task_state.input_data.clone();
        let model_id = task_state.model_id.clone();
        let num_shards = shard_order.len();
        drop(tasks);
        
        // 为任务创建聚合器
        {
            let agg = self.aggregator.read().await;
            agg.create_task(
                task_id.to_string(),
                model_id,
                num_shards,
                self.config.node_timeout_secs as u64 * 1000,
            ).await;
        }
        
        // 按顺序执行每个分片
        for (index, shard_id) in shard_order.iter().enumerate() {
            // 检查缓存
            if let Some(cached) = self.cache.get(task_id, shard_id).await {
                log::info!("💾 [Coordinator] Using cached result for shard {}", shard_id);
                current_data = cached.clone();
                
                // 添加到聚合器（使用缓存的结果）
                let partial = PartialResult {
                    node_id: self.node_id.clone(),
                    shard_id: shard_id.clone(),
                    output_text: String::from_utf8_lossy(&cached).to_string(),
                    confidence: 1.0,
                    execution_time_ms: 0,
                };
                {
                    let agg = self.aggregator.read().await;
                    agg.add_result(task_id, partial).await;
                }
                
                // 更新任务状态
                let mut tasks = self.tasks.write().await;
                if let Some(state) = tasks.get_mut(task_id) {
                    state.current_shard_index = index + 1;
                    state.intermediate_results.insert(shard_id.clone(), current_data.clone());
                }
                continue;
            }
            
            // 查找分片所在节点
            let node_id = {
                let table = self.shard_table.read().await;
                table.locate_shard(shard_id)
                    .map(|s| s.node_id.clone())
                    .ok_or_else(|| format!("Shard {} not found", shard_id))?
            };
            
            // 更新任务状态
            {
                let mut tasks = self.tasks.write().await;
                if let Some(state) = tasks.get_mut(task_id) {
                    state.status = InferenceStatus::Running {
                        current_node: node_id.clone(),
                        current_shard: shard_id.clone(),
                    };
                    state.current_shard_index = index;
                }
            }
            
            // 执行分片
            log::info!("📤 [Coordinator] Executing shard {} on node {}", shard_id, node_id);
            
            let result = self.execute_shard_on_node(&node_id, shard_id, task_id, &current_data).await?;
            
            if !result.success {
                let error = result.error.unwrap_or_else(|| "Unknown error".to_string());
                
                // 更新任务状态为失败
                let mut tasks = self.tasks.write().await;
                if let Some(state) = tasks.get_mut(task_id) {
                    state.status = InferenceStatus::Failed(error.clone());
                    state.error = Some(error.clone());
                }
                
                return Err(error);
            }
            
            // 添加到聚合器
            let partial = PartialResult {
                node_id: node_id.clone(),
                shard_id: shard_id.clone(),
                output_text: String::from_utf8_lossy(&result.output_data).to_string(),
                confidence: 1.0,
                execution_time_ms: result.metrics.execution_time_ms,
            };
            {
                let agg = self.aggregator.read().await;
                let aggregated = agg.add_result(task_id, partial).await;
                if let Some(agg_result) = aggregated {
                    log::info!("✅ [Coordinator] All shards aggregated, method: {:?}", agg_result.method);
                }
            }
            
            // 缓存中间结果
            self.cache.put(task_id, shard_id, result.output_data.clone()).await;
            current_data = result.output_data;
            
            // 更新任务状态
            {
                let mut tasks = self.tasks.write().await;
                if let Some(state) = tasks.get_mut(task_id) {
                    state.intermediate_results.insert(shard_id.clone(), current_data.clone());
                }
            }
        }
        
        // 任务完成
        let completed_at = Utc::now();
        let mut tasks = self.tasks.write().await;
        if let Some(state) = tasks.get_mut(task_id) {
            state.status = InferenceStatus::Completed;
            state.completed_at = Some(completed_at);
        }
        
        log::info!("✅ [Coordinator] Task {} completed with aggregation", task_id);
        
        Ok(InferenceResult {
            task_id: task_id.to_string(),
            output_data: current_data,
            success: true,
            error: None,
            metrics: ExecutionMetrics::default(),
            completed_at,
        })
    }
    
    /// 在指定节点执行分片
    async fn execute_shard_on_node(
        &self,
        node_id: &str,
        shard_id: &str,
        task_id: &str,
        input_data: &[u8],
    ) -> Result<InferenceResult, String> {
        // 如果是本地节点，直接执行
        if node_id == self.node_id {
            return self.execute_shard_locally(shard_id, task_id, input_data).await;
        }
        
        // 远程执行
        let message = InferenceMessage::ExecuteShard {
            shard_id: shard_id.to_string(),
            task_id: task_id.to_string(),
            input_data: input_data.to_vec(),
            metadata: super::protocol::ShardExecutionMetadata {
                model_id: String::new(),
                layer_start: 0,
                layer_end: 0,
                input_shape: vec![],
                timeout_ms: self.config.node_timeout_secs * 1000,
                priority: 5,
            },
        };
        
        // 发送消息到目标节点
        self.send_message_to_node(node_id, message).await?;
        
        // 等待结果
        let result = self.wait_for_result(task_id, shard_id).await?;
        Ok(result)
    }
    
    /// 本地执行分片
    async fn execute_shard_locally(
        &self,
        shard_id: &str,
        task_id: &str,
        input_data: &[u8],
    ) -> Result<InferenceResult, String> {
        // TODO: 实现本地推理执行
        // 这里需要与实际的推理引擎集成
        
        log::info!("🔧 [Coordinator] Executing shard {} locally", shard_id);
        
        // 模拟执行
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(InferenceResult {
            task_id: task_id.to_string(),
            output_data: input_data.to_vec(), // 简单返回输入
            success: true,
            error: None,
            metrics: ExecutionMetrics {
                execution_time_ms: 100,
                ..Default::default()
            },
            completed_at: Utc::now(),
        })
    }
    
    /// 发送消息到节点
    async fn send_message_to_node(&self, node_id: &str, message: InferenceMessage) -> Result<(), String> {
        let cm = self.connection_manager.lock().await;
        if let Some(manager) = cm.as_ref() {
            let data = serde_json::to_vec(&message)
                .map_err(|e| format!("Serialization error: {}", e))?;
            
            // TODO: 实现点对点消息发送
            log::info!("📤 [Coordinator] Sending message to node {}: {} bytes", node_id, data.len());
        }
        Ok(())
    }
    
    /// 等待执行结果
    async fn wait_for_result(&self, task_id: &str, shard_id: &str) -> Result<InferenceResult, String> {
        // TODO: 实现结果等待机制
        // 这里应该监听来自其他节点的响应
        
        // 模拟等待
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(InferenceResult {
            task_id: task_id.to_string(),
            output_data: vec![],
            success: true,
            error: None,
            metrics: ExecutionMetrics::default(),
            completed_at: Utc::now(),
        })
    }
    
    /// 获取任务状态
    pub async fn get_task_status(&self, task_id: &str) -> Option<InferenceTaskState> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }
    
    /// 取消任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.write().await;
        if let Some(state) = tasks.get_mut(task_id) {
            state.status = InferenceStatus::Cancelled;
            log::info!("🚫 [Coordinator] Task {} cancelled", task_id);
        }
        Ok(())
    }
    
    /// 获取节点列表
    pub async fn get_nodes(&self) -> Vec<NodeStatus> {
        let status = self.node_status.read().await;
        status.values().cloned().collect()
    }
    
    /// 更新节点状态
    pub async fn update_node_status(&self, node_id: &str, status: NodeStatus) {
        let mut node_status = self.node_status.write().await;
        node_status.insert(node_id.to_string(), status);
    }
    
    /// 启动心跳服务
    async fn start_heartbeat_service(&self) {
        let node_status = self.node_status.clone();
        let is_running = self.is_running.clone();
        let interval_secs = self.config.heartbeat_interval_secs;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            
            loop {
                interval.tick().await;
                
                if !*is_running.read().await {
                    break;
                }
                
                // 检查节点超时
                let mut status = node_status.write().await;
                let now = Utc::now();
                for node in status.values_mut() {
                    let elapsed = (now - node.last_heartbeat).num_seconds();
                    if elapsed > 60 {
                        node.online = false;
                        log::warn!("⚠️ [Coordinator] Node {} offline (no heartbeat for {}s)", node.node_id, elapsed);
                    }
                }
            }
        });
    }
    
    /// 启动任务调度器
    async fn start_task_scheduler(&self) {
        let tasks = self.tasks.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                if !*is_running.read().await {
                    break;
                }
                
                // 检查待执行任务
                let task_list = tasks.read().await;
                for (task_id, state) in task_list.iter() {
                    if state.status == InferenceStatus::Pending {
                        log::info!("📋 [Coordinator] Found pending task: {}", task_id);
                    }
                }
            }
        });
    }
    
    /// 使用 AI 优化分片调度
    pub async fn optimize_schedule(&self, model_id: &str) -> Result<(), String> {
        if let Some(ai_decision) = &self.ai_decision {
            // 收集节点信息
            let nodes = self.get_nodes().await;
            
            // 构建上下文
            let context = ExecutionContext {
                iteration: 0,
                completed_steps: vec![],
                current_step: Some("optimize_schedule".to_string()),
                execution_history: vec![],
                learned_knowledge: serde_json::json!({
                    "nodes": nodes,
                    "model_id": model_id,
                }),
                acceptance_criteria: vec![
                    AcceptanceCriterion {
                        id: "balanced_load".to_string(),
                        description: "负载均衡".to_string(),
                        completed: false,
                        evidence: None,
                    },
                ],
            };
            
            log::info!("🤖 [Coordinator] AI optimizing schedule for model {}", model_id);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_coordinator_creation() {
        let coordinator = DistributedInferenceCoordinator::new(
            "node_1".to_string(),
            CoordinatorConfig::default(),
        );
        
        assert!(!coordinator.config.enabled);
    }
    
    #[tokio::test]
    async fn test_register_shards() {
        let coordinator = DistributedInferenceCoordinator::new(
            "node_1".to_string(),
            CoordinatorConfig::default(),
        );
        
        let shard = ShardInfo {
            shard_id: "shard_0".to_string(),
            model_id: "model_1".to_string(),
            node_id: "node_1".to_string(),
            layer_range: (0, 10),
            size_bytes: 1024,
            checksum: "abc".to_string(),
            status: ShardStatus::Ready,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        coordinator.register_model_shards("model_1", vec![shard]).await.unwrap();
        
        let nodes = coordinator.get_nodes().await;
        assert_eq!(nodes.len(), 1);
    }
    
    #[tokio::test]
    async fn test_submit_task() {
        let coordinator = DistributedInferenceCoordinator::new(
            "node_1".to_string(),
            CoordinatorConfig::default(),
        );
        
        // 先注册分片
        let shard = ShardInfo {
            shard_id: "shard_0".to_string(),
            model_id: "model_1".to_string(),
            node_id: "node_1".to_string(),
            layer_range: (0, 10),
            size_bytes: 1024,
            checksum: "abc".to_string(),
            status: ShardStatus::Ready,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        coordinator.register_model_shards("model_1", vec![shard]).await.unwrap();
        
        // 提交任务
        let request = InferenceRequest {
            task_id: "task_1".to_string(),
            model_id: "model_1".to_string(),
            input_data: vec![1, 2, 3],
            config: InferenceConfig::default(),
        };
        
        let result = coordinator.submit_task(request).await;
        assert!(result.is_ok());
    }
}

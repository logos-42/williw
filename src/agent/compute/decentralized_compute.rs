//! 去中心化算力共享网络
//!
//! 实现P2P算力共享、任务分发、结果聚合和激励机制

use super::gpu_manager::{GpuManager, TaskStatus, NodeStatus};
use crate::comms::transport::iroh::{IrohConnectionManager, WrappedMessage, IrohConnectionConfig};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, mpsc};
use tokio::time::{interval, Duration};
use uuid::Uuid;

/// 去中心化计算网络
pub struct DecentralizedComputeNetwork {
    /// 本地节点ID
    node_id: String,
    /// GPU管理器
    gpu_manager: Arc<GpuManager>,
    /// 任务调度器
    task_scheduler: Arc<TaskScheduler>,
    /// 结果聚合器
    result_aggregator: Arc<ResultAggregator>,
    /// 激励机制
    incentive_system: Arc<IncentiveSystem>,
    /// 网络状态
    network_state: Arc<RwLock<NetworkState>>,
    /// 消息处理器
    message_handler: Arc<MessageHandler>,
    /// Iroh连接管理器
    connection_manager: Arc<Mutex<Option<Arc<IrohConnectionManager>>>>,
    /// 运行状态
    is_running: Arc<RwLock<bool>>,
}

/// 网络状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub total_compute_power: f64,
    pub network_load: f32,
    pub average_latency_ms: f64,
    pub tasks_in_flight: usize,
    pub completed_tasks_total: u64,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            active_nodes: 0,
            total_compute_power: 0.0,
            network_load: 0.0,
            average_latency_ms: 0.0,
            tasks_in_flight: 0,
            completed_tasks_total: 0,
        }
    }
}

/// 任务调度器
pub struct TaskScheduler {
    /// 任务队列（按优先级排序）
    task_queue: Arc<RwLock<VecDeque<ComputeTask>>>,
    /// 调度策略
    strategy: Arc<RwLock<SchedulingStrategy>>,
    /// 节点性能评分
    node_scores: Arc<RwLock<HashMap<String, f32>>>,
}

/// 计算任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeTask {
    pub task_id: String,
    pub task_type: ComputeTaskType,
    pub payload: TaskPayload,
    pub requirements: ComputeRequirements,
    pub priority: TaskPriority,
    pub deadline: Option<i64>,
    pub requester: String,
    pub assigned_node: Option<String>,
    pub status: TaskStatus,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub subtasks: Vec<SubTask>,
    pub results: Vec<TaskResult>,
}

/// 计算任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeTaskType {
    ModelTraining,
    ModelInference,
    DataProcessing,
    FederatedLearning,
    DistributedTraining,
    Custom(String),
}

/// 任务负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    pub model_id: Option<String>,
    pub data_hash: String,
    pub config: serde_json::Value,
    pub checkpoints: Vec<String>,
}

/// 计算需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRequirements {
    pub min_gpu_memory_gb: f32,
    pub min_cpu_cores: u32,
    pub min_memory_gb: f32,
    pub estimated_duration_minutes: u32,
    pub requires_internet: bool,
    pub preferred_regions: Vec<String>,
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// 子任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub subtask_id: String,
    pub parent_task_id: String,
    pub payload: serde_json::Value,
    pub assigned_node: Option<String>,
    pub status: TaskStatus,
    pub result: Option<TaskResult>,
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub node_id: String,
    pub status: ResultStatus,
    pub data: serde_json::Value,
    pub metrics: ExecutionMetrics,
    pub timestamp: i64,
}

/// 结果状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultStatus {
    Success,
    PartialSuccess,
    Failed,
    Timeout,
}

/// 执行指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetrics {
    pub execution_time_ms: u64,
    pub gpu_utilization_percent: f32,
    pub memory_peak_mb: u64,
    pub energy_consumption_wh: f32,
    pub data_transferred_mb: f64,
}

/// 调度策略
#[derive(Debug, Clone)]
pub struct SchedulingStrategy {
    pub load_balance_weight: f32,
    pub locality_weight: f32,
    pub cost_weight: f32,
    pub reliability_weight: f32,
    pub enable_preemption: bool,
    pub max_retries: u32,
}

impl Default for SchedulingStrategy {
    fn default() -> Self {
        Self {
            load_balance_weight: 0.3,
            locality_weight: 0.25,
            cost_weight: 0.2,
            reliability_weight: 0.25,
            enable_preemption: true,
            max_retries: 3,
        }
    }
}

/// 结果聚合器
pub struct ResultAggregator {
    /// 待聚合的任务
    pending_aggregations: Arc<RwLock<HashMap<String, Vec<TaskResult>>>>,
    /// 聚合策略
    aggregation_policies: Arc<RwLock<HashMap<String, AggregationPolicy>>>,
}

/// 聚合策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationPolicy {
    /// 取最佳结果
    BestResult,
    /// 平均结果
    Average,
    /// 投票机制
    Voting,
    /// 加权平均
    WeightedAverage(Vec<f32>),
    /// 自定义
    Custom(String),
}

/// 激励机制
pub struct IncentiveSystem {
    /// 节点贡献记录
    contributions: Arc<RwLock<HashMap<String, NodeContribution>>>,
    /// 奖励池
    reward_pool: Arc<RwLock<f64>>,
    /// 声誉评分
    reputation_scores: Arc<RwLock<HashMap<String, f32>>>,
}

/// 节点贡献
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeContribution {
    pub node_id: String,
    pub tasks_completed: u64,
    pub compute_hours: f64,
    pub data_processed_gb: f64,
    pub total_rewards: f64,
    pub reputation_score: f32,
    pub last_contribution: i64,
}

/// 消息处理器
pub struct MessageHandler {
    /// 消息队列
    message_queue: Arc<Mutex<mpsc::Receiver<NetworkMessage>>>,
    /// 消息发送器
    message_sender: mpsc::Sender<NetworkMessage>,
}

/// 网络消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// 任务请求
    TaskRequest(ComputeTask),
    /// 任务响应
    TaskResponse(TaskResult),
    /// 心跳
    Heartbeat { node_id: String, timestamp: i64, load: f32 },
    /// 节点加入
    NodeJoin { node_id: String, capabilities: NodeCapabilities },
    /// 节点离开
    NodeLeave { node_id: String },
    /// 状态更新
    StatusUpdate { node_id: String, status: NodeStatus },
}

/// 节点能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub gpu_count: u32,
    pub total_gpu_memory_gb: f32,
    pub cpu_cores: u32,
    pub total_memory_gb: f32,
    pub network_bandwidth_mbps: f32,
    pub supported_task_types: Vec<String>,
}

impl DecentralizedComputeNetwork {
    /// 创建去中心化计算网络
    pub async fn new(node_id: String) -> Result<Self, String> {
        let gpu_manager = Arc::new(GpuManager::new(node_id.clone()).await?);
        
        let (tx, rx) = mpsc::channel(1000);
        
        let network = Self {
            node_id: node_id.clone(),
            gpu_manager,
            task_scheduler: Arc::new(TaskScheduler {
                task_queue: Arc::new(RwLock::new(VecDeque::new())),
                strategy: Arc::new(RwLock::new(SchedulingStrategy::default())),
                node_scores: Arc::new(RwLock::new(HashMap::new())),
            }),
            result_aggregator: Arc::new(ResultAggregator {
                pending_aggregations: Arc::new(RwLock::new(HashMap::new())),
                aggregation_policies: Arc::new(RwLock::new(HashMap::new())),
            }),
            incentive_system: Arc::new(IncentiveSystem {
                contributions: Arc::new(RwLock::new(HashMap::new())),
                reward_pool: Arc::new(RwLock::new(0.0)),
                reputation_scores: Arc::new(RwLock::new(HashMap::new())),
            }),
            network_state: Arc::new(RwLock::new(NetworkState::default())),
            message_handler: Arc::new(MessageHandler {
                message_queue: Arc::new(Mutex::new(rx)),
                message_sender: tx,
            }),
            connection_manager: Arc::new(Mutex::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        };

        Ok(network)
    }

    /// 启动网络
    pub async fn start(&self) -> Result<(), String> {
        println!("🚀 [DCN] Starting Decentralized Compute Network...");

        // 初始化网络
        self.initialize_network().await?;

        // 设置运行状态
        *self.is_running.write().await = true;

        // 启动各个服务
        self.start_task_scheduler().await;
        self.start_message_handler().await;
        self.start_heartbeat_service().await;
        self.start_result_aggregator().await;

        println!("✅ [DCN] Network started successfully");
        Ok(())
    }

    /// 初始化网络
    async fn initialize_network(&self) -> Result<(), String> {
        // 初始化Iroh连接
        let config = IrohConnectionConfig::default();
        match IrohConnectionManager::new(config).await {
            Ok(manager) => {
                let mut cm = self.connection_manager.lock().await;
                *cm = Some(Arc::new(manager));
                println!("✅ [DCN] Network layer initialized");
            }
            Err(e) => {
                println!("⚠️ [DCN] Failed to initialize network layer: {}", e);
            }
        }

        // 发现节点
        self.discover_nodes().await?;

        Ok(())
    }

    /// 发现节点
    async fn discover_nodes(&self) -> Result<(), String> {
        println!("🔍 [DCN] Discovering network nodes...");

        // 广播发现请求
        let discovery_msg = NetworkMessage::NodeJoin {
            node_id: self.node_id.clone(),
            capabilities: NodeCapabilities {
                gpu_count: 1,
                total_gpu_memory_gb: 8.0,
                cpu_cores: 8,
                total_memory_gb: 16.0,
                network_bandwidth_mbps: 100.0,
                supported_task_types: vec![
                    "training".to_string(),
                    "inference".to_string(),
                ],
            },
        };

        self.broadcast_message(&discovery_msg).await?;

        // 从种子节点获取
        if let Ok(seed_nodes) = std::env::var("WILLIW_COMPUTE_SEEDS") {
            for seed in seed_nodes.split(',') {
                println!("🌱 [DCN] Found seed node: {}", seed.trim());
            }
        }

        Ok(())
    }

    /// 广播消息
    async fn broadcast_message(&self, message: &NetworkMessage) -> Result<(), String> {
        if let Some(cm) = self.connection_manager.lock().await.as_ref() {
            let data = serde_json::to_vec(message)
                .map_err(|e| format!("Serialization error: {}", e))?;
            
            let wrapped = WrappedMessage::new(
                "dcn_message".to_string(),
                self.node_id.clone(),
                data
            );

            cm.broadcast_message(wrapped.serialize().unwrap_or_default()).await
                .map_err(|e| format!("Broadcast error: {}", e))?;
        }

        Ok(())
    }

    /// 提交计算任务
    pub async fn submit_compute_task(&self, mut task: ComputeTask) -> Result<String, String> {
        task.task_id = format!("compute_{}", Uuid::new_v4());
        task.created_at = chrono::Utc::now().timestamp();
        task.status = TaskStatus::Pending;

        // 切分任务
        let subtasks = self.split_task(&task).await?;
        task.subtasks = subtasks;

        // 添加到队列
        {
            let mut queue = self.task_scheduler.task_queue.write().await;
            queue.push_back(task.clone());
            // 按优先级排序
            queue.make_contiguous().sort_by_key(|t| std::cmp::Reverse(t.priority.clone() as u8));
        }

        // 更新网络状态
        {
            let mut state = self.network_state.write().await;
            state.tasks_in_flight += 1;
        }

        println!("📋 [DCN] Task {} submitted with {} subtasks", task.task_id, task.subtasks.len());

        Ok(task.task_id)
    }

    /// 切分任务
    async fn split_task(&self, task: &ComputeTask) -> Result<Vec<SubTask>, String> {
        let mut subtasks = Vec::new();

        // 根据任务类型决定切分策略
        match task.task_type {
            ComputeTaskType::ModelTraining => {
                // 数据并行切分
                let num_shards = 4; // 可以根据可用节点数动态调整
                for i in 0..num_shards {
                    subtasks.push(SubTask {
                        subtask_id: format!("{}_shard_{}", task.task_id, i),
                        parent_task_id: task.task_id.clone(),
                        payload: serde_json::json!({
                            "shard_index": i,
                            "total_shards": num_shards,
                            "data_range": format!("{}% to {}%", i * 25, (i + 1) * 25),
                        }),
                        assigned_node: None,
                        status: TaskStatus::Pending,
                        result: None,
                    });
                }
            }
            ComputeTaskType::ModelInference => {
                // 简单切分：每个子任务处理一批数据
                subtasks.push(SubTask {
                    subtask_id: format!("{}_inference", task.task_id),
                    parent_task_id: task.task_id.clone(),
                    payload: task.payload.config.clone(),
                    assigned_node: None,
                    status: TaskStatus::Pending,
                    result: None,
                });
            }
            _ => {
                // 默认不切分
                subtasks.push(SubTask {
                    subtask_id: format!("{}_single", task.task_id),
                    parent_task_id: task.task_id.clone(),
                    payload: task.payload.config.clone(),
                    assigned_node: None,
                    status: TaskStatus::Pending,
                    result: None,
                });
            }
        }

        Ok(subtasks)
    }

    /// 启动任务调度器
    async fn start_task_scheduler(&self) {
        let scheduler = self.task_scheduler.clone();
        let _gpu_manager = self.gpu_manager.clone();
        let is_running = self.is_running.clone();
        let _network_state = self.network_state.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                if !*is_running.read().await {
                    break;
                }

                // 获取待调度任务
                let tasks_to_schedule: Vec<ComputeTask> = {
                    let mut queue = scheduler.task_queue.write().await;
                    let len = queue.len().min(10);
                    queue.drain(..len).collect()
                };

                for mut task in tasks_to_schedule {
                    // 调度每个子任务
                    let mut updated_subtasks = Vec::new();
                    for mut subtask in task.subtasks.clone().into_iter() {
                        if subtask.status == TaskStatus::Pending {
                            // 寻找最佳节点 - use a temporary task for the function call
                            let temp_task = task.clone();
                            if let Some(best_node) = Self::find_best_node(&scheduler, &temp_task).await {
                                subtask.assigned_node = Some(best_node.clone());
                                subtask.status = TaskStatus::Scheduled;
                                
                                println!("📤 [DCN] Subtask {} assigned to node {}", 
                                    subtask.subtask_id, best_node);
                            }
                        }
                        updated_subtasks.push(subtask);
                    }
                    task.subtasks = updated_subtasks;
                }
            }
        });
    }

    /// 寻找最佳节点
    async fn find_best_node(scheduler: &TaskScheduler, _task: &ComputeTask) -> Option<String> {
        let scores = scheduler.node_scores.read().await;
        let _strategy = scheduler.strategy.read().await;
        
        // 选择评分最高的节点（简化实现）
        scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(node_id, _)| node_id.clone())
    }

    /// 启动消息处理器
    async fn start_message_handler(&self) {
        let message_queue = self.message_handler.message_queue.clone();
        let is_running = self.is_running.clone();
        let incentive_system = self.incentive_system.clone();

        tokio::spawn(async move {
            let mut queue = message_queue.lock().await;
            
            while *is_running.read().await {
                if let Some(message) = queue.recv().await {
                    match message {
                        NetworkMessage::TaskResponse(result) => {
                            println!("📥 [DCN] Received task result from {}", result.node_id);
                            
                            // 更新贡献记录
                            let mut contributions = incentive_system.contributions.write().await;
                            let contribution = contributions.entry(result.node_id.clone())
                                .or_insert_with(|| NodeContribution {
                                    node_id: result.node_id.clone(),
                                    ..Default::default()
                                });
                            contribution.tasks_completed += 1;
                            contribution.last_contribution = chrono::Utc::now().timestamp();
                        }
                        NetworkMessage::Heartbeat { node_id, load, .. } => {
                            println!("💓 [DCN] Heartbeat from {} (load: {:.1}%)", node_id, load * 100.0);
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    /// 启动心跳服务
    async fn start_heartbeat_service(&self) {
        let connection_manager = self.connection_manager.clone();
        let node_id = self.node_id.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                if !*is_running.read().await {
                    break;
                }

                let heartbeat = NetworkMessage::Heartbeat {
                    node_id: node_id.clone(),
                    timestamp: chrono::Utc::now().timestamp(),
                    load: 0.5, // 简化：假设50%负载
                };

                if let Some(cm) = connection_manager.lock().await.as_ref() {
                    if let Ok(data) = serde_json::to_vec(&heartbeat) {
                        let wrapped = WrappedMessage::new(
                            "heartbeat".to_string(),
                            node_id.clone(),
                            data
                        );
                        
                        let _ = cm.broadcast_message(wrapped.serialize().unwrap_or_default()).await;
                    }
                }
            }
        });
    }

    /// 启动结果聚合器
    async fn start_result_aggregator(&self) {
        let aggregator = self.result_aggregator.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                if !*is_running.read().await {
                    break;
                }

                // 检查待聚合的任务
                let mut pending = aggregator.pending_aggregations.write().await;
                let ready_tasks: Vec<String> = pending.iter()
                    .filter(|(_, results)| results.len() >= 3) // 至少需要3个结果
                    .map(|(task_id, _)| task_id.clone())
                    .collect();

                for task_id in ready_tasks {
                    if let Some(results) = pending.remove(&task_id) {
                        let aggregated = Self::aggregate_results(&results).await;
                        println!("🔄 [DCN] Aggregated results for task {}: {:?}", 
                            task_id, aggregated.status);
                    }
                }
            }
        });
    }

    /// 聚合结果
    async fn aggregate_results(results: &[TaskResult]) -> TaskResult {
        // 简化实现：选择第一个成功结果
        results.iter()
            .find(|r| matches!(r.status, ResultStatus::Success))
            .cloned()
            .unwrap_or_else(|| TaskResult {
                task_id: results[0].task_id.clone(),
                node_id: "aggregator".to_string(),
                status: ResultStatus::Failed,
                data: serde_json::json!({"error": "No successful results"}),
                metrics: ExecutionMetrics::default(),
                timestamp: chrono::Utc::now().timestamp(),
            })
    }

    /// 停止网络
    pub async fn stop(&self) -> Result<(), String> {
        println!("🛑 [DCN] Stopping network...");
        
        *self.is_running.write().await = false;
        
        // 广播离开消息
        let leave_msg = NetworkMessage::NodeLeave {
            node_id: self.node_id.clone(),
        };
        self.broadcast_message(&leave_msg).await?;

        println!("✅ [DCN] Network stopped");
        Ok(())
    }

    /// 获取网络状态
    pub async fn get_network_state(&self) -> NetworkState {
        self.network_state.read().await.clone()
    }

    /// 获取节点贡献统计
    pub async fn get_contributions(&self) -> Vec<NodeContribution> {
        let contributions = self.incentive_system.contributions.read().await;
        contributions.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_decentralized_network_creation() {
        let network = DecentralizedComputeNetwork::new("test_node".to_string()).await;
        assert!(network.is_ok());
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_network_message_serialization() {
        let msg = NetworkMessage::Heartbeat {
            node_id: "test".to_string(),
            timestamp: 1234567890,
            load: 0.5,
        };
        
        let serialized = serde_json::to_string(&msg);
        assert!(serialized.is_ok());
    }
}

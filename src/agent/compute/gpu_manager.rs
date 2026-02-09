//! GPU计算节点管理器
//!
//! 管理GPU节点的发现、调度、监控和算力共享

use crate::device::capabilities::{DeviceCapabilities, GpuComputeApi};
use crate::comms::transport::iroh::{IrohConnectionManager, WrappedMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, Duration};
use uuid::Uuid;

/// GPU节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuNode {
    pub node_id: String,
    pub peer_id: String,
    pub device_info: DeviceCapabilities,
    pub gpu_info: Vec<GpuDevice>,
    pub status: NodeStatus,
    pub last_heartbeat: i64,
    pub current_tasks: Vec<String>,
    pub performance_metrics: PerformanceMetrics,
    pub location: NodeLocation,
    pub pricing: ComputePricing,
}

/// GPU设备详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub index: u32,
    pub name: String,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub compute_capability: String,
    pub driver_version: String,
    pub cuda_version: Option<String>,
    pub metal_support: bool,
    pub vulkan_support: bool,
}

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Online,
    Offline,
    Busy,
    Maintenance,
    Degraded,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub avg_task_duration_ms: f64,
    pub avg_gpu_utilization: f32,
    pub avg_memory_utilization: f32,
    pub network_latency_ms: f64,
    pub reliability_score: f32,
}

/// 节点位置信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeLocation {
    pub region: String,
    pub datacenter: Option<String>,
    pub network_zone: String,
}

/// 计算定价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputePricing {
    pub price_per_hour_usd: f64,
    pub price_per_gb_memory_usd: f64,
    pub min_duration_minutes: u32,
    pub discount_for_long_term: f32,
}

impl Default for ComputePricing {
    fn default() -> Self {
        Self {
            price_per_hour_usd: 0.5,
            price_per_gb_memory_usd: 0.01,
            min_duration_minutes: 15,
            discount_for_long_term: 0.1,
        }
    }
}

/// GPU任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuTask {
    pub task_id: String,
    pub requester_node_id: String,
    pub target_node_id: Option<String>,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub resource_requirements: ResourceRequirements,
    pub payload: serde_json::Value,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    ModelInference,
    ModelTraining,
    DataPreprocessing,
    ModelEvaluation,
    Custom(String),
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum TaskPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

/// 资源需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_gpu_memory_mb: u64,
    pub min_cpu_cores: u32,
    pub min_memory_mb: u64,
    pub max_duration_minutes: u32,
    pub preferred_gpu_type: Option<String>,
    pub allow_cpu_fallback: bool,
}

/// GPU管理器
pub struct GpuManager {
    /// 本地节点ID
    local_node_id: String,
    /// 已知GPU节点
    nodes: Arc<RwLock<HashMap<String, GpuNode>>>,
    /// 任务队列
    task_queue: Arc<RwLock<Vec<GpuTask>>>,
    /// 活跃任务
    active_tasks: Arc<RwLock<HashMap<String, GpuTask>>>,
    /// 任务历史
    task_history: Arc<RwLock<Vec<GpuTask>>>,
    /// Iroh连接管理器
    connection_manager: Arc<Mutex<Option<Arc<IrohConnectionManager>>>>,
    /// 调度器配置
    scheduler_config: SchedulerConfig,
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub enable_auto_scaling: bool,
    pub max_concurrent_tasks_per_node: u32,
    pub heartbeat_interval_seconds: u64,
    pub task_timeout_minutes: u32,
    pub enable_load_balancing: bool,
    pub preferred_local_execution: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enable_auto_scaling: true,
            max_concurrent_tasks_per_node: 4,
            heartbeat_interval_seconds: 30,
            task_timeout_minutes: 60,
            enable_load_balancing: true,
            preferred_local_execution: true,
        }
    }
}

impl GpuManager {
    /// 创建新的GPU管理器
    pub async fn new(local_node_id: String) -> Result<Self, String> {
        Ok(Self {
            local_node_id,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_history: Arc::new(RwLock::new(Vec::new())),
            connection_manager: Arc::new(Mutex::new(None)),
            scheduler_config: SchedulerConfig::default(),
        })
    }

    /// 初始化Iroh连接
    pub async fn initialize_network(&self) -> Result<(), String> {
        let config = crate::comms::transport::iroh::IrohConnectionConfig::default();
        
        match IrohConnectionManager::new(config).await {
            Ok(manager) => {
                let mut cm = self.connection_manager.lock().await;
                *cm = Some(Arc::new(manager));
                println!("✅ [GPU-MANAGER] Network initialized successfully");
                Ok(())
            }
            Err(e) => {
                println!("⚠️ [GPU-MANAGER] Network initialization failed: {}", e);
                Err(format!("Failed to initialize network: {}", e))
            }
        }
    }

    /// 注册本地GPU节点
    pub async fn register_local_node(&self, device_info: DeviceCapabilities) -> Result<(), String> {
        let gpu_info = Self::detect_local_gpus().await?;
        
        let node = GpuNode {
            node_id: self.local_node_id.clone(),
            peer_id: self.local_node_id.clone(),
            device_info,
            gpu_info,
            status: NodeStatus::Online,
            last_heartbeat: chrono::Utc::now().timestamp(),
            current_tasks: Vec::new(),
            performance_metrics: PerformanceMetrics::default(),
            location: NodeLocation::default(),
            pricing: ComputePricing::default(),
        };

        let mut nodes = self.nodes.write().await;
        nodes.insert(self.local_node_id.clone(), node);
        
        println!("✅ [GPU-MANAGER] Local node registered with {} GPUs", 
            nodes.get(&self.local_node_id).unwrap().gpu_info.len());
        
        Ok(())
    }

    /// 检测本地GPU
    async fn detect_local_gpus() -> Result<Vec<GpuDevice>, String> {
        let mut gpus = Vec::new();

        // 检测NVIDIA GPU
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args([
                    "--query-gpu=index,name,memory.total,memory.free,compute_cap,driver_version",
                    "--format=csv,noheader"
                ])
                .output()
            {
                if let Ok(gpu_info) = String::from_utf8(output.stdout) {
                    for line in gpu_info.lines() {
                        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 6 {
                            if let Ok(index) = parts[0].parse::<u32>() {
                                let total_mem = parts[2].replace("MiB", "").replace("MB", "")
                                    .parse::<u64>().unwrap_or(0);
                                let free_mem = parts[3].replace("MiB", "").replace("MB", "")
                                    .parse::<u64>().unwrap_or(0);
                                
                                gpus.push(GpuDevice {
                                    index,
                                    name: parts[1].to_string(),
                                    total_memory_mb: total_mem,
                                    available_memory_mb: free_mem,
                                    compute_capability: parts[4].to_string(),
                                    driver_version: parts[5].to_string(),
                                    cuda_version: None,
                                    metal_support: false,
                                    vulkan_support: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 检测Apple Silicon GPU
        #[cfg(target_os = "macos")]
        {
            if gpus.is_empty() {
                if let Ok(output) = std::process::Command::new("system_profiler")
                    .args(["SPDisplaysDataType"])
                    .output()
                {
                    if let Ok(display_info) = String::from_utf8(output.stdout) {
                        if display_info.contains("Apple") {
                            // 获取内存信息用于计算可用显存
                            let total_mem = if let Ok(mem_output) = std::process::Command::new("sysctl")
                                .args(["-n", "hw.memsize"])
                                .output()
                            {
                                String::from_utf8_lossy(&mem_output.stdout)
                                    .trim()
                                    .parse::<u64>()
                                    .unwrap_or(16 * 1024 * 1024 * 1024) / 1024 / 1024
                            } else {
                                16384 // 默认16GB
                            };

                            gpus.push(GpuDevice {
                                index: 0,
                                name: "Apple Silicon GPU".to_string(),
                                total_memory_mb: total_mem,
                                available_memory_mb: total_mem / 2, // 假设一半可用
                                compute_capability: "Metal 3".to_string(),
                                driver_version: "System".to_string(),
                                cuda_version: None,
                                metal_support: true,
                                vulkan_support: true,
                            });
                        }
                    }
                }
            }
        }

        Ok(gpus)
    }

    /// 发现远程GPU节点
    pub async fn discover_remote_nodes(&self) -> Result<Vec<String>, String> {
        println!("🔍 [GPU-MANAGER] Discovering remote GPU nodes...");
        
        // 通过Iroh广播发现请求
        let discovery_msg = serde_json::json!({
            "message_type": "GPU_NODE_DISCOVERY",
            "sender_id": self.local_node_id,
            "timestamp": chrono::Utc::now().timestamp(),
            "capabilities": {
                "has_gpu": true,
                "task_types": ["inference", "training", "preprocessing"]
            }
        });

        if let Some(cm) = self.connection_manager.lock().await.as_ref() {
            let wrapped = WrappedMessage::new(
                "gpu_discovery".to_string(),
                self.local_node_id.clone(),
                discovery_msg.to_string().into_bytes()
            );

            match cm.broadcast_message(wrapped.serialize().unwrap_or_default()).await {
                Ok(count) => {
                    println!("📡 [GPU-MANAGER] Discovery message broadcast to {} peers", count);
                }
                Err(e) => {
                    println!("⚠️ [GPU-MANAGER] Failed to broadcast discovery: {}", e);
                }
            }
        }

        // 从环境变量获取种子节点
        let mut discovered = Vec::new();
        if let Ok(seed_nodes) = std::env::var("WILLIW_GPU_SEED_NODES") {
            for node in seed_nodes.split(',') {
                let node = node.trim();
                if !node.is_empty() {
                    discovered.push(node.to_string());
                    println!("🌱 [GPU-MANAGER] Found seed node: {}", node);
                }
            }
        }

        Ok(discovered)
    }

    /// 提交GPU任务
    pub async fn submit_task(&self, mut task: GpuTask) -> Result<String, String> {
        task.task_id = format!("task_{}", Uuid::new_v4());
        task.created_at = chrono::Utc::now().timestamp();
        task.status = TaskStatus::Pending;

        // 尝试调度任务
        let scheduled = self.schedule_task(&task).await?;
        
        if scheduled {
            let mut active = self.active_tasks.write().await;
            active.insert(task.task_id.clone(), task.clone());
            println!("✅ [GPU-MANAGER] Task {} scheduled and started", task.task_id);
        } else {
            let mut queue = self.task_queue.write().await;
            queue.push(task.clone());
            queue.sort_by_key(|t| t.priority.clone() as u8);
            println!("⏳ [GPU-MANAGER] Task {} added to queue (position: {})", 
                task.task_id, queue.len());
        }

        Ok(task.task_id)
    }

    /// 调度任务到合适的节点
    async fn schedule_task(&self, task: &GpuTask) -> Result<bool, String> {
        let nodes = self.nodes.read().await;
        
        // 选择最佳节点
        let best_node = nodes.values()
            .filter(|n| n.status == NodeStatus::Online)
            .filter(|n| Self::can_handle_task(n, &task.resource_requirements))
            .min_by_key(|n| n.current_tasks.len());

        if let Some(node) = best_node {
            if node.node_id == self.local_node_id {
                // 本地执行
                println!("🖥️ [GPU-MANAGER] Executing task {} locally", task.task_id);
                self.execute_local_task(task.clone()).await?;
            } else {
                // 远程执行
                println!("🌐 [GPU-MANAGER] Delegating task {} to node {}", 
                    task.task_id, node.node_id);
                self.delegate_remote_task(task, &node.node_id).await?;
            }
            return Ok(true);
        }

        Ok(false) // 没有可用节点，加入队列
    }

    /// 检查节点是否能处理任务
    fn can_handle_task(node: &GpuNode, requirements: &ResourceRequirements) -> bool {
        // 检查GPU内存
        let total_gpu_memory: u64 = node.gpu_info.iter()
            .map(|g| g.available_memory_mb)
            .sum();
        
        if total_gpu_memory < requirements.min_gpu_memory_mb {
            return false;
        }

        // 检查CPU核心
        if node.device_info.cpu_cores < requirements.min_cpu_cores {
            return false;
        }

        // 检查内存
        if node.device_info.max_memory_mb < requirements.min_memory_mb {
            return false;
        }

        // 检查并发任务数
        if node.current_tasks.len() >= 4 { // 假设最多4个并发
            return false;
        }

        true
    }

    /// 执行本地任务
    async fn execute_local_task(&self, mut task: GpuTask) -> Result<(), String> {
        task.status = TaskStatus::Running;
        task.started_at = Some(chrono::Utc::now().timestamp());

        // 模拟任务执行（实际实现应调用具体的GPU计算代码）
        let task_id = task.task_id.clone();
        let active_tasks = self.active_tasks.clone();
        let task_history = self.task_history.clone();
        let nodes = self.nodes.clone();
        let local_node_id = self.local_node_id.clone();

        tokio::spawn(async move {
            // 模拟任务执行时间
            tokio::time::sleep(Duration::from_secs(5)).await;

            // 更新任务状态
            let mut active = active_tasks.write().await;
            if let Some(mut task) = active.remove(&task_id) {
                task.status = TaskStatus::Completed;
                task.completed_at = Some(chrono::Utc::now().timestamp());
                task.result = Some(serde_json::json!({
                    "status": "success",
                    "message": "Task completed successfully",
                    "execution_time_sec": 5
                }));

                // 移动到历史
                let mut history = task_history.write().await;
                history.push(task.clone());

                // 更新节点状态
                let mut nodes_guard = nodes.write().await;
                if let Some(node) = nodes_guard.get_mut(&local_node_id) {
                    node.current_tasks.retain(|t| t != &task_id);
                    node.performance_metrics.tasks_completed += 1;
                }

                println!("✅ [GPU-MANAGER] Local task {} completed", task_id);
            }
        });

        Ok(())
    }

    /// 委托远程任务
    async fn delegate_remote_task(&self, task: &GpuTask, target_node_id: &str) -> Result<(), String> {
        let task_msg = serde_json::json!({
            "message_type": "GPU_TASK_DELEGATE",
            "task": task,
            "sender_id": self.local_node_id,
        });

        if let Some(cm) = self.connection_manager.lock().await.as_ref() {
            let wrapped = WrappedMessage::new(
                "gpu_task".to_string(),
                self.local_node_id.clone(),
                task_msg.to_string().into_bytes()
            );

            match cm.send_message(target_node_id, wrapped.serialize().unwrap_or_default()).await {
                Ok(_) => {
                    println!("📤 [GPU-MANAGER] Task delegated to {}", target_node_id);
                    Ok(())
                }
                Err(e) => {
                    Err(format!("Failed to delegate task: {}", e))
                }
            }
        } else {
            Err("Connection manager not initialized".to_string())
        }
    }

    /// 启动心跳监控
    pub async fn start_heartbeat_monitor(&self) {
        let nodes = self.nodes.clone();
        let interval_seconds = self.scheduler_config.heartbeat_interval_seconds;
        let local_node_id = self.local_node_id.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_seconds));
            
            loop {
                interval.tick().await;
                
                let now = chrono::Utc::now().timestamp();
                let mut nodes_guard = nodes.write().await;
                
                // 更新本地节点心跳
                if let Some(node) = nodes_guard.get_mut(&local_node_id) {
                    node.last_heartbeat = now;
                }

                // 检查远程节点状态
                let mut offline_nodes = Vec::new();
                for (id, node) in nodes_guard.iter() {
                    if *id != local_node_id {
                        let elapsed = now - node.last_heartbeat;
                        if elapsed > (interval_seconds * 3) as i64 {
                            offline_nodes.push(id.clone());
                        }
                    }
                }

                // 标记离线节点
                for id in &offline_nodes {
                    if let Some(node) = nodes_guard.get_mut(id) {
                        node.status = NodeStatus::Offline;
                        println!("⚠️ [GPU-MANAGER] Node {} marked offline", id);
                    }
                }
            }
        });
    }

    /// 获取节点列表
    pub async fn list_nodes(&self) -> Vec<GpuNode> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// 获取任务统计
    pub async fn get_task_stats(&self) -> TaskStats {
        let queue = self.task_queue.read().await;
        let active = self.active_tasks.read().await;
        let history = self.task_history.read().await;

        TaskStats {
            queued: queue.len(),
            active: active.len(),
            completed: history.iter().filter(|t| t.status == TaskStatus::Completed).count(),
            failed: history.iter().filter(|t| t.status == TaskStatus::Failed).count(),
        }
    }
}

/// 任务统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub queued: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_manager_creation() {
        let manager = GpuManager::new("test_node".to_string()).await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Normal);
        assert!(TaskPriority::Normal < TaskPriority::Low);
    }

    #[test]
    fn test_can_handle_task() {
        let node = GpuNode {
            node_id: "test".to_string(),
            peer_id: "test".to_string(),
            device_info: DeviceCapabilities::default(),
            gpu_info: vec![GpuDevice {
                index: 0,
                name: "Test GPU".to_string(),
                total_memory_mb: 8192,
                available_memory_mb: 4096,
                compute_capability: "8.0".to_string(),
                driver_version: "1.0".to_string(),
                cuda_version: Some("11.8".to_string()),
                metal_support: false,
                vulkan_support: true,
            }],
            status: NodeStatus::Online,
            last_heartbeat: 0,
            current_tasks: Vec::new(),
            performance_metrics: PerformanceMetrics::default(),
            location: NodeLocation::default(),
            pricing: ComputePricing::default(),
        };

        let requirements = ResourceRequirements {
            min_gpu_memory_mb: 2048,
            min_cpu_cores: 2,
            min_memory_mb: 4096,
            max_duration_minutes: 30,
            preferred_gpu_type: None,
            allow_cpu_fallback: true,
        };

        assert!(GpuManager::can_handle_task(&node, &requirements));
    }
}

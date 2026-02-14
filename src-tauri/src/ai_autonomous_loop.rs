//! AI 自主闭环控制器
//!
//! AI 全自主控制：切分模型、传递模型、运行模型、管理算力
//! 持续 Loop 循环交流和执行

use crate::ai_decision::{
    AIDecisionEngine, DecisionType, TaskInfo, SystemInfo, NetworkInfo,
    create_task_execution_context,
};
use crate::state::AppState;
use crate::api_client::WorkersApiClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;
use chrono::Utc;

/// AI 自主操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutonomousAction {
    /// 切分模型
    SplitModel,
    /// 传递模型
    TransferModel,
    /// 运行模型
    RunModel,
    /// 管理算力
    ManageCompute,
    /// 节点连接
    ConnectNode,
    /// 分配任务
    AllocateTask,
}

/// AI 自主任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousTask {
    pub id: String,
    pub action: AutonomousAction,
    pub parameters: serde_json::Value,
    pub target_devices: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// AI 自主循环状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousLoopState {
    pub is_running: bool,
    pub current_action: Option<AutonomousAction>,
    pub active_tasks: Vec<AutonomousTask>,
    pub completed_tasks: Vec<AutonomousTask>,
    pub total_iterations: u32,
    pub last_error: Option<String>,
}

/// AI 自主闭环控制器
pub struct AIAutonomousController {
    state: Arc<Mutex<AutonomousLoopState>>,
    decision_engine: Arc<AIDecisionEngine>,
}

impl AIAutonomousController {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AutonomousLoopState {
                is_running: false,
                current_action: None,
                active_tasks: Vec::new(),
                completed_tasks: Vec::new(),
                total_iterations: 0,
                last_error: None,
            })),
            decision_engine: Arc::new(AIDecisionEngine::new()),
        }
    }

    /// 启动 AI 自主循环
    pub async fn start_autonomous_loop(
        &self,
        state: &AppState,
        api_client: &WorkersApiClient,
    ) -> Result<(), String> {
        {
            let mut s = self.state.lock();
            if s.is_running {
                return Err("Autonomous loop already running".to_string());
            }
            s.is_running = true;
            s.total_iterations = 0;
        }

        println!("🤖 [AI-AUTONOMOUS] Starting AI autonomous loop...");

        // Loop 主循环
        loop {
            // 检查是否停止
            {
                let s = self.state.lock();
                if !s.is_running {
                    println!("🛑 [AI-AUTONOMOUS] Loop stopped");
                    break;
                }
            }

            // 增加迭代计数
            {
                let mut s = self.state.lock();
                s.total_iterations += 1;
                let iter = s.total_iterations;
                println!("🔄 [AI-AUTONOMOUS] Iteration {}...", iter);
            }

            // 1. AI 收集上下文
            let context = self.collect_context(state).await;

            // 2. AI 决策下一步操作
            let decision = match self.decision_engine.make_decision(context, state).await {
                Ok(d) => d,
                Err(e) => {
                    println!("⚠️ [AI-AUTONOMOUS] Decision failed: {}", e);
                    self.set_error(&e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            println!("🤔 [AI-AUTONOMOUS] AI Decision: {} (confidence: {:.2})", 
                decision.action, decision.confidence);

            // 3. 执行操作
            let action = self.parse_action(&decision.action);
            self.set_current_action(action.clone());

            let result = self.execute_action(
                action,
                &decision.parameters,
                state,
                api_client,
            ).await;

            match result {
                Ok(_) => {
                    println!("✅ [AI-AUTONOMOUS] Action completed");
                    self.mark_task_completed();
                }
                Err(e) => {
                    println!("❌ [AI-AUTONOMOUS] Action failed: {}", e);
                    self.set_error(&e);
                }
            }

            // 4. 等待下一次迭代
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        Ok(())
    }

    /// 停止 AI 自主循环
    pub fn stop(&self) {
        let mut s = self.state.lock();
        s.is_running = false;
        println!("🛑 [AI-AUTONOMOUS] Stop signal sent");
    }

    /// 收集上下文信息
    async fn collect_context(&self, state: &AppState) -> DecisionContext {
        // 收集系统信息
        let device_info = state.device_info.lock().clone();
        let system_info = if let Some(info) = device_info {
            SystemInfo {
                has_gpu: info.gpu_type.is_some(),
                gpu_memory_gb: info.gpu_memory_total.unwrap_or(0.0),
                cpu_cores: info.cpu_cores,
                memory_gb: info.total_memory_gb,
                battery_level: info.battery_level.map(|v| v as f32),
                is_charging: info.is_charging.unwrap_or(false),
            }
        } else {
            SystemInfo {
                has_gpu: false,
                gpu_memory_gb: 0.0,
                cpu_cores: 4,
                memory_gb: 8.0,
                battery_level: None,
                is_charging: true,
            }
        };

        // 收集网络信息
        let network_info = {
            let node_guard = state.node.lock();
            if let Some(node) = node_guard.as_ref() {
                let (primary, backups) = node.topology.neighbor_sets();
                NetworkInfo {
                    connected_peers: primary.len() as u32,
                    total_nodes_available: (primary.len() + backups.len()) as u32,
                    avg_latency_ms: 50,
                    network_type: "broadband".to_string(),
                }
            } else {
                NetworkInfo {
                    connected_peers: 0,
                    total_nodes_available: 0,
                    avg_latency_ms: 0,
                    network_type: "unknown".to_string(),
                }
            }
        };

        // 创建任务信息
        let task_info = TaskInfo {
            task_id: uuid::Uuid::new_v4().to_string(),
            task_type: "autonomous".to_string(),
            model_id: "auto".to_string(),
            input_size: 0,
            priority: "high".to_string(),
        };

        DecisionContext {
            decision_type: DecisionType::TaskExecution,
            system_info,
            network_info,
            task_info: Some(task_info),
            history: Vec::new(),
        }
    }

    /// 解析 AI 决策为具体操作
    fn parse_action(&self, action_str: &str) -> AutonomousAction {
        let lower = action_str.to_lowercase();
        if lower.contains("split") || lower.contains("切分") {
            AutonomousAction::SplitModel
        } else if lower.contains("transfer") || lower.contains("传递") || lower.contains("分发") {
            AutonomousAction::TransferModel
        } else if lower.contains("run") || lower.contains("执行") || lower.contains("运行") {
            AutonomousAction::RunModel
        } else if lower.contains("compute") || lower.contains("算力") || lower.contains("资源") {
            AutonomousAction::ManageCompute
        } else if lower.contains("connect") || lower.contains("连接") {
            AutonomousAction::ConnectNode
        } else if lower.contains("allocat") || lower.contains("分配") {
            AutonomousAction::AllocateTask
        } else {
            AutonomousAction::ManageCompute // 默认
        }
    }

    /// 执行 AI 决策的操作
    async fn execute_action(
        &self,
        action: AutonomousAction,
        parameters: &serde_json::Value,
        state: &AppState,
        api_client: &WorkersApiClient,
    ) -> Result<(), String> {
        match action {
            AutonomousAction::SplitModel => {
                self.execute_model_split(parameters).await
            }
            AutonomousAction::TransferModel => {
                self.execute_model_transfer(parameters, state).await
            }
            AutonomousAction::RunModel => {
                self.execute_model_run(parameters, state).await
            }
            AutonomousAction::ManageCompute => {
                self.execute_compute_management(parameters, state, api_client).await
            }
            AutonomousAction::ConnectNode => {
                self.execute_node_connection(parameters, state).await
            }
            AutonomousAction::AllocateTask => {
                self.execute_task_allocation(parameters, state, api_client).await
            }
        }
    }

    /// 执行模型切分
    async fn execute_model_split(&self, params: &serde_json::Value) -> Result<(), String> {
        let model_id = params.get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        let num_splits = params.get("num_splits")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        println!("✂️ [AI-AUTONOMOUS] Splitting model {} into {} parts", model_id, num_splits);

        // TODO: 实际调用模型切分服务
        // 返回模拟成功
        Ok(())
    }

    /// 执行模型传递
    async fn execute_model_transfer(&self, params: &serde_json::Value, state: &AppState) -> Result<(), String> {
        let target_nodes: Vec<String> = params.get("target_nodes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        println!("📤 [AI-AUTONOMOUS] Transferring model to {} nodes", target_nodes.len());

        // 通过 P2P 网络传输
        for node in &target_nodes {
            println!("  → Transferring to: {}", node);
            // TODO: 实际通过 iroh 传输
        }

        Ok(())
    }

    /// 执行模型运行
    async fn execute_model_run(&self, params: &serde_json::Value, state: &AppState) -> Result<(), String> {
        let model_id = params.get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        let input_data = params.get("input_data").cloned();

        println!("🚀 [AI-AUTONOMOUS] Running model: {}", model_id);

        // 检查本地算力
        let device_info = state.device_info.lock().clone();
        if let Some(info) = device_info {
            if info.gpu_type.is_some() {
                println!("  → Using local GPU");
            } else {
                println!("  → Using local CPU");
            }
        }

        // TODO: 实际运行模型推理
        Ok(())
    }

    /// 执行算力管理
    async fn execute_compute_management(
        &self,
        params: &serde_json::Value,
        state: &AppState,
        api_client: &WorkersApiClient,
    ) -> Result<(), String> {
        println!("💻 [AI-AUTONOMOUS] Managing compute resources...");

        // 收集所有可用设备
        let mut available_devices = Vec::new();
        
        // 本地设备
        let device_info = state.device_info.lock().clone();
        if let Some(info) = device_info {
            available_devices.push(serde_json::json!({
                "type": "local",
                "gpu": info.gpu_type,
                "cpu_cores": info.cpu_cores,
                "memory_gb": info.total_memory_gb,
            }));
        }

        // 网络节点
        let node_guard = state.node.lock();
        if let Some(node) = node_guard.as_ref() {
            let (primary, backups) = node.topology.neighbor_sets();
            for peer_id in primary {
                available_devices.push(serde_json::json!({
                    "type": "peer",
                    "peer_id": peer_id,
                }));
            }
        }

        println!("  → Found {} available devices", available_devices.len());

        // 上报可用算力到 Workers
        let _ = api_client.upload_full_node_info(
            state.device_info.lock().clone().unwrap_or_else(|| crate::state::DeviceInfo {
                gpu_type: None,
                gpu_usage: None,
                gpu_memory_total: None,
                gpu_memory_used: None,
                cpu_cores: 4,
                cpu_usage: 0.0,
                total_memory_gb: 8.0,
                battery_level: None,
                is_charging: None,
            }),
            None,
        ).await;

        Ok(())
    }

    /// 执行节点连接
    async fn execute_node_connection(&self, params: &serde_json::Value, state: &AppState) -> Result<(), String> {
        let target_node = params.get("target_node")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        println!("🔗 [AI-AUTONOMOUS] Connecting to node: {}", target_node);

        if target_node.is_empty() {
            return Err("No target node specified".to_string());
        }

        // 尝试连接
        let mut node_opt = { state.node.lock().take() };
        if let Some(mut node) = node_opt.take() {
            let result = node.comms.connect(target_node.to_string()).await;
            state.node.lock().replace(node);
            result.map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// 执行任务分配
    async fn execute_task_allocation(
        &self,
        params: &serde_json::Value,
        state: &AppState,
        api_client: &WorkersApiClient,
    ) -> Result<(), String> {
        let task_type = params.get("task_type")
            .and_then(|v| v.as_str())
            .unwrap_or("inference");
        
        let model_id = params.get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        println!("📋 [AI-AUTONOMOUS] Allocating task: {} with model: {}", task_type, model_id);

        // 请求 Workers 分配算力
        let response = api_client.request_inference(
            model_id.to_string(),
            serde_json::json!({ "task_type": task_type }),
        ).await?;

        if response.success {
            println!("  → Task allocated to {} nodes", response.selected_nodes.len());
        }

        Ok(())
    }

    /// 设置当前动作
    fn set_current_action(&self, action: AutonomousAction) {
        let mut s = self.state.lock();
        s.current_action = Some(action);
    }

    /// 设置错误
    fn set_error(&self, error: &str) {
        let mut s = self.state.lock();
        s.last_error = Some(error.to_string());
    }

    /// 标记任务完成
    fn mark_task_completed(&self) {
        let mut s = self.state.lock();
        if let Some(action) = s.current_action.take() {
            let task = AutonomousTask {
                id: uuid::Uuid::new_v4().to_string(),
                action,
                parameters: serde_json::Value::Object(Default::default()),
                target_devices: Vec::new(),
                status: "completed".to_string(),
                created_at: Utc::now().to_rfc3339(),
                completed_at: Some(Utc::now().to_rfc3339()),
            };
            s.completed_tasks.push(task);
        }
    }

    /// 获取状态
    pub fn get_state(&self) -> AutonomousLoopState {
        self.state.lock().clone()
    }
}

impl Default for AIAutonomousController {
    fn default() -> Self {
        Self::new()
    }
}

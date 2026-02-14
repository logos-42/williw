//! AI 决策引擎模块（文档驱动版）
//!
//! AI 根据读取的文档自主决策，实现真正的自主闭环
//! 不需要硬编码决策类型，AI 理解文档后自己决定下一步

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;
use chrono::{DateTime, Utc};

/// AI 决策（无硬编码类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIDecision {
    /// 决策类型（AI 自主决定）
    pub decision_type: String,
    /// 具体行动描述
    pub action: String,
    /// 决策理由
    pub reasoning: String,
    /// 置信度
    pub confidence: f32,
    /// 相关参数
    pub parameters: serde_json::Value,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 执行上下文（传递给 AI）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// 当前迭代次数
    pub iteration: u32,
    /// 已完成步骤
    pub completed_steps: Vec<String>,
    /// 当前步骤
    pub current_step: Option<String>,
    /// 执行历史
    pub execution_history: Vec<StepResult>,
    /// 学到的知识
    pub learned_knowledge: serde_json::Value,
    /// 验收标准
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
}

/// 步骤结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// 验收标准
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub evidence: Option<String>,
}

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub has_gpu: bool,
    pub gpu_memory_gb: f64,
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub battery_level: Option<f32>,
    pub is_charging: bool,
}

/// 网络信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub connected_peers: u32,
    pub total_nodes_available: u32,
    pub avg_latency_ms: u32,
    pub network_type: String,
}

/// 任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String,
    pub task_type: String,
    pub model_id: String,
    pub input_size: u32,
    pub priority: String,
}

/// AI 决策结果（用于学习）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub context: ExecutionContext,
    pub decision: AIDecision,
    pub outcome: Option<DecisionOutcome>,
    pub created_at: DateTime<Utc>,
}

/// 决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub success: bool,
    pub execution_time_ms: u64,
    pub error: Option<String>,
    pub metrics: serde_json::Value,
}

/// 文档内容（身份、任务、工具）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentContent {
    /// 身份文档
    pub identity: Option<String>,
    /// 任务文档
    pub task: Option<String>,
    /// 工具文档
    pub tools: Option<String>,
}

/// AI 决策引擎（文档驱动）
pub struct AIDecisionEngine {
    decisions: Arc<Mutex<Vec<DecisionRecord>>>,
    /// 当前文档内容
    documents: Arc<Mutex<DocumentContent>>,
}

impl AIDecisionEngine {
    pub fn new() -> Self {
        Self {
            decisions: Arc::new(Mutex::new(Vec::new())),
            documents: Arc::new(Mutex::new(DocumentContent {
                identity: None,
                task: None,
                tools: None,
            })),
        }
    }

    /// 设置文档内容（AI 读取文档后调用）
    pub fn set_documents(&self, docs: DocumentContent) {
        let mut d = self.documents.lock();
        *d = docs;
    }

    /// 获取当前文档
    pub fn get_documents(&self) -> DocumentContent {
        self.documents.lock().clone()
    }

    /// AI 自主决策（文档驱动）
    /// 不需要传入决策类型，AI 自己根据文档和上下文决定
    pub async fn make_autonomous_decision(
        &self,
        context: ExecutionContext,
        api_key: &str,
        base_url: &str,
        model: &str,
    ) -> Result<AIDecision, String> {
        // 构建提示词（让 AI 根据文档自主决策）
        let prompt = self.build_autonomous_prompt(&context);

        // 调用 AI
        let response = Self::call_ai_api(api_key, base_url, model, &prompt).await?;

        // 解析决策
        let decision = self.parse_decision_response(&response, &context)?;

        // 保存决策记录
        let record = DecisionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            context,
            decision: decision.clone(),
            outcome: None,
            created_at: Utc::now(),
        };

        self.decisions.lock().push(record);

        Ok(decision)
    }

    /// 构建自主决策提示词（让 AI 自己决定做什么）
    fn build_autonomous_prompt(&self, context: &ExecutionContext) -> String {
        let docs = self.documents.lock();

        let system_prompt = r#"你是一个去中心化算力系统的 AI 自主决策专家。

## 你的角色
{doc_identity}

## 你的任务
{doc_task}

## 可用工具
{doc_tools}

## 当前执行状态
- 迭代次数: {iteration}
- 已完成步骤: {completed_steps}
- 当前步骤: {current_step}

## 验收标准
{acceptance_criteria}

## 执行历史
{execution_history}

## 决策要求
1. 仔细阅读以上文档，理解你的角色和任务
2. 分析当前执行状态和验收标准
3. 自主决定下一步要做什么
4. 如果所有验收标准都达成，返回 "COMPLETED"
5. 如果需要继续，返回具体的行动指令

请以 JSON 格式返回决策：
{{
    "decision_type": "AI自主决定的类型（如 split_model, transfer_model, run_model, verify, connect_node 等）",
    "action": "具体行动描述",
    "reasoning": "为什么做这个决定",
    "confidence": 0.0-1.0,
    "parameters": {{具体参数}}
}}
"#;

        // 格式化上下文
        let doc_identity = docs.identity.as_deref().unwrap_or("去中心化算力专家");
        let doc_task = docs.task.as_deref().unwrap_or("管理算力网络");
        let doc_tools = docs.tools.as_deref().unwrap_or("模型切分、传输、运行工具");

        let completed_steps = context.completed_steps.join(", ");
        let current_step = context.current_step.as_deref().unwrap_or("无");

        let acceptance: Vec<String> = context.acceptance_criteria.iter()
            .map(|c| format!("- [{}] {}", if c.completed { "x" } else { " " }, c.description))
            .collect();
        let acceptance_criteria = acceptance.join("\n");

        let history: Vec<String> = context.execution_history.iter()
            .map(|h| format!("- {}: {} ({})", h.step_id, h.success, h.output))
            .collect();
        let execution_history = if history.is_empty() {
            "无".to_string()
        } else {
            history.join("\n")
        };

        system_prompt
            .replace("{doc_identity}", doc_identity)
            .replace("{doc_task}", doc_task)
            .replace("{doc_tools}", doc_tools)
            .replace("{iteration}", &context.iteration.to_string())
            .replace("{completed_steps}", &completed_steps)
            .replace("{current_step}", current_step)
            .replace("{acceptance_criteria}", &acceptance_criteria)
            .replace("{execution_history}", &execution_history)
    }

    /// 调用 AI API
    async fn call_ai_api(
        api_key: &str,
        base_url: &str,
        model: &str,
        prompt: &str,
    ) -> Result<String, String> {
        use reqwest::Client;
        use std::time::Duration;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "messages": [
                    {
                        "role": "system",
                        "content": "你是一个去中心化算力系统的 AI 自主决策专家。请根据文档和上下文自主决定下一步行动，以 JSON 格式返回。"
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "max_tokens": 1024,
                "temperature": 0.3,
            }))
            .send()
            .await
            .map_err(|e| format!("API 请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API 返回错误: {}", response.status()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let content = json
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .ok_or("无法解析 AI 响应")?
            .to_string();

        Ok(content)
    }

    /// 解析 AI 决策响应
    fn parse_decision_response(
        &self,
        response: &str,
        context: &ExecutionContext,
    ) -> Result<AIDecision, String> {
        // 尝试解析 JSON
        let json: serde_json::Value = serde_json::from_str(response)
            .or_else(|_| {
                let json_start = response.find('{');
                let json_end = response.rfind('}');
                if let (Some(start), Some(end)) = (json_start, json_end) {
                    serde_json::from_str(&response[start..=end])
                } else {
                    Err(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "No JSON found",
                    )))
                }
            })
            .map_err(|e| format!("解析决策失败: {}", e))?;

        let decision_type = json
            .get("decision_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let action = json
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("continue")
            .to_string();

        let reasoning = json
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        let confidence = json
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let parameters = json
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default()));

        Ok(AIDecision {
            decision_type,
            action,
            reasoning,
            confidence,
            parameters,
            timestamp: Utc::now(),
        })
    }

    /// 更新决策结果（用于学习）
    pub fn update_outcome(&self, decision_id: &str, outcome: DecisionOutcome) {
        let mut decisions = self.decisions.lock();
        if let Some(record) = decisions.iter_mut().find(|r| r.id == decision_id) {
            record.outcome = Some(outcome);
        }
    }

    /// 获取决策历史
    pub fn get_history(&self) -> Vec<DecisionRecord> {
        self.decisions.lock().clone()
    }

    /// 清除历史记录
    pub fn clear_history(&self) {
        self.decisions.lock().clear();
    }
}

impl Default for AIDecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 便捷函数 ============

/// 创建执行上下文
pub fn create_execution_context(
    iteration: u32,
    completed_steps: Vec<String>,
    current_step: Option<String>,
    acceptance_criteria: Vec<AcceptanceCriterion>,
) -> ExecutionContext {
    ExecutionContext {
        iteration,
        completed_steps,
        current_step,
        execution_history: Vec::new(),
        learned_knowledge: serde_json::json!({}),
        acceptance_criteria,
    }
}

/// 创建默认身份文档
pub fn default_identity_doc() -> String {
    r#"# 身份文档
角色: 去中心化算力专家
专业领域: 模型切分、P2P网络、节点管理
行为准则: 
- 切分前先分析模型结构
- 传输时考虑网络延迟
- 运行前验证节点可用性
"#.to_string()
}

/// 创建默认任务文档（切分模型示例）
pub fn default_task_doc() -> String {
    r#"# 任务文档
目标: 切分模型并分发到多个节点
验收标准:
- [ ] 模型分析完成
- [ ] 切分为指定数量分片
- [ ] 所有分片分发完成
- [ ] 验证通过

步骤:
1. 分析模型结构
2. 执行模型切分
3. 分发分片到节点
4. 验证分发结果
"#.to_string()
}

/// 创建默认工具文档
pub fn default_tools_doc() -> String {
    r#"# 工具文档
可用工具:
- DecentralizedModel::Analyze - 分析模型
- DecentralizedModel::Split - 切分模型
- DecentralizedModel::Transfer - 传输分片
- DecentralizedModel::Verify - 验证结果
- P2PNetwork::Connect - 连接节点
- ComputeManager::Allocate - 分配算力
"#.to_string()
}

// ============ 兼容旧版本的方法（用于 node.rs）============

/// 节点性能记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePerformance {
    pub node_id: String,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub gpu_utilization: Option<f64>,
    pub network_latency: f64,
    pub throughput: f64,
    pub success_rate: f64,
    pub last_updated: DateTime<Utc>,
}

/// 拓扑信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyInfo {
    pub node_id: String,
    pub neighbors: Vec<String>,
    pub network_distance: f64,  // 简化为 f64
    pub connection_quality: f64,
}

/// 网络健康报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealthReport {
    pub average_success_rate: f64,
    pub total_nodes: usize,
    pub recommendations: Vec<String>,
}

impl AIDecisionEngine {
    /// 更新节点性能数据（兼容旧版本）
    pub async fn update_node_performance(&self, performance: NodePerformance) {
        log::info!(
            "[AI-DECISION] 更新节点性能: node={}, cpu={:.2}%, mem={:.2}%, success={:.2}%",
            performance.node_id,
            performance.cpu_utilization * 100.0,
            performance.memory_utilization * 100.0,
            performance.success_rate * 100.0
        );
    }

    /// 更新拓扑信息（兼容旧版本）
    pub async fn update_topology_info(&self, topology: TopologyInfo) {
        log::info!(
            "[AI-DECISION] 更新拓扑信息: node={}, neighbors={}, quality={:.2}",
            topology.node_id,
            topology.neighbors.len(),
            topology.connection_quality
        );
    }

    /// 获取网络健康报告（兼容旧版本）
    pub async fn get_network_health_report(&self) -> NetworkHealthReport {
        let decisions = self.decisions.lock();
        let total = decisions.len();
        
        let successful = decisions
            .iter()
            .filter(|r| r.outcome.as_ref().map(|o| o.success).unwrap_or(false))
            .count();
        
        let average_success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            0.85
        };

        let mut recommendations = Vec::new();
        
        if average_success_rate < 0.8 {
            recommendations.push("成功率较低，建议检查网络连接".to_string());
        }
        if total < 5 {
            recommendations.push("决策样本较少，继续收集数据".to_string());
        }

        NetworkHealthReport {
            average_success_rate,
            total_nodes: total,
            recommendations,
        }
    }
}

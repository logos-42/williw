//! 分布式推理消息协议
//!
//! 定义节点间通信的消息格式

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 推理消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InferenceMessage {
    /// 分布式推理请求（广播到所有节点）
    DistributedInferenceRequest {
        /// 任务 ID
        task_id: String,
        /// 模型 ID
        model_id: String,
        /// 输入文本
        input_text: String,
        /// 推理配置
        config: InferenceConfig,
        /// 请求时间戳
        timestamp: i64,
    },
    
    /// 分布式推理响应（各节点返回结果）
    DistributedInferenceResponse {
        /// 任务 ID
        task_id: String,
        /// 节点 ID
        node_id: String,
        /// 分片 ID
        shard_id: String,
        /// 输出文本
        output_text: String,
        /// 置信度
        confidence: f32,
        /// 执行时间（毫秒）
        execution_time_ms: u64,
        /// 是否成功
        success: bool,
        /// 错误信息
        error: Option<String>,
    },
    
    /// 聚合推理结果（协调节点汇总）
    AggregatedInferenceResult {
        /// 任务 ID
        task_id: String,
        /// 最终输出
        final_output: String,
        /// 各节点结果
        partial_results: Vec<PartialResult>,
        /// 聚合方法
        aggregation_method: AggregationMethod,
        /// 总执行时间
        total_time_ms: u64,
    },
    
    /// 执行分片请求
    ExecuteShard {
        /// 分片 ID
        shard_id: String,
        /// 任务 ID
        task_id: String,
        /// 输入数据（序列化的张量）
        input_data: Vec<u8>,
        /// 元数据
        metadata: ShardExecutionMetadata,
    },
    
    /// 执行结果响应
    ExecutionResult {
        /// 分片 ID
        shard_id: String,
        /// 任务 ID
        task_id: String,
        /// 输出数据（序列化的张量）
        output_data: Vec<u8>,
        /// 执行指标
        metrics: ExecutionMetrics,
        /// 是否成功
        success: bool,
        /// 错误信息
        error: Option<String>,
    },
    
    /// 分片注册
    RegisterShard {
        /// 模型 ID
        model_id: String,
        /// 分片信息
        shard_info: ShardInfo,
    },
    
    /// 分片查询
    QueryShard {
        /// 分片 ID
        shard_id: String,
    },
    
    /// 分片位置响应
    ShardLocation {
        /// 分片 ID
        shard_id: String,
        /// 节点 ID
        node_id: String,
        /// 是否可用
        available: bool,
    },
    
    /// 模型分片表同步
    ShardTableSync {
        /// 模型 ID
        model_id: String,
        /// 分片表
        shards: Vec<ShardInfo>,
        /// 版本号
        version: u64,
    },
    
    /// 心跳
    Heartbeat {
        /// 节点 ID
        node_id: String,
        /// 时间戳
        timestamp: i64,
        /// 负载 (0.0-1.0)
        load: f32,
        /// 可用显存 (MB)
        available_memory_mb: u64,
    },
    
    /// 任务状态查询
    TaskStatusQuery {
        /// 任务 ID
        task_id: String,
    },
    
    /// 任务状态响应
    TaskStatusResponse {
        /// 任务 ID
        task_id: String,
        /// 当前状态
        status: String,
        /// 当前进度 (0.0-1.0)
        progress: f32,
        /// 当前执行的节点
        current_node: Option<String>,
        /// 预计剩余时间（秒）
        estimated_remaining_secs: Option<u64>,
    },
}

/// 分片执行元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardExecutionMetadata {
    /// 模型 ID
    pub model_id: String,
    /// 起始层
    pub layer_start: usize,
    /// 结束层
    pub layer_end: usize,
    /// 输入形状
    pub input_shape: Vec<usize>,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 优先级 (0-9, 9 最高)
    pub priority: u8,
}

/// 聚合方法
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregationMethod {
    /// 投票（选择最常见的答案）
    Voting,
    /// 加权平均
    WeightedAverage,
    /// 选取置信度最高的
    BestConfidence,
    /// 拼接所有输出
    Concatenate,
    /// 简单平均
    Average,
}

/// 部分推理结果（来自单个节点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialResult {
    /// 节点 ID
    pub node_id: String,
    /// 分片 ID
    pub shard_id: String,
    /// 输出文本
    pub output_text: String,
    /// 置信度
    pub confidence: f32,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
}

/// 分片信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    /// 分片 ID
    pub shard_id: String,
    /// 模型 ID
    pub model_id: String,
    /// 所在节点 ID
    pub node_id: String,
    /// 层范围 (起始, 结束)
    pub layer_range: (usize, usize),
    /// 分片大小（字节）
    pub size_bytes: u64,
    /// 校验和
    pub checksum: String,
    /// 状态
    pub status: ShardStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

/// 分片状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShardStatus {
    /// 正在传输
    Transferring,
    /// 就绪
    Ready,
    /// 执行中
    Executing,
    /// 错误
    Error(String),
    /// 离线
    Offline,
}

/// 执行指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetrics {
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// GPU 利用率 (%)
    pub gpu_utilization_percent: f32,
    /// 内存峰值 (MB)
    pub memory_peak_mb: u64,
    /// 能耗 (Wh)
    pub energy_consumption_wh: f32,
    /// 数据传输量 (MB)
    pub data_transferred_mb: f64,
    /// 令牌数（如果是文本生成）
    pub tokens_generated: Option<u32>,
}

/// 分片表
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShardTable {
    /// 模型 ID -> 分片列表
    pub models: std::collections::HashMap<String, Vec<ShardInfo>>,
    /// 版本号
    pub version: u64,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

impl ShardTable {
    /// 创建新的分片表
    pub fn new() -> Self {
        Self {
            models: std::collections::HashMap::new(),
            version: 0,
            last_updated: Utc::now(),
        }
    }
    
    /// 注册模型的分片
    pub fn register_shards(&mut self, model_id: &str, shards: Vec<ShardInfo>) {
        self.models.insert(model_id.to_string(), shards);
        self.version += 1;
        self.last_updated = Utc::now();
    }
    
    /// 获取模型的分片列表
    pub fn get_shards(&self, model_id: &str) -> Option<&Vec<ShardInfo>> {
        self.models.get(model_id)
    }
    
    /// 查找分片所在的节点
    pub fn locate_shard(&self, shard_id: &str) -> Option<&ShardInfo> {
        for shards in self.models.values() {
            if let Some(shard) = shards.iter().find(|s| s.shard_id == shard_id) {
                return Some(shard);
            }
        }
        None
    }
    
    /// 更新分片状态
    pub fn update_shard_status(&mut self, shard_id: &str, status: ShardStatus) {
        for shards in self.models.values_mut() {
            if let Some(shard) = shards.iter_mut().find(|s| s.shard_id == shard_id) {
                shard.status = status;
                shard.updated_at = Utc::now();
                self.version += 1;
                self.last_updated = Utc::now();
                break;
            }
        }
    }
}

/// 推理任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// 任务 ID
    pub task_id: String,
    /// 模型 ID
    pub model_id: String,
    /// 输入数据
    pub input_data: Vec<u8>,
    /// 配置
    pub config: InferenceConfig,
}

/// 推理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// 最大生成长度
    pub max_new_tokens: u32,
    /// 温度
    pub temperature: f32,
    /// Top-p 采样
    pub top_p: f32,
    /// 是否流式输出
    pub stream: bool,
    /// 超时时间（秒）
    pub timeout_secs: u64,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_new_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            stream: false,
            timeout_secs: 60,
        }
    }
}

/// 推理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    /// 任务 ID
    pub task_id: String,
    /// 输出数据
    pub output_data: Vec<u8>,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 执行指标
    pub metrics: ExecutionMetrics,
    /// 完成时间
    pub completed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shard_table() {
        let mut table = ShardTable::new();
        
        let shard = ShardInfo {
            shard_id: "shard_0".to_string(),
            model_id: "model_1".to_string(),
            node_id: "node_1".to_string(),
            layer_range: (0, 10),
            size_bytes: 1024 * 1024 * 350,
            checksum: "abc123".to_string(),
            status: ShardStatus::Ready,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        table.register_shards("model_1", vec![shard.clone()]);
        
        assert!(table.get_shards("model_1").is_some());
        assert_eq!(table.locate_shard("shard_0").unwrap().node_id, "node_1");
    }
    
    #[test]
    fn test_message_serialization() {
        let msg = InferenceMessage::ExecuteShard {
            shard_id: "shard_0".to_string(),
            task_id: "task_1".to_string(),
            input_data: vec![1, 2, 3, 4],
            metadata: ShardExecutionMetadata {
                model_id: "model_1".to_string(),
                layer_start: 0,
                layer_end: 10,
                input_shape: vec![1, 512],
                timeout_ms: 30000,
                priority: 5,
            },
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: InferenceMessage = serde_json::from_str(&json).unwrap();
        
        match decoded {
            InferenceMessage::ExecuteShard { shard_id, .. } => {
                assert_eq!(shard_id, "shard_0");
            }
            _ => panic!("Wrong message type"),
        }
    }
}

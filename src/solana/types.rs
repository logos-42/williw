//! Solana 相关类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// 节点 ID（公钥）
    pub node_id: String,
    /// 节点所有者地址
    pub owner_address: String,
    /// 节点名称
    pub name: String,
    /// 设备类型
    pub device_type: String,
    /// 注册时间戳
    pub registered_at: i64,
    /// 最后活跃时间戳
    pub last_active_at: i64,
    /// 节点状态
    pub status: NodeStatus,
}

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// 活跃
    Active,
    /// 离线
    Offline,
    /// 暂停
    Paused,
    /// 封禁
    Banned,
}

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    /// 训练
    Training,
    /// 推理
    Inference,
    /// 验证
    Validation,
    /// 数据收集
    DataCollection,
}

/// 节点地理位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: i32,  // 纬度 * 1000000
    pub longitude: i32, // 经度 * 1000000
    pub country: String, // 国家代码
    pub region: String, // 地区
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_id: String,
    pub version: String,
    pub parameters_hash: String, // 参数哈希
    pub size_mb: u32,
}

/// 质押信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeInfo {
    pub amount: u64,        // 质押数量（lamports）
    pub staked_at: i64,     // 质押时间
    pub lock_until: i64,    // 锁定到期时间
    pub is_slashed: bool,   // 是否被罚没
}

/// 算力贡献记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeContribution {
    /// 记录 ID
    pub id: String,
    /// 节点 ID
    pub node_id: String,
    /// 任务 ID
    pub task_id: String,
    /// 开始时间戳
    pub start_timestamp: i64,
    /// 结束时间戳
    pub end_timestamp: i64,
    /// 持续时间（秒）
    pub duration_seconds: u64,
    /// 平均 GPU 使用率（0-100）
    pub avg_gpu_usage_percent: f32,
    /// GPU 显存使用量（MB）
    pub gpu_memory_used_mb: u64,
    /// 平均 CPU 使用率（0-100）
    pub avg_cpu_usage_percent: f32,
    /// 内存使用量（MB）
    pub memory_used_mb: u64,
    /// 网络上传量（MB）
    pub network_upload_mb: u64,
    /// 网络下载量（MB）
    pub network_download_mb: u64,
    /// 计算的样本数量
    pub samples_processed: u64,
    /// 计算的批次数量
    pub batches_processed: u64,
    /// 算力评分（基于上述指标计算）
    pub compute_score: f64,
}

/// 算力贡献统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeStats {
    /// 节点 ID
    pub node_id: String,
    /// 总计算时间（秒）
    pub total_compute_seconds: u64,
    /// 总处理样本数
    pub total_samples_processed: u64,
    /// 总计算分数
    pub total_compute_score: f64,
    /// 平均 GPU 使用率
    pub avg_gpu_usage_percent: f32,
    /// 平均 CPU 使用率
    pub avg_cpu_usage_percent: f32,
    /// 总网络流量（MB）
    pub total_network_mb: u64,
    /// 贡献记录数量
    pub contribution_count: u32,
}

/// 收益分配记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistribution {
    /// 记录 ID
    pub id: String,
    /// 节点 ID
    pub node_id: String,
    /// 任务 ID
    pub task_id: String,
    /// 收益金额（lamports）
    pub amount_lamports: u64,
    /// 分配时间戳
    pub distributed_at: i64,
    /// 交易签名
    pub transaction_signature: String,
    /// 状态
    pub status: RewardStatus,
}

/// 收益状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardStatus {
    /// 待确认
    Pending,
    /// 已确认
    Confirmed,
    /// 已完成
    Completed,
    /// 失败
    Failed,
}

/// 节点钱包余额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeWalletBalance {
    /// 节点 ID
    pub node_id: String,
    /// 钱包地址
    pub wallet_address: String,
    /// SOL 余额（lamports）
    pub sol_balance_lamports: u64,
    /// 待结算收益（lamports）
    pub pending_rewards_lamports: u64,
    /// 已分配总收益（lamports）
    pub total_rewards_distributed_lamports: u64,
    /// 最后更新时间戳
    pub last_updated_at: i64,
}

/// 智能合约状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    /// 程序 ID
    pub program_id: String,
    /// 管理员地址
    pub admin_address: String,
    /// 财库地址
    pub treasury_address: String,
    /// 注册节点数量
    pub total_nodes: u32,
    /// 总贡献记录数
    pub total_contributions: u32,
    /// 总分配收益（lamports）
    pub total_rewards_distributed_lamports: u64,
    /// 每次计算的基础奖励（lamports）
    pub base_reward_per_compute_lamports: u64,
    /// 收益池余额（lamports）
    pub reward_pool_balance_lamports: u64,
}

/// 交易结果
#[derive(Debug, Clone)]
pub struct TransactionResult {
    /// 交易签名
    pub signature: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

/// 算力贡献报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeContributionReport {
    /// 节点信息
    pub node_info: NodeInfo,
    /// 算力贡献记录
    pub contribution: ComputeContribution,
    /// 收益预估（lamports）
    pub estimated_reward_lamports: u64,
}

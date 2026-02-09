use anchor_lang::prelude::*;

/// 节点状态枚举
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    Active,
    Offline,
    Paused,
    Banned,
}

/// 收益状态枚举
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum RewardStatus {
    Pending,
    Confirmed,
    Completed,
    Failed,
}

/// 贡献等级枚举
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum ContributionLevel {
    Beginner,
    Regular,
    Medium,
    High,
    Elite,
}

/// 任务类型枚举
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum TaskType {
    Training,
    Inference,
    Validation,
    DataCollection,
}

/// 节点地理位置
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Location {
    pub latitude: i32,  // 纬度 * 1000000
    pub longitude: i32, // 经度 * 1000000
    pub country: String, // 国家代码
    pub region: String, // 地区
}

/// 模型信息
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ModelInfo {
    pub model_id: String,
    pub version: String,
    pub parameters_hash: String, // 参数哈希
    pub size_mb: u32,
}

/// 质押信息
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct StakeInfo {
    pub amount: u64,        // 质押数量（lamports）
    pub staked_at: i64,     // 质押时间
    pub lock_until: i64,    // 锁定到期时间
    pub is_slashed: bool,   // 是否被罚没
}

/// 交易账户元数据
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl ContributionLevel {
    pub fn reward_multiplier(&self) -> u64 {
        match self {
            ContributionLevel::Beginner => 100,    // 1.0x
            ContributionLevel::Regular => 110,     // 1.1x
            ContributionLevel::Medium => 125,      // 1.25x
            ContributionLevel::High => 150,        // 1.5x
            ContributionLevel::Elite => 200,       // 2.0x
        }
    }
}

/// 计算贡献等级
pub fn calculate_contribution_level(total_compute_score: f64, contribution_count: u32) -> ContributionLevel {
    let avg_score = if contribution_count > 0 {
        total_compute_score / contribution_count as f64
    } else {
        0.0
    };

    match (avg_score, contribution_count) {
        (s, c) if s >= 5.0 && c >= 100 => ContributionLevel::Elite,
        (s, c) if s >= 3.0 && c >= 50 => ContributionLevel::High,
        (s, c) if s >= 1.5 && c >= 20 => ContributionLevel::Medium,
        (s, c) if s >= 0.5 && c >= 10 => ContributionLevel::Regular,
        _ => ContributionLevel::Beginner,
    }
}

/// 计算奖励金额
pub fn calculate_reward_amount(
    compute_score: f64,
    duration_seconds: u64,
    base_reward: u64,
    total_compute_score: f64,
    total_contributions: u32,
    quality_score: f32,
    task_type: TaskType,
) -> u64 {
    // 基础奖励 + 算力评分加成
    let base_reward_f64 = base_reward as f64;
    let score_multiplier = 1.0 + compute_score;

    // 持续时间奖励（每额外1小时增加5%）
    let hours = duration_seconds as f64 / 3600.0;
    let duration_multiplier = 1.0 + (hours * 0.05);

    // 质量评分加成（0.0-1.0，转换为0.5-1.5倍）
    let quality_multiplier = 0.5 + (quality_score as f64);

    // 任务类型加成
    let task_multiplier = match task_type {
        TaskType::Training => 1.2,      // 训练任务奖励更高
        TaskType::Inference => 0.8,      // 推理任务奖励较低
        TaskType::Validation => 1.0,     // 验证任务标准奖励
        TaskType::DataCollection => 0.6, // 数据收集任务奖励最低
    };

    // 贡献等级倍率
    let level = calculate_contribution_level(total_compute_score, total_contributions);
    let level_multiplier = level.reward_multiplier() as f64 / 100.0;

    let total_reward = base_reward_f64 * score_multiplier * duration_multiplier * quality_multiplier * task_multiplier * level_multiplier;
    total_reward as u64
}

/// 更新信誉分数
pub fn update_reputation_score(reputation_score: &mut u32, compute_score: f64, quality_score: f32) {
    let score_increase = ((compute_score * 10.0) + (quality_score as f64 * 5.0)) as u32;
    
    // 信誉分数范围 0-1000
    *reputation_score = (*reputation_score + score_increase).min(1000);
}

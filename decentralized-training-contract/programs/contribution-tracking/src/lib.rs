use anchor_lang::prelude::*;
use shared_types::*;

declare_id!("CONTRIBUTION_TRACKING_PROGRAM_ID");

/// 算力贡献账户
#[account]
pub struct ContributionAccount {
    pub id: String,                       // 贡献记录ID
    pub node_id: Pubkey,                  // 节点ID
    pub task_id: String,                  // 任务ID
    pub task_type: TaskType,               // 任务类型
    pub model_info: ModelInfo,            // 模型信息
    pub start_timestamp: i64,             // 开始时间戳
    pub end_timestamp: i64,               // 结束时间戳
    pub duration_seconds: u64,            // 持续时间（秒）
    pub avg_gpu_usage_percent: f32,       // 平均GPU使用率
    pub gpu_memory_used_mb: u64,          // GPU显存使用量
    pub avg_cpu_usage_percent: f32,       // 平均CPU使用率
    pub memory_used_mb: u64,              // 内存使用量
    pub network_upload_mb: u64,           // 网络上传量
    pub network_download_mb: u64,         // 网络下载量
    pub samples_processed: u64,           // 处理的样本数量
    pub batches_processed: u64,           // 处理的批次数量
    pub compute_score: f64,               // 算力评分
    pub quality_score: f32,               // 质量评分
    pub reward_amount: u64,               // 奖励金额（lamports）
    pub is_verified: bool,               // 是否已验证
    pub verified_by: Option<Pubkey>,      // 验证者
    pub verification_timestamp: Option<i64>, // 验证时间
    pub bump: u8,                         // PDA bump
}

/// 贡献跟踪全局状态
#[account]
pub struct ContributionTrackingState {
    pub admin: Pubkey,                    // 管理员公钥
    pub total_contributions: u32,         // 总贡献记录数
    pub total_compute_score: f64,        // 总算力评分
    pub base_reward_per_compute: u64,     // 每次计算的基础奖励（lamports）
    pub verification_required: bool,      // 是否需要验证
    pub min_quality_threshold: f32,       // 最低质量阈值
    pub bump: u8,                         // PDA bump
}

#[program]
pub mod contribution_tracking {
    use super::*;

    /// 初始化贡献跟踪合约
    pub fn initialize(
        ctx: Context<Initialize>,
        base_reward_per_compute: u64,
        verification_required: bool,
        min_quality_threshold: f32,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.total_contributions = 0;
        state.total_compute_score = 0.0;
        state.base_reward_per_compute = base_reward_per_compute;
        state.verification_required = verification_required;
        state.min_quality_threshold = min_quality_threshold;
        state.bump = ctx.bumps.state;

        msg!("Contribution tracking contract initialized");
        Ok(())
    }

    /// 记录算力贡献
    pub fn record_contribution(
        ctx: Context<RecordContribution>,
        contribution_id: String,
        node_id: Pubkey,
        task_id: String,
        task_type: TaskType,
        model_info: ModelInfo,
        start_timestamp: i64,
        end_timestamp: i64,
        duration_seconds: u64,
        avg_gpu_usage_percent: f32,
        gpu_memory_used_mb: u64,
        avg_cpu_usage_percent: f32,
        memory_used_mb: u64,
        network_upload_mb: u64,
        network_download_mb: u64,
        samples_processed: u64,
        batches_processed: u64,
        compute_score: f64,
        quality_score: f32,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 验证时间戳
        require!(start_timestamp < end_timestamp, ErrorCode::InvalidContributionData);
        require!(end_timestamp <= current_time, ErrorCode::InvalidContributionData);

        // 验证质量分数
        let state = &ctx.accounts.state;
        if state.verification_required {
            require!(quality_score >= state.min_quality_threshold, ErrorCode::QualityTooLow);
        }

        // 计算奖励金额
        let reward_amount = calculate_reward_amount(
            compute_score,
            duration_seconds,
            state.base_reward_per_compute,
            0.0, // 新贡献，历史评分为0
            0,   // 新贡献，历史贡献次数为0
            quality_score,
            task_type.clone(),
        );

        let contribution_account = &mut ctx.accounts.contribution_account;
        let state = &mut ctx.accounts.state;

        // 初始化贡献账户
        contribution_account.id = contribution_id.clone();
        contribution_account.node_id = node_id;
        contribution_account.task_id = task_id;
        contribution_account.task_type = task_type;
        contribution_account.model_info = model_info;
        contribution_account.start_timestamp = start_timestamp;
        contribution_account.end_timestamp = end_timestamp;
        contribution_account.duration_seconds = duration_seconds;
        contribution_account.avg_gpu_usage_percent = avg_gpu_usage_percent;
        contribution_account.gpu_memory_used_mb = gpu_memory_used_mb;
        contribution_account.avg_cpu_usage_percent = avg_cpu_usage_percent;
        contribution_account.memory_used_mb = memory_used_mb;
        contribution_account.network_upload_mb = network_upload_mb;
        contribution_account.network_download_mb = network_download_mb;
        contribution_account.samples_processed = samples_processed;
        contribution_account.batches_processed = batches_processed;
        contribution_account.compute_score = compute_score;
        contribution_account.quality_score = quality_score;
        contribution_account.reward_amount = reward_amount;
        contribution_account.is_verified = !state.verification_required;
        contribution_account.verified_by = None;
        contribution_account.verification_timestamp = None;
        contribution_account.bump = ctx.bumps.contribution_account;

        // 更新全局统计
        state.total_contributions += 1;
        state.total_compute_score += compute_score;

        msg!("Contribution recorded: {} for node {}", contribution_id, node_id);
        Ok(())
    }

    /// 验证贡献
    pub fn verify_contribution(
        ctx: Context<VerifyContribution>,
        contribution_id: String,
        is_valid: bool,
        verifier_notes: Option<String>,
    ) -> Result<()> {
        let contribution_account = &mut ctx.accounts.contribution_account;
        let state = &ctx.accounts.state;

        // 只有管理员可以验证贡献
        require!(ctx.accounts.verifier.key() == state.admin, ErrorCode::Unauthorized);
        require!(!contribution_account.is_verified, ErrorCode::AlreadyVerified);

        let current_time = Clock::get()?.unix_timestamp;

        contribution_account.is_verified = is_valid;
        contribution_account.verified_by = Some(ctx.accounts.verifier.key());
        contribution_account.verification_timestamp = Some(current_time);

        // 如果验证失败，将奖励金额设为0
        if !is_valid {
            contribution_account.reward_amount = 0;
        }

        msg!("Contribution verified: {} -> {}", contribution_id, is_valid);
        Ok(())
    }

    /// 批量验证贡献
    pub fn batch_verify_contributions(
        ctx: Context<BatchVerifyContributions>,
        contribution_ids: Vec<String>,
        verification_results: Vec<bool>,
    ) -> Result<()> {
        let state = &ctx.accounts.state;

        // 只有管理员可以批量验证
        require!(ctx.accounts.verifier.key() == state.admin, ErrorCode::Unauthorized);
        require!(contribution_ids.len() == verification_results.len(), ErrorCode::MismatchedArrays);

        let current_time = Clock::get()?.unix_timestamp;

        for (i, contribution_id) in contribution_ids.iter().enumerate() {
            let contribution_account = &mut ctx.accounts.contribution_accounts[i];
            
            if !contribution_account.is_verified {
                contribution_account.is_verified = verification_results[i];
                contribution_account.verified_by = Some(ctx.accounts.verifier.key());
                contribution_account.verification_timestamp = Some(current_time);

                // 如果验证失败，将奖励金额设为0
                if !verification_results[i] {
                    contribution_account.reward_amount = 0;
                }
            }
        }

        msg!("Batch verified {} contributions", contribution_ids.len());
        Ok(())
    }

    /// 更新基础奖励
    pub fn update_base_reward(
        ctx: Context<UpdateBaseReward>,
        new_base_reward: u64,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;

        // 只有管理员可以更新基础奖励
        require!(ctx.accounts.authority.key() == state.admin, ErrorCode::Unauthorized);

        state.base_reward_per_compute = new_base_reward;

        msg!("Base reward updated to: {} lamports", new_base_reward);
        Ok(())
    }

    /// 更新验证要求
    pub fn update_verification_settings(
        ctx: Context<UpdateVerificationSettings>,
        verification_required: bool,
        min_quality_threshold: f32,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;

        // 只有管理员可以更新验证设置
        require!(ctx.accounts.authority.key() == state.admin, ErrorCode::Unauthorized);
        require!(min_quality_threshold >= 0.0 && min_quality_threshold <= 1.0, ErrorCode::InvalidQualityThreshold);

        state.verification_required = verification_required;
        state.min_quality_threshold = min_quality_threshold;

        msg!("Verification settings updated: required={}, threshold={}", verification_required, min_quality_threshold);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 4 + 8 + 8 + 1 + 4 + 1, // 空间计算
        seeds = [b"contribution-tracking-state"],
        bump
    )]
    pub state: Account<'info, ContributionTrackingState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(contribution_id: String)]
pub struct RecordContribution<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + (4 + 36) + 32 + (4 + 36) + 8 + 8 + 8 + 4 + 8 + 4 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1, // 空间计算
        seeds = [b"contribution", contribution_id.as_bytes()],
        bump
    )]
    pub contribution_account: Account<'info, ContributionAccount>,

    #[account(mut)]
    pub state: Account<'info, ContributionTrackingState>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyContribution<'info> {
    #[account(mut)]
    pub contribution_account: Account<'info, ContributionAccount>,

    pub state: Account<'info, ContributionTrackingState>,

    pub verifier: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(contribution_ids: Vec<String>)]
pub struct BatchVerifyContributions<'info> {
    #[account(mut)]
    pub state: Account<'info, ContributionTrackingState>,

    // 最多支持10个贡献的批量验证
    #[account(mut, constraint = contribution_accounts.len() <= 10)]
    pub contribution_accounts: Vec<Account<'info, ContributionAccount>>,

    pub verifier: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateBaseReward<'info> {
    #[account(mut)]
    pub state: Account<'info, ContributionTrackingState>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateVerificationSettings<'info> {
    #[account(mut)]
    pub state: Account<'info, ContributionTrackingState>,

    pub authority: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid contribution data")]
    InvalidContributionData,
    #[msg("Quality score too low")]
    QualityTooLow,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Contribution already verified")]
    AlreadyVerified,
    #[msg("Mismatched array sizes")]
    MismatchedArrays,
    #[msg("Invalid quality threshold")]
    InvalidQualityThreshold,
}

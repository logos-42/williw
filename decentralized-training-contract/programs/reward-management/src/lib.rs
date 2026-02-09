use anchor_lang::prelude::*;
use shared_types::*;

declare_id!("REWARD_MANAGEMENT_PROGRAM_ID");

/// 收益分配账户
#[account]
pub struct RewardAccount {
    pub id: String,                       // 分配记录ID
    pub node_id: Pubkey,                  // 节点ID
    pub contribution_id: String,          // 贡献记录ID
    pub amount_lamports: u64,             // 收益金额
    pub distributed_at: i64,              // 分配时间戳
    pub status: RewardStatus,             // 状态
    pub bump: u8,                         // PDA bump
}

/// 节点收益汇总账户
#[account]
pub struct NodeRewardSummary {
    pub node_id: Pubkey,                  // 节点ID
    pub total_earned: u64,               // 总收益
    pub total_distributed: u64,          // 已分配收益
    pub pending_rewards: u64,             // 待分配收益
    pub last_distribution_at: i64,       // 最后分配时间
    pub distribution_count: u32,         // 分配次数
    pub bump: u8,                         // PDA bump
}

/// 收益管理全局状态
#[account]
pub struct RewardManagementState {
    pub admin: Pubkey,                    // 管理员公钥
    pub treasury: Pubkey,                 // 国库地址
    pub total_rewards_distributed: u64,   // 总分配收益（lamports）
    pub reward_pool_balance: u64,         // 奖励池余额（lamports）
    pub min_distribution_amount: u64,     // 最小分配金额
    pub distribution_frequency: u64,       // 分配频率（秒）
    pub auto_distribution_enabled: bool,  // 是否启用自动分配
    pub bump: u8,                         // PDA bump
}

#[program]
pub mod reward_management {
    use super::*;

    /// 初始化收益管理合约
    pub fn initialize(
        ctx: Context<Initialize>,
        treasury: Pubkey,
        min_distribution_amount: u64,
        distribution_frequency: u64,
        auto_distribution_enabled: bool,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.treasury = treasury;
        state.total_rewards_distributed = 0;
        state.reward_pool_balance = 0;
        state.min_distribution_amount = min_distribution_amount;
        state.distribution_frequency = distribution_frequency;
        state.auto_distribution_enabled = auto_distribution_enabled;
        state.bump = ctx.bumps.state;

        msg!("Reward management contract initialized");
        Ok(())
    }

    /// 分配收益到节点
    pub fn distribute_rewards(
        ctx: Context<DistributeRewards>,
        node_id: Pubkey,
        contribution_id: String,
        amount_lamports: u64,
    ) -> Result<()> {
        let reward_account = &mut ctx.accounts.reward_account;
        let node_summary = &mut ctx.accounts.node_reward_summary;
        let state = &mut ctx.accounts.state;

        // 验证金额
        require!(amount_lamports >= state.min_distribution_amount, ErrorCode::AmountTooLow);
        require!(state.reward_pool_balance >= amount_lamports, ErrorCode::InsufficientPoolBalance);

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 转移收益
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? -= amount_lamports;
        **ctx.accounts.node_wallet.to_account_info().try_borrow_mut_lamports()? += amount_lamports;

        // 创建收益分配记录
        reward_account.id = format!("reward_{}_{}", node_id, current_time);
        reward_account.node_id = node_id;
        reward_account.contribution_id = contribution_id;
        reward_account.amount_lamports = amount_lamports;
        reward_account.distributed_at = current_time;
        reward_account.status = RewardStatus::Completed;
        reward_account.bump = ctx.bumps.reward_account;

        // 更新节点收益汇总
        node_summary.total_earned += amount_lamports;
        node_summary.total_distributed += amount_lamports;
        node_summary.last_distribution_at = current_time;
        node_summary.distribution_count += 1;

        // 更新全局状态
        state.total_rewards_distributed += amount_lamports;
        state.reward_pool_balance -= amount_lamports;

        msg!("Rewards distributed: {} lamports to node {}", amount_lamports, node_id);
        Ok(())
    }

    /// 批量分配收益
    pub fn batch_distribute_rewards(
        ctx: Context<BatchDistributeRewards>,
        distributions: Vec<RewardDistribution>,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        let total_amount: u64 = distributions.iter().map(|d| d.amount_lamports).sum();

        // 验证总金额
        require!(state.reward_pool_balance >= total_amount, ErrorCode::InsufficientPoolBalance);

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        for (i, distribution) in distributions.iter().enumerate() {
            // 验证单个金额
            require!(distribution.amount_lamports >= state.min_distribution_amount, ErrorCode::AmountTooLow);

            // 转移收益
            **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? -= distribution.amount_lamports;
            **ctx.accounts.node_wallets[i].to_account_info().try_borrow_mut_lamports()? += distribution.amount_lamports;

            // 创建收益分配记录
            let reward_account = &mut ctx.accounts.reward_accounts[i];
            reward_account.id = format!("batch_reward_{}_{}", distribution.node_id, current_time);
            reward_account.node_id = distribution.node_id;
            reward_account.contribution_id = distribution.contribution_id.clone();
            reward_account.amount_lamports = distribution.amount_lamports;
            reward_account.distributed_at = current_time;
            reward_account.status = RewardStatus::Completed;
            reward_account.bump = ctx.bumps.reward_accounts[i];

            // 更新节点收益汇总
            let node_summary = &mut ctx.accounts.node_summaries[i];
            node_summary.total_earned += distribution.amount_lamports;
            node_summary.total_distributed += distribution.amount_lamports;
            node_summary.last_distribution_at = current_time;
            node_summary.distribution_count += 1;
        }

        // 更新全局状态
        state.total_rewards_distributed += total_amount;
        state.reward_pool_balance -= total_amount;

        msg!("Batch distributed rewards: {} lamports to {} nodes", total_amount, distributions.len());
        Ok(())
    }

    /// 质押代币
    pub fn stake_tokens(
        ctx: Context<StakeTokens>,
        node_id: Pubkey,
        amount: u64,
        lock_duration_seconds: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        let lock_until = current_time + lock_duration_seconds as i64;

        // 转移质押代币到国库
        **ctx.accounts.staker.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? += amount;

        // 更新节点质押信息（这里需要通过CPI调用节点管理合约）
        // 简化实现，实际应该调用节点管理合约的更新质押信息函数

        msg!("Staked {} lamports for node {} until {}", amount, node_id, lock_until);
        Ok(())
    }

    /// 解除质押代币
    pub fn unstake_tokens(
        ctx: Context<UnstakeTokens>,
        node_id: Pubkey,
        amount: u64,
    ) -> Result<()> {
        // 验证质押已到期且未被罚没
        // 简化实现，实际应该从节点管理合约查询质押信息

        // 转移代币从国库到用户
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.staker.to_account_info().try_borrow_mut_lamports()? += amount;

        msg!("Unstaked {} lamports for node {}", amount, node_id);
        Ok(())
    }

    /// 增加奖励池余额
    pub fn add_to_reward_pool(
        ctx: Context<AddToRewardPool>,
        amount: u64,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;

        // 转移代币到国库
        **ctx.accounts.funder.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? += amount;

        state.reward_pool_balance += amount;

        msg!("Added {} lamports to reward pool", amount);
        Ok(())
    }

    /// 更新分配设置
    pub fn update_distribution_settings(
        ctx: Context<UpdateDistributionSettings>,
        min_distribution_amount: u64,
        distribution_frequency: u64,
        auto_distribution_enabled: bool,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;

        // 只有管理员可以更新设置
        require!(ctx.accounts.authority.key() == state.admin, ErrorCode::Unauthorized);

        state.min_distribution_amount = min_distribution_amount;
        state.distribution_frequency = distribution_frequency;
        state.auto_distribution_enabled = auto_distribution_enabled;

        msg!("Distribution settings updated");
        Ok(())
    }

    /// 紧急提取（仅管理员）
    pub fn emergency_withdraw(
        ctx: Context<EmergencyWithdraw>,
        amount: u64,
    ) -> Result<()> {
        let state = &ctx.accounts.state;

        // 只有管理员可以紧急提取
        require!(ctx.accounts.authority.key() == state.admin, ErrorCode::Unauthorized);
        require!(state.reward_pool_balance >= amount, ErrorCode::InsufficientPoolBalance);

        // 转移代币
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += amount;

        msg!("Emergency withdraw: {} lamports", amount);
        Ok(())
    }
}

/// 收益分配结构
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RewardDistribution {
    pub node_id: Pubkey,
    pub contribution_id: String,
    pub amount_lamports: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1, // 空间计算
        seeds = [b"reward-management-state"],
        bump
    )]
    pub state: Account<'info, RewardManagementState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + (4 + 36) + 32 + (4 + 36) + 8 + 8 + 1 + 1, // 空间计算
        seeds = [b"reward", node_id.as_ref(), &Clock::get().unwrap().unix_timestamp.to_le_bytes()],
        bump
    )]
    pub reward_account: Account<'info, RewardAccount>,

    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 8 + 8 + 4 + 1, // 空间计算
        seeds = [b"node-reward-summary", node_id.as_ref()],
        bump
    )]
    pub node_reward_summary: Account<'info, NodeRewardSummary>,

    #[account(mut)]
    pub state: Account<'info, RewardManagementState>,

    /// CHECK: 国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    /// CHECK: 节点钱包地址
    #[account(mut)]
    pub node_wallet: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BatchDistributeRewards<'info> {
    #[account(mut)]
    pub state: Account<'info, RewardManagementState>,

    // 最多支持10个节点的批量分配
    #[account(mut, constraint = reward_accounts.len() <= 10)]
    pub reward_accounts: Vec<Account<'info, RewardAccount>>,

    #[account(mut, constraint = node_summaries.len() <= 10)]
    pub node_summaries: Vec<Account<'info, NodeRewardSummary>>,

    /// CHECK: 国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    /// CHECK: 节点钱包地址列表
    #[account(mut, constraint = node_wallets.len() <= 10)]
    pub node_wallets: Vec<AccountInfo<'info>>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct StakeTokens<'info> {
    #[account(mut)]
    pub state: Account<'info, RewardManagementState>,

    /// CHECK: 国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub staker: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnstakeTokens<'info> {
    #[account(mut)]
    pub state: Account<'info, RewardManagementState>,

    /// CHECK: 国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub staker: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddToRewardPool<'info> {
    #[account(mut)]
    pub state: Account<'info, RewardManagementState>,

    /// CHECK: 国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub funder: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateDistributionSettings<'info> {
    #[account(mut)]
    pub state: Account<'info, RewardManagementState>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct EmergencyWithdraw<'info> {
    #[account(mut)]
    pub state: Account<'info, RewardManagementState>,

    /// CHECK: 国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    /// CHECK: 接收者地址
    #[account(mut)]
    pub recipient: AccountInfo<'info>,

    pub authority: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount too low")]
    AmountTooLow,
    #[msg("Insufficient pool balance")]
    InsufficientPoolBalance,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Tokens are still locked")]
    TokensStillLocked,
    #[msg("Tokens have been slashed")]
    TokensSlashed,
}

use anchor_lang::prelude::*;
use shared_types::*;

declare_id!("NODE_MANAGEMENT_PROGRAM_ID");

/// 节点账户
#[account]
pub struct NodeAccount {
    pub node_id: Pubkey,                  // 节点ID（公钥）
    pub owner: Pubkey,                    // 节点所有者
    pub name: String,                     // 节点名称
    pub device_type: String,              // 设备类型
    pub location: Location,               // 地理位置
    pub registered_at: i64,               // 注册时间戳
    pub last_active_at: i64,              // 最后活跃时间戳
    pub status: NodeStatus,               // 节点状态
    pub total_contributions: u32,         // 总贡献次数
    pub total_compute_score: f64,         // 总计算分数
    pub stake_info: StakeInfo,            // 质押信息
    pub reputation_score: u32,            // 信誉分数 (0-1000)
    pub is_verified: bool,                // 是否已验证
    pub verification_level: u8,           // 验证等级 (0-5)
    pub bump: u8,                         // PDA bump
}

/// 全局节点管理状态
#[account]
pub struct NodeManagementState {
    pub admin: Pubkey,                    // 管理员公钥
    pub total_nodes: u32,                 // 总节点数
    pub active_nodes: u32,                // 活跃节点数
    pub min_stake_amount: u64,            // 最小质押数量
    pub verification_fee: u64,            // 验证费用
    pub bump: u8,                         // PDA bump
}

#[program]
pub mod node_management {
    use super::*;

    /// 初始化节点管理合约
    pub fn initialize(ctx: Context<Initialize>, min_stake_amount: u64, verification_fee: u64) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.total_nodes = 0;
        state.active_nodes = 0;
        state.min_stake_amount = min_stake_amount;
        state.verification_fee = verification_fee;
        state.bump = ctx.bumps.state;

        msg!("Node management contract initialized");
        Ok(())
    }

    /// 注册新节点
    pub fn register_node(
        ctx: Context<RegisterNode>,
        node_id: Pubkey,
        name: String,
        device_type: String,
        location: Location,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 验证输入
        require!(name.len() <= 100, ErrorCode::NameTooLong);
        require!(device_type.len() <= 50, ErrorCode::DeviceTypeTooLong);
        require!(location.country.len() <= 3, ErrorCode::InvalidLocation);
        require!(location.region.len() <= 50, ErrorCode::InvalidLocation);

        let node_account = &mut ctx.accounts.node_account;
        let state = &mut ctx.accounts.state;

        // 初始化节点账户
        node_account.node_id = node_id;
        node_account.owner = ctx.accounts.owner.key();
        node_account.name = name;
        node_account.device_type = device_type;
        node_account.location = location;
        node_account.registered_at = current_time;
        node_account.last_active_at = current_time;
        node_account.status = NodeStatus::Active;
        node_account.total_contributions = 0;
        node_account.total_compute_score = 0.0;
        node_account.stake_info = StakeInfo {
            amount: 0,
            staked_at: 0,
            lock_until: 0,
            is_slashed: false,
        };
        node_account.reputation_score = 500; // 初始信誉分数
        node_account.is_verified = false;
        node_account.verification_level = 0;
        node_account.bump = ctx.bumps.node_account;

        // 更新全局状态
        state.total_nodes += 1;
        state.active_nodes += 1;

        msg!("Node registered: {} ({})", node_account.node_id, node_account.name);
        Ok(())
    }

    /// 更新节点状态
    pub fn update_node_status(
        ctx: Context<UpdateNodeStatus>,
        node_id: Pubkey,
        new_status: NodeStatus,
    ) -> Result<()> {
        let node_account = &mut ctx.accounts.node_account;
        let state = &mut ctx.accounts.state;

        // 只有管理员或节点所有者可以更新状态
        require!(
            ctx.accounts.authority.key() == state.admin || 
            ctx.accounts.authority.key() == node_account.owner,
            ErrorCode::Unauthorized
        );

        // 更新活跃节点统计
        match (node_account.status, new_status) {
            (NodeStatus::Active, NodeStatus::Offline | NodeStatus::Paused | NodeStatus::Banned) => {
                state.active_nodes -= 1;
            }
            (NodeStatus::Offline | NodeStatus::Paused, NodeStatus::Active) => {
                state.active_nodes += 1;
            }
            _ => {}
        }

        node_account.status = new_status;
        node_account.last_active_at = Clock::get()?.unix_timestamp;

        msg!("Node status updated: {} -> {:?}", node_id, new_status);
        Ok(())
    }

    /// 验证节点
    pub fn verify_node(
        ctx: Context<VerifyNode>,
        node_id: Pubkey,
        verification_level: u8,
    ) -> Result<()> {
        let node_account = &mut ctx.accounts.node_account;
        let state = &ctx.accounts.state;

        // 只有管理员可以验证节点
        require!(ctx.accounts.verifier.key() == state.admin, ErrorCode::Unauthorized);
        require!(verification_level <= 5, ErrorCode::InvalidVerificationLevel);

        node_account.is_verified = true;
        node_account.verification_level = verification_level;

        msg!("Node verified: {} at level {}", node_id, verification_level);
        Ok(())
    }

    /// 罚没节点
    pub fn slash_node(
        ctx: Context<SlashNode>,
        node_id: Pubkey,
        slash_ratio: u32, // 罚没比例 (0-10000, 基点)
    ) -> Result<()> {
        let node_account = &mut ctx.accounts.node_account;
        let state = &ctx.accounts.state;

        // 只有管理员可以罚没节点
        require!(ctx.accounts.authority.key() == state.admin, ErrorCode::Unauthorized);
        require!(slash_ratio <= 10000, ErrorCode::InvalidSlashRatio);

        // 计算罚没金额
        let slash_amount = (node_account.stake_info.amount * slash_ratio as u64) / 10000;
        
        if slash_amount > 0 {
            // 转移罚没金额到国库
            **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? += slash_amount;
            **ctx.accounts.node_account.to_account_info().try_borrow_mut_lamports()? -= slash_amount;
            
            node_account.stake_info.amount -= slash_amount;
            node_account.stake_info.is_slashed = true;
        }

        // 将节点状态设为禁用
        node_account.status = NodeStatus::Banned;
        state.active_nodes -= 1;

        msg!("Node slashed: {} amount: {} lamports", node_id, slash_amount);
        Ok(())
    }

    /// 更新节点活跃时间
    pub fn update_last_active(
        ctx: Context<UpdateLastActive>,
        node_id: Pubkey,
    ) -> Result<()> {
        let node_account = &mut ctx.accounts.node_account;
        
        // 只有节点所有者可以更新活跃时间
        require!(
            ctx.accounts.authority.key() == node_account.owner,
            ErrorCode::Unauthorized
        );

        node_account.last_active_at = Clock::get()?.unix_timestamp;

        msg!("Node last active updated: {}", node_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 4 + 4 + 8 + 8 + 1, // 空间计算
        seeds = [b"node-management-state"],
        bump
    )]
    pub state: Account<'info, NodeManagementState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(node_id: Pubkey)]
pub struct RegisterNode<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 32 + 32 + (4 + 100) + (4 + 50) + 8 + 8 + 1 + 4 + 8 + 8 + 8 + 1, // 空间计算
        seeds = [b"node", node_id.as_ref()],
        bump
    )]
    pub node_account: Account<'info, NodeAccount>,

    #[account(mut)]
    pub state: Account<'info, NodeManagementState>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateNodeStatus<'info> {
    #[account(mut)]
    pub node_account: Account<'info, NodeAccount>,

    #[account(mut)]
    pub state: Account<'info, NodeManagementState>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct VerifyNode<'info> {
    #[account(mut)]
    pub node_account: Account<'info, NodeAccount>,

    pub state: Account<'info, NodeManagementState>,

    pub verifier: Signer<'info>,
}

#[derive(Accounts)]
pub struct SlashNode<'info> {
    #[account(mut)]
    pub node_account: Account<'info, NodeAccount>,

    pub state: Account<'info, NodeManagementState>,

    /// CHECK: 国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateLastActive<'info> {
    #[account(mut)]
    pub node_account: Account<'info, NodeAccount>,

    pub authority: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Node name is too long")]
    NameTooLong,
    #[msg("Device type is too long")]
    DeviceTypeTooLong,
    #[msg("Invalid location data")]
    InvalidLocation,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Invalid verification level")]
    InvalidVerificationLevel,
    #[msg("Invalid slash ratio")]
    InvalidSlashRatio,
}

use anchor_lang::prelude::*;
use shared_types::*;

declare_id!("GOVERNANCE_PROGRAM_ID");

/// 多签账户
#[account]
pub struct MultisigAccount {
    pub owners: Vec<Pubkey>,              // 签名者列表
    pub threshold: u64,                   // 所需签名阈值
    pub nonce: u64,                       // nonce防止重放
    pub is_active: bool,                  // 是否激活
    pub created_at: i64,                  // 创建时间
    pub bump: u8,                         // PDA bump
}

/// 多签交易账户
#[account]
pub struct MultisigTransaction {
    pub multisig: Pubkey,                 // 多签账户地址
    pub program_id: Pubkey,               // 目标程序ID
    pub accounts: Vec<TransactionAccount>, // 账户列表
    pub data: Vec<u8>,                    // 指令数据
    pub signers: Vec<bool>,               // 签名状态
    pub did_execute: bool,                // 是否已执行
    pub created_at: i64,                 // 创建时间
    pub executed_at: Option<i64>,        // 执行时间
    pub bump: u8,                         // PDA bump
}

/// 治理提案账户
#[account]
pub struct GovernanceProposal {
    pub id: String,                       // 提案ID
    pub proposer: Pubkey,                 // 提案者
    pub title: String,                    // 提案标题
    pub description: String,              // 提案描述
    pub proposal_type: ProposalType,      // 提案类型
    pub target_program: Pubkey,           // 目标程序ID
    pub target_accounts: Vec<TransactionAccount>, // 目标账户
    pub instruction_data: Vec<u8>,        // 指令数据
    pub voting_start_at: i64,             // 投票开始时间
    pub voting_end_at: i64,               // 投票结束时间
    pub votes_for: u64,                   // 赞成票数
    pub votes_against: u64,               // 反对票数
    pub status: ProposalStatus,           // 提案状态
    pub execution_result: Option<String>,  // 执行结果
    pub created_at: i64,                  // 创建时间
    pub bump: u8,                         // PDA bump
}

/// 治理全局状态
#[account]
pub struct GovernanceState {
    pub admin: Pubkey,                    // 管理员公钥
    pub total_proposals: u64,             // 总提案数
    pub voting_period: u64,               // 投票周期（秒）
    pub execution_delay: u64,             // 执行延迟（秒）
    pub min_voting_power: u64,            // 最小投票权
    pub quorum: u64,                      // 法定人数
    pub is_active: bool,                  // 是否激活
    pub bump: u8,                         // PDA bump
}

/// 提案类型
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum ProposalType {
    ParameterUpdate,    // 参数更新
    ContractUpgrade,    // 合约升级
    TreasuryManagement, // 资金管理
    NodeManagement,     // 节点管理
    Other,              // 其他
}

/// 提案状态
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,           // 待投票
    Active,            // 投票中
    Passed,            // 通过
    Rejected,          // 拒绝
    Executed,          // 已执行
    Failed,            // 执行失败
    Expired,           // 已过期
}

#[program]
pub mod governance {
    use super::*;

    /// 初始化治理合约
    pub fn initialize(
        ctx: Context<Initialize>,
        voting_period: u64,
        execution_delay: u64,
        min_voting_power: u64,
        quorum: u64,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.total_proposals = 0;
        state.voting_period = voting_period;
        state.execution_delay = execution_delay;
        state.min_voting_power = min_voting_power;
        state.quorum = quorum;
        state.is_active = true;
        state.bump = ctx.bumps.state;

        msg!("Governance contract initialized");
        Ok(())
    }

    /// 创建多签账户
    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {
        require!(owners.len() >= 1, ErrorCode::InvalidOwners);
        require!(threshold > 0 && threshold <= owners.len() as u64, ErrorCode::InvalidThreshold);

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        let multisig_account = &mut ctx.accounts.multisig_account;
        multisig_account.owners = owners;
        multisig_account.threshold = threshold;
        multisig_account.nonce = 0;
        multisig_account.is_active = true;
        multisig_account.created_at = current_time;
        multisig_account.bump = ctx.bumps.multisig_account;

        msg!("Multisig account created with {} owners and threshold {}", multisig_account.owners.len(), threshold);
        Ok(())
    }

    /// 创建多签交易
    pub fn create_multisig_transaction(
        ctx: Context<CreateMultisigTransaction>,
        program_id: Pubkey,
        accounts: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<()> {
        let multisig_transaction = &mut ctx.accounts.multisig_transaction;
        let multisig_account = &mut ctx.accounts.multisig_account;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        multisig_transaction.multisig = multisig_account.key();
        multisig_transaction.program_id = program_id;
        multisig_transaction.accounts = accounts;
        multisig_transaction.data = data;
        multisig_transaction.signers = vec![false; multisig_account.owners.len()];
        multisig_transaction.did_execute = false;
        multisig_transaction.created_at = current_time;
        multisig_transaction.executed_at = None;
        multisig_transaction.bump = ctx.bumps.multisig_transaction;

        // 自动设置创建者签名
        let creator_index = multisig_account.owners.iter().position(|&owner| owner == ctx.accounts.creator.key());
        if let Some(index) = creator_index {
            multisig_transaction.signers[index] = true;
        }

        multisig_account.nonce += 1;

        msg!("Multisig transaction created");
        Ok(())
    }

    /// 批准多签交易
    pub fn approve_multisig_transaction(ctx: Context<ApproveMultisigTransaction>) -> Result<()> {
        let multisig_transaction = &mut ctx.accounts.multisig_transaction;
        let multisig_account = &ctx.accounts.multisig_account;

        require!(!multisig_transaction.did_execute, ErrorCode::TransactionAlreadyExecuted);

        // 查找签名者索引
        let signer_index = multisig_account.owners.iter().position(|&owner| owner == ctx.accounts.signer.key());
        require!(signer_index.is_some(), ErrorCode::Unauthorized);
        let index = signer_index.unwrap();
        require!(!multisig_transaction.signers[index], ErrorCode::AlreadySigned);

        // 设置签名
        multisig_transaction.signers[index] = true;

        msg!("Multisig transaction approved by signer {}", index);
        Ok(())
    }

    /// 执行多签交易
    pub fn execute_multisig_transaction(ctx: Context<ExecuteMultisigTransaction>) -> Result<()> {
        let multisig_transaction = &mut ctx.accounts.multisig_transaction;
        let multisig_account = &ctx.accounts.multisig_account;

        require!(!multisig_transaction.did_execute, ErrorCode::TransactionAlreadyExecuted);

        // 检查签名数量是否达到阈值
        let signed_count = multisig_transaction.signers.iter().filter(|&&signed| signed).count() as u64;
        require!(signed_count >= multisig_account.threshold, ErrorCode::InsufficientSignatures);

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 这里应该执行实际的指令调用
        // 为了简化，我们标记为已执行
        multisig_transaction.did_execute = true;
        multisig_transaction.executed_at = Some(current_time);

        msg!("Multisig transaction executed");
        Ok(())
    }

    /// 创建治理提案
    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        id: String,
        title: String,
        description: String,
        proposal_type: ProposalType,
        target_program: Pubkey,
        target_accounts: Vec<TransactionAccount>,
        instruction_data: Vec<u8>,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        let state = &ctx.accounts.state;

        let proposal = &mut ctx.accounts.proposal;
        proposal.id = id.clone();
        proposal.proposer = ctx.accounts.proposer.key();
        proposal.title = title;
        proposal.description = description;
        proposal.proposal_type = proposal_type;
        proposal.target_program = target_program;
        proposal.target_accounts = target_accounts;
        proposal.instruction_data = instruction_data;
        proposal.voting_start_at = current_time;
        proposal.voting_end_at = current_time + state.voting_period as i64;
        proposal.votes_for = 0;
        proposal.votes_against = 0;
        proposal.status = ProposalStatus::Active;
        proposal.execution_result = None;
        proposal.created_at = current_time;
        proposal.bump = ctx.bumps.proposal;

        msg!("Governance proposal created: {}", id);
        Ok(())
    }

    /// 对提案投票
    pub fn vote_on_proposal(
        ctx: Context<VoteOnProposal>,
        proposal_id: String,
        vote: bool, // true for yes, false for no
    ) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 验证投票时间
        require!(current_time >= proposal.voting_start_at, ErrorCode::VotingNotStarted);
        require!(current_time <= proposal.voting_end_at, ErrorCode::VotingEnded);
        require!(proposal.status == ProposalStatus::Active, ErrorCode::ProposalNotActive);

        // 简化实现：每次投票算1票
        // 实际应该根据投票者的代币数量或质押数量计算投票权
        if vote {
            proposal.votes_for += 1;
        } else {
            proposal.votes_against += 1;
        }

        msg!("Vote cast on proposal {}: {}", proposal_id, if vote { "YES" } else { "NO" });
        Ok(())
    }

    /// 执行通过的提案
    pub fn execute_proposal(
        ctx: Context<ExecuteProposal>,
        proposal_id: String,
    ) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let state = &ctx.accounts.state;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 验证提案状态
        require!(proposal.status == ProposalStatus::Passed, ErrorCode::ProposalNotPassed);
        require!(current_time >= proposal.voting_end_at + state.execution_delay as i64, ErrorCode::ExecutionDelayNotMet);

        // 这里应该执行实际的指令调用
        // 简化实现，标记为已执行
        proposal.status = ProposalStatus::Executed;
        proposal.execution_result = Some("Executed successfully".to_string());

        msg!("Proposal executed: {}", proposal_id);
        Ok(())
    }

    /// 结束投票并更新提案状态
    pub fn finalize_proposal(
        ctx: Context<FinalizeProposal>,
        proposal_id: String,
    ) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let state = &ctx.accounts.state;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // 验证投票时间已结束
        require!(current_time > proposal.voting_end_at, ErrorCode::VotingNotEnded);
        require!(proposal.status == ProposalStatus::Active, ErrorCode::ProposalNotActive);

        // 检查是否达到法定人数
        let total_votes = proposal.votes_for + proposal.votes_against;
        if total_votes < state.quorum {
            proposal.status = ProposalStatus::Expired;
        } else if proposal.votes_for > proposal.votes_against {
            proposal.status = ProposalStatus::Passed;
        } else {
            proposal.status = ProposalStatus::Rejected;
        }

        msg!("Proposal finalized: {} -> {:?}", proposal_id, proposal.status);
        Ok(())
    }

    /// 更新治理参数
    pub fn update_governance_params(
        ctx: Context<UpdateGovernanceParams>,
        voting_period: Option<u64>,
        execution_delay: Option<u64>,
        min_voting_power: Option<u64>,
        quorum: Option<u64>,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;

        // 只有管理员可以更新参数
        require!(ctx.accounts.authority.key() == state.admin, ErrorCode::Unauthorized);

        if let Some(vp) = voting_period {
            state.voting_period = vp;
        }
        if let Some(ed) = execution_delay {
            state.execution_delay = ed;
        }
        if let Some(mvp) = min_voting_power {
            state.min_voting_power = mvp;
        }
        if let Some(q) = quorum {
            state.quorum = q;
        }

        msg!("Governance parameters updated");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 8 + 8 + 8 + 8 + 8 + 1 + 1, // 空间计算
        seeds = [b"governance-state"],
        bump
    )]
    pub state: Account<'info, GovernanceState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(owners: Vec<Pubkey>)]
pub struct CreateMultisig<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + (4 + 32 * 10) + 8 + 8 + 1 + 8, // 支持最多10个签名者
        seeds = [b"multisig", creator.key().as_ref()],
        bump
    )]
    pub multisig_account: Account<'info, MultisigAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateMultisigTransaction<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + 32 + 32 + (4 + 32 * 10) + (4 + 1000) + (4 + 10) + 1 + 8 + 8 + 1, // 空间计算
        seeds = [b"multisig-tx", multisig_account.key().as_ref(), &multisig_account.nonce.to_le_bytes()],
        bump
    )]
    pub multisig_transaction: Account<'info, MultisigTransaction>,

    #[account(mut)]
    pub multisig_account: Account<'info, MultisigAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveMultisigTransaction<'info> {
    #[account(mut)]
    pub multisig_transaction: Account<'info, MultisigTransaction>,

    pub multisig_account: Account<'info, MultisigAccount>,

    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteMultisigTransaction<'info> {
    #[account(mut)]
    pub multisig_transaction: Account<'info, MultisigTransaction>,

    pub multisig_account: Account<'info, MultisigAccount>,

    pub executor: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(id: String)]
pub struct CreateProposal<'info> {
    #[account(
        init,
        payer = proposer,
        space = 8 + (4 + 36) + 32 + (4 + 100) + (4 + 1000) + 1 + 32 + (4 + 32 * 10) + (4 + 1000) + 8 + 8 + 8 + 8 + 1 + (4 + 100) + 8 + 1, // 空间计算
        seeds = [b"proposal", id.as_bytes()],
        bump
    )]
    pub proposal: Account<'info, GovernanceProposal>,

    pub state: Account<'info, GovernanceState>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VoteOnProposal<'info> {
    #[account(mut)]
    pub proposal: Account<'info, GovernanceProposal>,

    pub voter: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteProposal<'info> {
    #[account(mut)]
    pub proposal: Account<'info, GovernanceProposal>,

    pub executor: Signer<'info>,
}

#[derive(Accounts)]
pub struct FinalizeProposal<'info> {
    #[account(mut)]
    pub proposal: Account<'info, GovernanceProposal>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateGovernanceParams<'info> {
    #[account(mut)]
    pub state: Account<'info, GovernanceState>,

    pub authority: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid owners list")]
    InvalidOwners,
    #[msg("Invalid threshold")]
    InvalidThreshold,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Transaction already executed")]
    TransactionAlreadyExecuted,
    #[msg("Already signed")]
    AlreadySigned,
    #[msg("Insufficient signatures")]
    InsufficientSignatures,
    #[msg("Voting not started")]
    VotingNotStarted,
    #[msg("Voting ended")]
    VotingEnded,
    #[msg("Proposal not active")]
    ProposalNotActive,
    #[msg("Proposal not passed")]
    ProposalNotPassed,
    #[msg("Execution delay not met")]
    ExecutionDelayNotMet,
}

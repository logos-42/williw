//! Solana 指令构建
//!
//! 本模块提供构建 Solana 程序指令的函数。

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

use super::types::*;

/// 指令标识符
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum DecentralizedTrainingInstruction {
    /// 初始化合约
    Initialize,
    /// 注册节点
    RegisterNode {
        node_id: Pubkey,
        name: String,
        device_type: String,
    },
    /// 记录算力贡献
    RecordContribution {
        contribution_id: String,
        task_id: String,
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
    },
    /// 分配收益
    DistributeRewards,
    /// 创建多签账户
    CreateMultisig {
        owners: Vec<Pubkey>,
        threshold: u64,
    },
    /// 创建多签交易
    CreateMultisigTransaction {
        program_id: Pubkey,
        accounts: Vec<TransactionAccount>,
        data: Vec<u8>,
    },
    /// 批准多签交易
    ApproveMultisigTransaction,
    /// 执行多签交易
    ExecuteMultisigTransaction,
}

/// 交易账户元数据
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// 构建初始化指令
pub fn build_initialize_instruction(
    program_id: &Pubkey,
    admin: &Pubkey,
    treasury: &Pubkey,
    global_state: &Pubkey,
) -> Result<Instruction> {
    let data = borsh::to_vec(&DecentralizedTrainingInstruction::Initialize)
        .map_err(|e| anyhow!("Failed to serialize instruction: {}", e))?;

    let accounts = vec![
        AccountMeta::new(*global_state, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*treasury, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// 构建注册节点指令
pub fn build_register_node_instruction(
    program_id: &Pubkey,
    node_account: &Pubkey,
    global_state: &Pubkey,
    owner: &Pubkey,
    node_id: Pubkey,
    name: String,
    device_type: String,
) -> Result<Instruction> {
    let data = borsh::to_vec(&DecentralizedTrainingInstruction::RegisterNode {
        node_id,
        name,
        device_type,
    })
    .map_err(|e| anyhow!("Failed to serialize instruction: {}", e))?;

    let accounts = vec![
        AccountMeta::new(*node_account, false),
        AccountMeta::new(*global_state, false),
        AccountMeta::new(*owner, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// 构建记录贡献指令
pub fn build_record_contribution_instruction(
    program_id: &Pubkey,
    contribution_account: &Pubkey,
    node_account: &Pubkey,
    global_state: &Pubkey,
    authority: &Pubkey,
    contribution_id: String,
    task_id: String,
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
) -> Result<Instruction> {
    let data = borsh::to_vec(&DecentralizedTrainingInstruction::RecordContribution {
        contribution_id,
        task_id,
        start_timestamp,
        end_timestamp,
        duration_seconds,
        avg_gpu_usage_percent,
        gpu_memory_used_mb,
        avg_cpu_usage_percent,
        memory_used_mb,
        network_upload_mb,
        network_download_mb,
        samples_processed,
        batches_processed,
        compute_score,
    })
    .map_err(|e| anyhow!("Failed to serialize instruction: {}", e))?;

    let accounts = vec![
        AccountMeta::new(*contribution_account, false),
        AccountMeta::new(*node_account, false),
        AccountMeta::new(*global_state, false),
        AccountMeta::new(*authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// 构建分配收益指令
pub fn build_distribute_rewards_instruction(
    program_id: &Pubkey,
    reward_account: &Pubkey,
    node_account: &Pubkey,
    global_state: &Pubkey,
    treasury: &Pubkey,
    node_wallet: &Pubkey,
    authority: &Pubkey,
) -> Result<Instruction> {
    let data = borsh::to_vec(&DecentralizedTrainingInstruction::DistributeRewards)
        .map_err(|e| anyhow!("Failed to serialize instruction: {}", e))?;

    let accounts = vec![
        AccountMeta::new(*reward_account, false),
        AccountMeta::new(*node_account, false),
        AccountMeta::new(*global_state, false),
        AccountMeta::new(*treasury, false),
        AccountMeta::new(*node_wallet, false),
        AccountMeta::new(*authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// 构建创建多签指令
pub fn build_create_multisig_instruction(
    program_id: &Pubkey,
    multisig_account: &Pubkey,
    creator: &Pubkey,
    owners: Vec<Pubkey>,
    threshold: u64,
) -> Result<Instruction> {
    let data = borsh::to_vec(&DecentralizedTrainingInstruction::CreateMultisig {
        owners,
        threshold,
    })
    .map_err(|e| anyhow!("Failed to serialize instruction: {}", e))?;

    let accounts = vec![
        AccountMeta::new(*multisig_account, false),
        AccountMeta::new(*creator, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

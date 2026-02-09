//! 模块化 Solana 客户端
//!
//! 支持拆分后的智能合约架构

use anyhow::{anyhow, Result};
use std::sync::Arc;
use parking_lot::RwLock;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use super::types::*;
use super::compute::{ComputeTracker, ComputeCalculator, ContributionLevel};
use super::rewards::{RewardManager, RewardSettler};

/// 模块化 Solana 客户端
pub struct ModularSolanaClient {
    /// RPC 客户端
    rpc_client: RpcClient,
    /// 支付者密钥对
    payer_keypair: Option<Keypair>,
    /// 程序 ID 配置
    program_ids: ProgramIds,
    /// 算力跟踪器
    compute_tracker: Arc<RwLock<ComputeTracker>>,
    /// 收益管理器
    reward_manager: Arc<RwLock<RewardManager>>,
    /// 收益结算器
    reward_settler: Arc<RwLock<RewardSettler>>,
}

/// 程序 ID 配置
#[derive(Debug, Clone)]
pub struct ProgramIds {
    pub node_management: Pubkey,
    pub contribution_tracking: Pubkey,
    pub reward_management: Pubkey,
    pub governance: Pubkey,
}

impl ModularSolanaClient {
    /// 创建新的模块化 Solana 客户端
    pub fn new(
        rpc_url: String,
        program_ids: ProgramIds,
        node_id: String,
        payer_keypair_base58: Option<String>,
    ) -> Result<Self> {
        let commitment = CommitmentConfig::confirmed();
        let rpc_client = RpcClient::new_with_commitment(&rpc_url, commitment);
        
        // 解析支付者密钥对
        let payer_keypair = if let Some(keypair_base58) = payer_keypair_base58 {
            Some(Keypair::from_base58_string(&keypair_base58)
                .map_err(|e| anyhow!("Invalid keypair: {}", e))?)
        } else {
            None
        };
        
        Ok(Self {
            rpc_client,
            payer_keypair,
            program_ids,
            compute_tracker: Arc::new(RwLock::new(ComputeTracker::new(node_id))),
            reward_manager: Arc::new(RwLock::new(RewardManager::with_defaults())),
            reward_settler: Arc::new(RwLock::new(RewardSettler::with_defaults())),
        })
    }

    /// 检查连接状态
    pub async fn check_connection(&self) -> Result<bool> {
        match self.rpc_client.get_version() {
            Ok(_) => Ok(true),
            Err(e) => {
                log::warn!("Failed to connect to Solana RPC: {}", e);
                Ok(false)
            }
        }
    }

    // ============ 节点管理 ============

    /// 注册节点到区块链
    pub async fn register_node(&self, node_info: NodeInfo) -> Result<TransactionResult> {
        log::info!("注册节点到区块链: {}", node_info.node_id);

        if let Some(payer) = &self.payer_keypair {
            let node_id = node_info.node_id.parse::<Pubkey>()
                .map_err(|e| anyhow!("Invalid node ID: {}", e))?;
            let owner = node_info.owner_address.parse::<Pubkey>()
                .map_err(|e| anyhow!("Invalid owner address: {}", e))?;

            // 查找 PDA
            let (node_account_pda, _) = Pubkey::find_program_address(
                &[b"node", node_id.as_ref()],
                &self.program_ids.node_management
            );
            let (state_pda, _) = Pubkey::find_program_address(
                &[b"node-management-state"],
                &self.program_ids.node_management
            );

            // 构建指令
            let instruction = self.build_register_node_instruction(
                &node_account_pda,
                &state_pda,
                &owner,
                node_id,
                node_info.name.clone(),
                node_info.device_type.clone(),
            )?;

            // 创建交易
            let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
            let recent_blockhash = self.rpc_client.get_latest_blockhash()
                .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;
            transaction.sign(&[payer], recent_blockhash);

            // 发送交易
            match self.send_transaction_with_retry(&transaction, 3).await {
                Ok(signature) => Ok(TransactionResult {
                    signature: signature.to_string(),
                    success: true,
                    error: None,
                }),
                Err(e) => Ok(TransactionResult {
                    signature: "".to_string(),
                    success: false,
                    error: Some(format!("Transaction failed: {}", e)),
                }),
            }
        } else {
            // 模拟实现
            Ok(TransactionResult {
                signature: "mock_signature_".to_string() + &node_info.node_id,
                success: true,
                error: None,
            })
        }
    }

    /// 更新节点状态
    pub async fn update_node_status(
        &self,
        node_id: &str,
        status: NodeStatus,
    ) -> Result<TransactionResult> {
        log::info!("更新节点状态: {} -> {:?}", node_id, status);

        // TODO: 实现实际的链上状态更新逻辑
        Ok(TransactionResult {
            signature: format!("mock_status_update_{}", node_id),
            success: true,
            error: None,
        })
    }

    // ============ 贡献跟踪 ============

    /// 上报算力贡献
    pub async fn report_compute_contribution(
        &self,
        contribution: ComputeContribution,
    ) -> Result<TransactionResult> {
        log::info!(
            "上报算力贡献: 节点={}, 任务={}, 算力评分={:.2}",
            contribution.node_id,
            contribution.task_id,
            contribution.compute_score
        );

        if let Some(payer) = &self.payer_keypair {
            let node_id = contribution.node_id.parse::<Pubkey>()
                .map_err(|e| anyhow!("Invalid node ID: {}", e))?;

            // 查找 PDA
            let (contribution_account_pda, _) = Pubkey::find_program_address(
                &[b"contribution", contribution.id.as_bytes()],
                &self.program_ids.contribution_tracking
            );
            let (state_pda, _) = Pubkey::find_program_address(
                &[b"contribution-tracking-state"],
                &self.program_ids.contribution_tracking
            );

            // 构建指令
            let instruction = self.build_record_contribution_instruction(
                &contribution_account_pda,
                &state_pda,
                &payer.pubkey(),
                contribution.id.clone(),
                contribution.task_id.clone(),
                contribution.start_timestamp,
                contribution.end_timestamp,
                contribution.duration_seconds,
                contribution.avg_gpu_usage_percent,
                contribution.gpu_memory_used_mb,
                contribution.avg_cpu_usage_percent,
                contribution.memory_used_mb,
                contribution.network_upload_mb,
                contribution.network_download_mb,
                contribution.samples_processed,
                contribution.batches_processed,
                contribution.compute_score,
            )?;

            // 创建交易
            let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
            let recent_blockhash = self.rpc_client.get_latest_blockhash()
                .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;
            transaction.sign(&[payer], recent_blockhash);

            // 发送交易
            match self.send_transaction_with_retry(&transaction, 3).await {
                Ok(signature) => Ok(TransactionResult {
                    signature: signature.to_string(),
                    success: true,
                    error: None,
                }),
                Err(e) => Ok(TransactionResult {
                    signature: "".to_string(),
                    success: false,
                    error: Some(format!("Transaction failed: {}", e)),
                }),
            }
        } else {
            // 模拟实现
            Ok(TransactionResult {
                signature: format!("mock_contribution_{}", contribution.id),
                success: true,
                error: None,
            })
        }
    }

    // ============ 收益管理 ============

    /// 分配收益到节点
    pub async fn distribute_rewards(
        &self,
        node_id: &str,
        contribution_id: String,
        amount_lamports: u64,
    ) -> Result<TransactionResult> {
        log::info!("分配收益: 节点={}, 金额={} lamports", node_id, amount_lamports);

        if let Some(payer) = &self.payer_keypair {
            let node_pubkey = node_id.parse::<Pubkey>()
                .map_err(|e| anyhow!("Invalid node ID: {}", e))?;

            // 查找 PDA
            let (reward_account_pda, _) = Pubkey::find_program_address(
                &[b"reward", node_pubkey.as_ref(), &Clock::get().unwrap().unix_timestamp.to_le_bytes()],
                &self.program_ids.reward_management
            );
            let (node_summary_pda, _) = Pubkey::find_program_address(
                &[b"node-reward-summary", node_pubkey.as_ref()],
                &self.program_ids.reward_management
            );
            let (state_pda, _) = Pubkey::find_program_address(
                &[b"reward-management-state"],
                &self.program_ids.reward_management
            );

            // 构建指令
            let instruction = self.build_distribute_rewards_instruction(
                &reward_account_pda,
                &node_summary_pda,
                &state_pda,
                node_pubkey,
                contribution_id,
                amount_lamports,
            )?;

            // 创建交易
            let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
            let recent_blockhash = self.rpc_client.get_latest_blockhash()
                .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;
            transaction.sign(&[payer], recent_blockhash);

            // 发送交易
            match self.send_transaction_with_retry(&transaction, 3).await {
                Ok(signature) => Ok(TransactionResult {
                    signature: signature.to_string(),
                    success: true,
                    error: None,
                }),
                Err(e) => Ok(TransactionResult {
                    signature: "".to_string(),
                    success: false,
                    error: Some(format!("Transaction failed: {}", e)),
                }),
            }
        } else {
            // 模拟实现
            Ok(TransactionResult {
                signature: format!("mock_reward_{}", node_id),
                success: true,
                error: None,
            })
        }
    }

    // ============ 辅助函数 ============

    /// 发送交易并确认（带重试）
    async fn send_transaction_with_retry(
        &self,
        transaction: &Transaction,
        max_retries: u32,
    ) -> Result<solana_sdk::signature::Signature> {
        let mut retries = 0;
        
        loop {
            match self.rpc_client.send_and_confirm_transaction(transaction) {
                Ok(signature) => {
                    log::info!("Transaction sent successfully: {}", signature);
                    return Ok(signature);
                }
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(anyhow!("Transaction failed after {} retries: {}", max_retries, e));
                    }
                    
                    log::warn!("Transaction failed (attempt {}/{}): {}, retrying...", retries, max_retries, e);
                    
                    // 等待一段时间后重试
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000 * retries as u64)).await;
                }
            }
        }
    }

    /// 构建注册节点指令
    fn build_register_node_instruction(
        &self,
        node_account: &Pubkey,
        state: &Pubkey,
        owner: &Pubkey,
        node_id: Pubkey,
        name: String,
        device_type: String,
    ) -> Result<solana_sdk::instruction::Instruction> {
        use solana_sdk::instruction::{Instruction, AccountMeta};

        // 序列化指令数据
        let mut data = Vec::new();
        data.extend_from_slice(&node_id.to_bytes());
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name.as_bytes());
        data.extend_from_slice(&(device_type.len() as u32).to_le_bytes());
        data.extend_from_slice(device_type.as_bytes());

        Ok(Instruction {
            program_id: self.program_ids.node_management,
            accounts: vec![
                AccountMeta::new(*node_account, false),
                AccountMeta::new(*state, false),
                AccountMeta::new(*owner, true),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data,
        })
    }

    /// 构建记录贡献指令
    fn build_record_contribution_instruction(
        &self,
        contribution_account: &Pubkey,
        state: &Pubkey,
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
    ) -> Result<solana_sdk::instruction::Instruction> {
        use solana_sdk::instruction::{Instruction, AccountMeta};

        // 序列化指令数据
        let mut data = Vec::new();
        
        // 贡献 ID
        data.extend_from_slice(&(contribution_id.len() as u32).to_le_bytes());
        data.extend_from_slice(contribution_id.as_bytes());
        
        // 任务 ID
        data.extend_from_slice(&(task_id.len() as u32).to_le_bytes());
        data.extend_from_slice(task_id.as_bytes());
        
        // 时间戳和持续时间
        data.extend_from_slice(&start_timestamp.to_le_bytes());
        data.extend_from_slice(&end_timestamp.to_le_bytes());
        data.extend_from_slice(&duration_seconds.to_le_bytes());
        
        // 资源使用数据
        data.extend_from_slice(&avg_gpu_usage_percent.to_le_bytes());
        data.extend_from_slice(&gpu_memory_used_mb.to_le_bytes());
        data.extend_from_slice(&avg_cpu_usage_percent.to_le_bytes());
        data.extend_from_slice(&memory_used_mb.to_le_bytes());
        data.extend_from_slice(&network_upload_mb.to_le_bytes());
        data.extend_from_slice(&network_download_mb.to_le_bytes());
        data.extend_from_slice(&samples_processed.to_le_bytes());
        data.extend_from_slice(&batches_processed.to_le_bytes());
        data.extend_from_slice(&compute_score.to_le_bytes());
        
        Ok(Instruction {
            program_id: self.program_ids.contribution_tracking,
            accounts: vec![
                AccountMeta::new(*contribution_account, false),
                AccountMeta::new(*state, false),
                AccountMeta::new_readonly(*authority, true),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data,
        })
    }

    /// 构建分配收益指令
    fn build_distribute_rewards_instruction(
        &self,
        reward_account: &Pubkey,
        node_summary: &Pubkey,
        state: &Pubkey,
        node_id: Pubkey,
        contribution_id: String,
        amount_lamports: u64,
    ) -> Result<solana_sdk::instruction::Instruction> {
        use solana_sdk::instruction::{Instruction, AccountMeta};

        // 序列化指令数据
        let mut data = Vec::new();
        data.extend_from_slice(&node_id.to_bytes());
        data.extend_from_slice(&(contribution_id.len() as u32).to_le_bytes());
        data.extend_from_slice(contribution_id.as_bytes());
        data.extend_from_slice(&amount_lamports.to_le_bytes());
        
        Ok(Instruction {
            program_id: self.program_ids.reward_management,
            accounts: vec![
                AccountMeta::new(*reward_account, false),
                AccountMeta::new(*node_summary, false),
                AccountMeta::new(*state, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data,
        })
    }
}

// 导入 Clock 用于时间戳
use solana_sdk::clock::Clock;

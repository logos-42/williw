//! Solana 区块链客户端
//!
//! 本模块提供与 Solana 区块链交互的客户端功能。

use anyhow::{anyhow, Result};
use chrono::Utc;
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
use super::SolanaConfig;
use super::compute::{ComputeTracker, ComputeCalculator, ContributionLevel};
use super::rewards::{RewardManager, RewardSettler};
use super::accounts::*;
use super::instruction::*;

/// Solana 客户端
pub struct SolanaClient {
    /// 配置
    config: SolanaConfig,
    /// RPC 客户端
    rpc_client: RpcClient,
    /// 支付者密钥对
    payer_keypair: Option<Keypair>,
    /// 算力跟踪器
    compute_tracker: Arc<RwLock<ComputeTracker>>,
    /// 收益管理器
    reward_manager: Arc<RwLock<RewardManager>>,
    /// 收益结算器
    reward_settler: Arc<RwLock<RewardSettler>>,
}

impl SolanaClient {
    /// 创建新的 Solana 客户端
    pub fn new(config: SolanaConfig, node_id: String) -> Result<Self> {
        let commitment = CommitmentConfig::confirmed();
        let rpc_client = RpcClient::new_with_commitment(&config.rpc_url, commitment);
        
        // 解析程序 ID
        let _program_id = config.program_id.parse::<Pubkey>()
            .map_err(|e| anyhow!("Invalid program ID: {}", e))?;
        
        // 创建支付者密钥对
        let payer_keypair = if let Some(keypair_base58) = &config.payer_keypair_base58 {
            Some(Keypair::from_base58_string(keypair_base58)
                .map_err(|e| anyhow!("Invalid keypair: {}", e))?)
        } else {
            None
        };
        
        Ok(Self {
            config,
            rpc_client,
            payer_keypair,
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
    
    /// 获取程序账户
    pub async fn get_program_account(&self, address: &str) -> Result<Pubkey> {
        address.parse::<Pubkey>()
            .map_err(|e| anyhow!("Invalid address: {}", e))
    }

    /// 获取算力跟踪器
    pub fn get_compute_tracker(&self) -> Arc<RwLock<ComputeTracker>> {
        Arc::clone(&self.compute_tracker)
    }

    /// 获取收益管理器
    pub fn get_reward_manager(&self) -> Arc<RwLock<RewardManager>> {
        Arc::clone(&self.reward_manager)
    }

    /// 获取收益结算器
    pub fn get_reward_settler(&self) -> Arc<RwLock<RewardSettler>> {
        Arc::clone(&self.reward_settler)
    }

    /// 获取配置
    pub fn get_config(&self) -> &SolanaConfig {
        &self.config
    }

    // ============ 节点管理 ============

    /// 注册节点到区块链
    pub async fn register_node(&self, node_info: NodeInfo) -> Result<TransactionResult> {
        log::info!("注册节点到区块链: {}", node_info.node_id);

        // 如果有支付者密钥，使用真实的智能合约调用
        if let Some(payer) = &self.payer_keypair {
            let program_id = self.get_program_account(&self.config.program_id).await?;
            let node_id = self.get_program_account(&node_info.node_id).await?;
            let owner = self.get_program_account(&node_info.owner_address).await?;

            // 查找 PDA
            let (node_account_pda, _) = find_node_account_pda(&node_id, &program_id);
            let (global_state_pda, _) = find_global_state_pda(&program_id);

            // 构建指令
            let instruction = build_register_node_instruction(
                &program_id,
                &node_account_pda,
                &global_state_pda,
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
            // 模拟实现（用于测试）
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

    /// 查询节点信息
    pub async fn get_node_info(&self, node_id: &str) -> Result<NodeInfo> {
        log::info!("查询节点信息: {}", node_id);

        // TODO: 实现实际的链上查询逻辑
        // 返回模拟数据
        Ok(NodeInfo {
            node_id: node_id.to_string(),
            owner_address: "mock_owner_address".to_string(),
            name: "Mock Node".to_string(),
            device_type: "Desktop".to_string(),
            registered_at: Utc::now().timestamp(),
            last_active_at: Utc::now().timestamp(),
            status: NodeStatus::Active,
        })
    }

    // ============ 算力贡献 ============

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

        // 如果有支付者密钥，使用真实的智能合约调用
        if let Some(payer) = &self.payer_keypair {
            let program_id = self.get_program_account(&self.config.program_id).await?;
            let node_id = self.get_program_account(&contribution.node_id).await?;

            // 查找 PDA
            let (contribution_account_pda, _) = find_contribution_account_pda(&contribution.id, &program_id);
            let (node_account_pda, _) = find_node_account_pda(&node_id, &program_id);
            let (global_state_pda, _) = find_global_state_pda(&program_id);

            // 构建指令
            let instruction = build_record_contribution_instruction(
                &program_id,
                &contribution_account_pda,
                &node_account_pda,
                &global_state_pda,
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
                    Ok(signature) => {
                        log::info!("Node registration successful: {}", signature);
                        Ok(TransactionResult {
                            signature: signature.to_string(),
                            success: true,
                            error: None,
                        })
                    },
                    Err(e) => {
                        log::error!("Node registration failed: {}", e);
                        Ok(TransactionResult {
                            signature: "".to_string(),
                            success: false,
                            error: Some(format!("Transaction failed: {}", e)),
                        })
                    },
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

    /// 查询节点的算力统计
    pub async fn get_compute_stats(&self, node_id: &str) -> Result<ComputeStats> {
        log::info!("查询算力统计: {}", node_id);

        // TODO: 实现实际的链上查询逻辑
        Ok(ComputeStats {
            node_id: node_id.to_string(),
            total_compute_seconds: 3600,
            total_samples_processed: 10000,
            total_compute_score: 25.5,
            avg_gpu_usage_percent: 75.0,
            avg_cpu_usage_percent: 45.0,
            total_network_mb: 500,
            contribution_count: 10,
        })
    }

    /// 查询节点的贡献等级
    pub async fn get_contribution_level(
        &self,
        node_id: &str,
    ) -> Result<ContributionLevel> {
        let stats = self.get_compute_stats(node_id).await?;
        let level = ComputeCalculator::calculate_contribution_level(
            stats.total_compute_score,
            stats.contribution_count,
        );

        log::info!("节点 {} 的贡献等级: {:?}", node_id, level);

        Ok(level)
    }

    // ============ 收益管理 ============

    /// 查询节点钱包余额
    pub async fn get_wallet_balance(&self, wallet_address: &str) -> Result<NodeWalletBalance> {
        log::info!("查询钱包余额: {}", wallet_address);

        // TODO: 实现实际的链上查询逻辑
        Ok(NodeWalletBalance {
            node_id: "mock_node".to_string(),
            wallet_address: wallet_address.to_string(),
            sol_balance_lamports: 1_000_000_000, // 1 SOL
            pending_rewards_lamports: 5_000_000,
            total_rewards_distributed_lamports: 100_000_000,
            last_updated_at: Utc::now().timestamp(),
        })
    }

    /// 分配收益到节点钱包
    pub async fn distribute_rewards(
        &self,
        distributions: Vec<RewardDistribution>,
    ) -> Result<Vec<TransactionResult>> {
        log::info!("分配收益到 {} 个节点", distributions.len());

        let mut results = Vec::new();

        // 如果有支付者密钥，使用真实的智能合约调用
        if let Some(payer) = &self.payer_keypair {
            let program_id = self.get_program_account(&self.config.program_id).await?;
            let (global_state_pda, _) = find_global_state_pda(&program_id);

            for distribution in distributions {
                let node_id = self.get_program_account(&distribution.node_id).await?;
                let node_wallet = self.get_program_account(&distribution.transaction_signature).await?; // 假设这里存储了钱包地址

                // 查找 PDA
                let (node_account_pda, _) = find_node_account_pda(&node_id, &program_id);
                let reward_count = 0; // 简化处理，实际应该从链上查询
                let (reward_account_pda, _) = find_reward_account_pda(&node_id, reward_count, &program_id);

                // 查找国库地址（简化，从全局状态获取）
                let treasury = self.get_program_account("11111111111111111111111111111112").await?; // 临时地址

                // 构建指令
                let instruction = build_distribute_rewards_instruction(
                    &program_id,
                    &reward_account_pda,
                    &node_account_pda,
                    &global_state_pda,
                    &treasury,
                    &node_wallet,
                    &payer.pubkey(),
                )?;

                // 创建交易
                let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
                let recent_blockhash = self.rpc_client.get_latest_blockhash()
                    .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;
                transaction.sign(&[payer], recent_blockhash);

                // 发送交易
                match self.send_transaction_with_retry(&transaction, 3).await {
                    Ok(signature) => results.push(TransactionResult {
                        signature: signature.to_string(),
                        success: true,
                        error: None,
                    }),
                    Err(e) => results.push(TransactionResult {
                        signature: "".to_string(),
                        success: false,
                        error: Some(format!("Transaction failed: {}", e)),
                    }),
                }
            }
        } else {
            // 模拟实现
            for dist in distributions {
                results.push(TransactionResult {
                    signature: dist.id.clone(),
                    success: true,
                    error: None,
                });
            }
        }

        Ok(results)
    }

    /// 生成结算计划
    pub async fn generate_settlement_plan(
        &self,
        node_wallet_balances: Vec<NodeWalletBalance>,
    ) -> Result<super::rewards::SettlementPlan> {
        let settler = self.reward_settler.read();
        Ok(settler.generate_settlement_plan(&node_wallet_balances))
    }

    /// 执行收益结算
    pub async fn execute_settlement(
        &self,
        settlement_plan: &super::rewards::SettlementPlan,
    ) -> Result<Vec<TransactionResult>> {
        log::info!(
            "执行收益结算: 总金额={}, 节点数={}",
            settlement_plan.total_amount_lamports,
            settlement_plan.nodes_to_settle.len()
        );

        // TODO: 实现实际的链上结算逻辑

        let results: Vec<TransactionResult> = settlement_plan
            .nodes_to_settle
            .iter()
            .map(|node| TransactionResult {
                signature: format!("settlement_{}", node.node_id),
                success: true,
                error: None,
            })
            .collect();

        Ok(results)
    }

    // ============ 合约状态 ============

    /// 查询智能合约状态
    pub async fn get_contract_state(&self) -> Result<ContractState> {
        log::info!("查询合约状态");

        // 如果有程序客户端，查询真实状态
        if self.payer_keypair.is_some() {
            let global_state_pda = self.get_global_state_pda().await?;
            
            // 查询全局状态账户
            match self.rpc_client.get_account(&global_state_pda) {
                Ok(account) => {
                    // 简化解析，实际应该使用 Anchor 的反序列化
                    Ok(ContractState {
                        program_id: self.config.program_id.clone(),
                        admin_address: "mock_admin".to_string(), // 需要从账户数据解析
                        treasury_address: "mock_treasury".to_string(), // 需要从账户数据解析
                        total_nodes: 100, // 需要从账户数据解析
                        total_contributions: 1000, // 需要从账户数据解析
                        total_rewards_distributed_lamports: 1_000_000_000, // 需要从账户数据解析
                        base_reward_per_compute_lamports: 1_000_000, // 需要从账户数据解析
                        reward_pool_balance_lamports: 10_000_000_000, // 需要从账户数据解析
                    })
                }
                Err(e) => {
                    log::warn!("Failed to fetch global state: {}", e);
                    // 返回模拟数据
                    Ok(ContractState {
                        program_id: self.config.program_id.clone(),
                        admin_address: "mock_admin".to_string(),
                        treasury_address: "mock_treasury".to_string(),
                        total_nodes: 100,
                        total_contributions: 1000,
                        total_rewards_distributed_lamports: 1_000_000_000,
                        base_reward_per_compute_lamports: 1_000_000,
                        reward_pool_balance_lamports: 10_000_000_000,
                    })
                }
            }
        } else {
            // 模拟实现
            Ok(ContractState {
                program_id: self.config.program_id.clone(),
                admin_address: "mock_admin".to_string(),
                treasury_address: "mock_treasury".to_string(),
                total_nodes: 100,
                total_contributions: 1000,
                total_rewards_distributed_lamports: 1_000_000_000,
                base_reward_per_compute_lamports: 1_000_000,
                reward_pool_balance_lamports: 10_000_000_000,
            })
        }
    }

    /// 查询当前基础奖励
    pub async fn get_base_reward_per_compute(&self) -> Result<u64> {
        let state = self.get_contract_state().await?;
        Ok(state.base_reward_per_compute_lamports)
    }
    
    // ============ 辅助函数 ============
    
    /// 获取全局状态 PDA
    async fn get_global_state_pda(&self) -> Result<Pubkey> {
        let program_id = self.get_program_account(&self.config.program_id).await?;
        let (pda, _bump) = find_global_state_pda(&program_id);
        Ok(pda)
    }
    
    /// 获取贡献账户 PDA
    async fn get_contribution_pda(&self, contribution_id: &str) -> Result<Pubkey> {
        let program_id = self.get_program_account(&self.config.program_id).await?;
        let (pda, _bump) = find_contribution_account_pda(contribution_id, &program_id);
        Ok(pda)
    }
    
    /// 获取节点账户 PDA
    async fn get_node_pda(&self, node_id: &str) -> Result<Pubkey> {
        let program_id = self.get_program_account(&self.config.program_id).await?;
        let node_pubkey = self.get_program_account(node_id).await?;
        let (pda, _bump) = find_node_account_pda(&node_pubkey, &program_id);
        Ok(pda)
    }
    
    /// 等待交易确认
    pub async fn wait_for_confirmation(&self, signature: &str) -> Result<bool> {
        let signature = signature.parse()
            .map_err(|e| anyhow!("Invalid signature: {}", e))?;
        
        match self.rpc_client.confirm_transaction(&signature) {
            Ok(confirmation) => Ok(confirmation.value.is_some()),
            Err(e) => {
                log::warn!("Failed to confirm transaction {}: {}", signature, e);
                Ok(false)
            }
        }
    }
    
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
    
    /// 获取账户租金豁免最低余额
    pub async fn get_rent_exemption_minimum(&self, data_size: usize) -> Result<u64> {
        match self.rpc_client.get_minimum_balance_for_rent_exemption(data_size) {
            Ok(balance) => Ok(balance),
            Err(e) => {
                log::warn!("Failed to get rent exemption: {}", e);
                // 返回默认值（0.001 SOL）
                Ok(1_000_000)
            }
        }
    }
    
    /// 检查账户是否存在
    pub async fn account_exists(&self, pubkey: &Pubkey) -> Result<bool> {
        match self.rpc_client.get_account(pubkey) {
            Ok(_) => Ok(true),
            Err(solana_client::client_error::ClientError::AccountNotFound) => Ok(false),
            Err(e) => {
                log::warn!("Error checking account existence: {}", e);
                Ok(false)
            }
        }
    }
}

// ============ PDA 查找函数 ============

/// 查找全局状态 PDA
pub fn find_global_state_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"global-state"], program_id)
}

/// 查找节点账户 PDA
pub fn find_node_account_pda(node_id: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"node", node_id.as_ref()], program_id)
}

/// 查找贡献账户 PDA
pub fn find_contribution_account_pda(contribution_id: &str, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"contribution", contribution_id.as_bytes()], program_id)
}

/// 查找收益账户 PDA
pub fn find_reward_account_pda(node_id: &Pubkey, reward_count: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"reward", node_id.as_ref(), &reward_count.to_le_bytes()], program_id)
}

// ============ 指令构建函数 ============

/// 构建注册节点指令
pub fn build_register_node_instruction(
    program_id: &Pubkey,
    node_account: &Pubkey,
    global_state: &Pubkey,
    owner: &Pubkey,
    node_id: Pubkey,
    name: String,
    device_type: String,
) -> Result<solana_sdk::instruction::Instruction> {
    use solana_sdk::instruction::{Instruction, AccountMeta};
    use super::instruction::RegisterNode;
    
    // 构建位置信息
    let location = super::types::Location {
        latitude: 0,
        longitude: 0,
        country: "CN".to_string(),
        region: "Unknown".to_string(),
    };
    
    // 序列化指令数据
    let mut data = Vec::new();
    data.extend_from_slice(&(node_id.to_bytes()));
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(&(device_type.len() as u32).to_le_bytes());
    data.extend_from_slice(device_type.as_bytes());
    data.extend_from_slice(&(location.country.len() as u32).to_le_bytes());
    data.extend_from_slice(location.country.as_bytes());
    data.extend_from_slice(&(location.region.len() as u32).to_le_bytes());
    data.extend_from_slice(location.region.as_bytes());
    data.extend_from_slice(&location.latitude.to_le_bytes());
    data.extend_from_slice(&location.longitude.to_le_bytes());
    
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*node_account, false),
            AccountMeta::new(*global_state, false),
            AccountMeta::new(*owner, true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
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
) -> Result<solana_sdk::instruction::Instruction> {
    use solana_sdk::instruction::{Instruction, AccountMeta};
    
    // 构建模型信息
    let model_info = super::types::ModelInfo {
        model_id: "default_model".to_string(),
        version: "1.0".to_string(),
        parameters_hash: "hash".to_string(),
        size_mb: 100,
    };
    
    // 序列化指令数据
    let mut data = Vec::new();
    
    // 贡献 ID
    data.extend_from_slice(&(contribution_id.len() as u32).to_le_bytes());
    data.extend_from_slice(contribution_id.as_bytes());
    
    // 任务 ID
    data.extend_from_slice(&(task_id.len() as u32).to_le_bytes());
    data.extend_from_slice(task_id.as_bytes());
    
    // 任务类型 (Training = 0)
    data.push(0);
    
    // 模型信息
    data.extend_from_slice(&(model_info.model_id.len() as u32).to_le_bytes());
    data.extend_from_slice(model_info.model_id.as_bytes());
    data.extend_from_slice(&(model_info.version.len() as u32).to_le_bytes());
    data.extend_from_slice(model_info.version.as_bytes());
    data.extend_from_slice(&(model_info.parameters_hash.len() as u32).to_le_bytes());
    data.extend_from_slice(model_info.parameters_hash.as_bytes());
    data.extend_from_slice(&model_info.size_mb.to_le_bytes());
    
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
    
    // 质量评分 (默认 0.8)
    data.extend_from_slice(&0.8f32.to_le_bytes());
    
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*contribution_account, false),
            AccountMeta::new(*node_account, false),
            AccountMeta::new(*global_state, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
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
) -> Result<solana_sdk::instruction::Instruction> {
    use solana_sdk::instruction::{Instruction, AccountMeta};
    
    // 简化的指令数据（实际应该包含更多参数）
    let data = vec![2]; // distribute_rewards 指令 ID
    
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*reward_account, false),
            AccountMeta::new(*node_account, false),
            AccountMeta::new(*global_state, false),
            AccountMeta::new(*treasury, false),
            AccountMeta::new(*node_wallet, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data,
    })
}

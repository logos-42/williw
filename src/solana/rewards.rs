//! 收益分配管理模块
//!
//! 本模块提供节点收益的分配和管理功能。

use anyhow::{anyhow, Result};
use chrono::Utc;
use uuid::Uuid;

use super::compute::ComputeCalculator;
use super::types::*;

/// 收益分配管理器
pub struct RewardManager {
    /// 基础每次计算的奖励（lamports）
    base_reward_per_compute_lamports: u64,
    /// 最小结算阈值（lamports）
    min_settlement_threshold_lamports: u64,
}

impl RewardManager {
    /// 创建新的收益管理器
    pub fn new(
        base_reward_per_compute_lamports: u64,
        min_settlement_threshold_lamports: u64,
    ) -> Self {
        Self {
            base_reward_per_compute_lamports,
            min_settlement_threshold_lamports,
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self {
            base_reward_per_compute_lamports: 1_000_000, // 0.001 SOL
            min_settlement_threshold_lamports: 10_000_000, // 0.01 SOL
        }
    }

    /// 创建收益分配记录
    pub fn create_reward_distribution(
        &self,
        node_id: String,
        task_id: String,
        contribution: &ComputeContribution,
        transaction_signature: String,
    ) -> RewardDistribution {
        let amount = ComputeCalculator::calculate_reward(
            contribution,
            self.base_reward_per_compute_lamports,
        );

        RewardDistribution {
            id: Uuid::new_v4().to_string(),
            node_id,
            task_id,
            amount_lamports: amount,
            distributed_at: Utc::now().timestamp(),
            transaction_signature,
            status: RewardStatus::Pending,
        }
    }

    /// 创建批量收益分配记录
    pub fn create_batch_reward_distributions(
        &self,
        contributions: &[ComputeContribution],
        transaction_signatures: Vec<String>,
    ) -> Vec<RewardDistribution> {
        contributions
            .iter()
            .zip(transaction_signatures.iter())
            .map(|(contribution, signature)| {
                self.create_reward_distribution(
                    contribution.node_id.clone(),
                    contribution.task_id.clone(),
                    contribution,
                    signature.clone(),
                )
            })
            .collect()
    }

    /// 计算节点的待结算收益
    pub fn calculate_pending_rewards(
        &self,
        contributions: &[ComputeContribution],
    ) -> u64 {
        contributions
            .iter()
            .map(|c| ComputeCalculator::calculate_reward(c, self.base_reward_per_compute_lamports))
            .sum()
    }

    /// 检查是否达到结算阈值
    pub fn meets_settlement_threshold(&self, pending_rewards: u64) -> bool {
        pending_rewards >= self.min_settlement_threshold_lamports
    }

    /// 将 lamports 转换为 SOL
    pub fn lamports_to_sol(&self, lamports: u64) -> f64 {
        lamports as f64 / 1_000_000_000.0
    }

    /// 将 SOL 转换为 lamports
    pub fn sol_to_lamports(&self, sol: f64) -> u64 {
        (sol * 1_000_000_000.0) as u64
    }

    /// 获取基础奖励
    pub fn get_base_reward(&self) -> u64 {
        self.base_reward_per_compute_lamports
    }

    /// 设置基础奖励
    pub fn set_base_reward(&mut self, amount: u64) {
        self.base_reward_per_compute_lamports = amount;
    }

    /// 获取最小结算阈值
    pub fn get_min_settlement_threshold(&self) -> u64 {
        self.min_settlement_threshold_lamports
    }

    /// 设置最小结算阈值
    pub fn set_min_settlement_threshold(&mut self, amount: u64) {
        self.min_settlement_threshold_lamports = amount;
    }
}

/// 收益结算器
pub struct RewardSettler {
    /// 收益管理器
    reward_manager: RewardManager,
}

impl RewardSettler {
    /// 创建新的收益结算器
    pub fn new(reward_manager: RewardManager) -> Self {
        Self { reward_manager }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(RewardManager::with_defaults())
    }

    /// 生成结算计划
    pub fn generate_settlement_plan(
        &self,
        node_wallet_balances: &[NodeWalletBalance],
    ) -> SettlementPlan {
        let mut total_to_settle = 0u64;
        let mut eligible_nodes = Vec::new();

        for balance in node_wallet_balances {
            if balance.pending_rewards_lamports > 0 {
                let should_settle = self.reward_manager.meets_settlement_threshold(
                    balance.pending_rewards_lamports,
                );

                if should_settle {
                    total_to_settle += balance.pending_rewards_lamports;
                    eligible_nodes.push(SettlementNode {
                        node_id: balance.node_id.clone(),
                        wallet_address: balance.wallet_address.clone(),
                        pending_rewards: balance.pending_rewards_lamports,
                    });
                }
            }
        }

        SettlementPlan {
            total_amount_lamports: total_to_settle,
            nodes_to_settle: eligible_nodes,
            estimated_gas_fee_lamports: total_to_settle / 100, // 1% gas fee
            created_at: Utc::now().timestamp(),
        }
    }

    /// 计算实际结算金额（扣除 gas 费）
    pub fn calculate_net_rewards(
        &self,
        settlement_plan: &SettlementPlan,
    ) -> Vec<NetReward> {
        let total_gas = settlement_plan.estimated_gas_fee_lamports;
        let total_rewards = settlement_plan.total_amount_lamports;

        // 按比例分配 gas 费
        settlement_plan
            .nodes_to_settle
            .iter()
            .map(|node| {
                let gas_share = (node.pending_rewards as f64 / total_rewards as f64)
                    * total_gas as f64;
                let net_reward = node.pending_rewards.saturating_sub(gas_share as u64);

                NetReward {
                    node_id: node.node_id.clone(),
                    wallet_address: node.wallet_address.clone(),
                    gross_rewards: node.pending_rewards,
                    gas_fee: gas_share as u64,
                    net_rewards: net_reward,
                }
            })
            .collect()
    }
}

/// 结算计划
#[derive(Debug, Clone)]
pub struct SettlementPlan {
    /// 总结算金额（lamports）
    pub total_amount_lamports: u64,
    /// 需要结算的节点列表
    pub nodes_to_settle: Vec<SettlementNode>,
    /// 预估 gas 费用（lamports）
    pub estimated_gas_fee_lamports: u64,
    /// 计划创建时间
    pub created_at: i64,
}

/// 需要结算的节点信息
#[derive(Debug, Clone)]
pub struct SettlementNode {
    /// 节点 ID
    pub node_id: String,
    /// 钱包地址
    pub wallet_address: String,
    /// 待结算金额（lamports）
    pub pending_rewards: u64,
}

/// 实际结算金额
#[derive(Debug, Clone)]
pub struct NetReward {
    /// 节点 ID
    pub node_id: String,
    /// 钱包地址
    pub wallet_address: String,
    /// 总收益（lamports）
    pub gross_rewards: u64,
    /// gas 费用（lamports）
    pub gas_fee: u64,
    /// 净收益（lamports）
    pub net_rewards: u64,
}

impl Default for RewardSettler {
    fn default() -> Self {
        Self::with_defaults()
    }
}

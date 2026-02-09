//! Solana 账户和 PDA 管理
//!
//! 本模块提供与 Solana 账户交互的工具函数。

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

use super::types::*;

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
    Pubkey::find_program_address(
        &[b"reward", node_id.as_ref(), &reward_count.to_le_bytes()],
        program_id
    )
}

/// 查找多签账户 PDA
pub fn find_multisig_account_pda(creator: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"multisig", creator.as_ref()], program_id)
}

/// 获取账户余额
pub fn get_account_balance(client: &RpcClient, pubkey: &Pubkey) -> Result<u64> {
    client.get_balance(pubkey).map_err(|e| anyhow!("Failed to get balance: {}", e))
}

/// 获取账户信息
pub fn get_account_info(client: &RpcClient, pubkey: &Pubkey) -> Result<Option<solana_account_decoder::parse_account_data::ParsedAccount>> {
    let account = client.get_account(pubkey).map_err(|e| anyhow!("Failed to get account: {}", e))?;

    if account.owner == solana_sdk::system_program::id() {
        // 系统账户
        Ok(None)
    } else {
        // 尝试解析程序账户
        match solana_account_decoder::parse_account_data::parse_account_data(
            &account.owner,
            &account.data,
            None,
        ) {
            Ok(parsed) => Ok(Some(parsed)),
            Err(_) => Ok(None),
        }
    }
}

/// 发送并确认交易
pub fn send_and_confirm_transaction(
    client: &RpcClient,
    transaction: &Transaction,
    signers: &[&Keypair],
) -> Result<String> {
    let recent_blockhash = client.get_latest_blockhash()
        .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

    let mut tx = transaction.clone();
    tx.sign(signers, recent_blockhash);

    let signature = client.send_and_confirm_transaction(&tx)
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

    Ok(signature.to_string())
}

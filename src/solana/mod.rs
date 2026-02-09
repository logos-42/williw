//! Solana 区块链集成模块
//!
//! 本模块提供与 Solana 区块链的交互功能，包括：
//! 1. 节点算力贡献记录
//! 2. 收益分配管理
//! 3. 智能合约交互
//! 4. 交易签名和广播

use anyhow::{anyhow, Result};
use std::sync::Arc;
use parking_lot::RwLock;

// 子模块
pub mod client;
pub mod types;
pub mod compute;
pub mod rewards;
pub mod accounts;
pub mod instruction;

// 重新导出常用类型
pub use client::*;
pub use types::*;
pub use compute::*;
pub use rewards::*;
pub use accounts::*;
pub use instruction::*;

/// Solana 配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SolanaConfig {
    /// RPC 端点 URL
    pub rpc_url: String,
    /// WebSocket 端点 URL（可选）
    pub ws_url: Option<String>,
    /// 程序 ID（智能合约地址）
    pub program_id: String,
    /// 支付者的私钥（base58 编码）
    pub payer_keypair_base58: Option<String>,
    /// 网络环境
    pub network: SolanaNetwork,
}

/// Solana 网络环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SolanaNetwork {
    /// 主网
    Mainnet,
    /// 开发网
    Devnet,
    /// 测试网
    Testnet,
    /// 本地网络
    Localnet,
}

impl SolanaConfig {
    /// 创建默认配置（使用 devnet）
    pub fn devnet(program_id: &str) -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            ws_url: Some("wss://api.devnet.solana.com".to_string()),
            program_id: program_id.to_string(),
            payer_keypair_base58: None,
            network: SolanaNetwork::Devnet,
        }
    }

    /// 创建本地测试配置
    pub fn localnet(program_id: &str) -> Self {
        Self {
            rpc_url: "http://localhost:8899".to_string(),
            ws_url: Some("ws://localhost:8900".to_string()),
            program_id: program_id.to_string(),
            payer_keypair_base58: None,
            network: SolanaNetwork::Localnet,
        }
    }

    /// 创建主网配置
    pub fn mainnet(program_id: &str) -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: Some("wss://api.mainnet-beta.solana.com".to_string()),
            program_id: program_id.to_string(),
            payer_keypair_base58: None,
            network: SolanaNetwork::Mainnet,
        }
    }
}

impl Default for SolanaConfig {
    fn default() -> Self {
        Self::devnet("11111111111111111111111111111111")
    }
}

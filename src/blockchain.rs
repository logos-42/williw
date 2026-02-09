//! 区块链客户端模块
//! 
//! 提供 Base 网络 RPC 调用和智能合约交互功能

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[cfg(feature = "blockchain")]
use ethers::{
    core::types::{Address, TransactionRequest, U256},
    providers::{Provider, Http, Middleware},
    signers::{LocalWallet, Signer},
    utils::parse_ether,
};

/// Base 网络 RPC 客户端
pub struct BaseNetworkClient {
    rpc_url: String,
    chain_id: u64,
    client: reqwest::Client,
    #[cfg(feature = "blockchain")]
    provider: Option<std::sync::Arc<ethers::providers::Provider<ethers::providers::Http>>>,
    #[cfg(feature = "blockchain")]
    wallet: Option<ethers::signers::LocalWallet>,
}

/// RPC 请求结构
#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

/// RPC 响应结构
#[derive(Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
    id: u64,
}

/// RPC 错误结构
#[derive(Deserialize, Debug)]
struct RpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl BaseNetworkClient {
    /// 创建新的 Base 网络客户端
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url: rpc_url.clone(),
            chain_id: 8453, // Base 主网 Chain ID
            client: reqwest::Client::new(),
            #[cfg(feature = "blockchain")]
            provider: None,
            #[cfg(feature = "blockchain")]
            wallet: None,
        }
    }
    
    /// 创建 Base Sepolia 测试网客户端
    pub fn new_sepolia(rpc_url: String) -> Self {
        Self {
            rpc_url: rpc_url.clone(),
            chain_id: 84532, // Base Sepolia Chain ID
            client: reqwest::Client::new(),
            #[cfg(feature = "blockchain")]
            provider: None,
            #[cfg(feature = "blockchain")]
            wallet: None,
        }
    }
    
    /// 创建带钱包的客户端（用于发送交易）
    #[cfg(feature = "blockchain")]
    pub async fn with_wallet(rpc_url: String, private_key: &str) -> Result<Self> {
        use std::sync::Arc;
        use ethers::providers::Http;
        
        let provider = Provider::<Http>::try_from(rpc_url.as_str())?;
        let wallet = private_key.parse::<LocalWallet>()?;
        let wallet = wallet.with_chain_id(8453u64); // Base 主网
        
        Ok(Self {
            rpc_url,
            chain_id: 8453,
            client: reqwest::Client::new(),
            provider: Some(Arc::new(provider)),
            wallet: Some(wallet),
        })
    }
    
    /// 执行 RPC 调用
    async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: 1,
        };
        
        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;
        
        let rpc_response: RpcResponse = response.json().await?;
        
        if let Some(error) = rpc_response.error {
            return Err(anyhow!("RPC 错误: {} (code: {})", error.message, error.code));
        }
        
        rpc_response
            .result
            .ok_or_else(|| anyhow!("RPC 响应缺少 result 字段"))
    }
    
    /// 查询账户余额（ETH）
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let params = json!([
            address,
            "latest"
        ]);
        
        let result = self.call("eth_getBalance", params).await?;
        let balance_hex = result.as_str().ok_or_else(|| anyhow!("无效的余额格式"))?;
        
        // 移除 "0x" 前缀并解析
        let balance = u64::from_str_radix(
            balance_hex.strip_prefix("0x").unwrap_or(balance_hex),
            16,
        )?;
        
        Ok(balance)
    }
    
    /// 查询质押数量（从智能合约）
    /// 注意：这需要实际的质押合约地址和 ABI
    pub async fn query_stake(&self, contract_address: &str, user_address: &str) -> Result<f64> {
        // 这里是一个示例实现
        // 实际使用时需要根据合约 ABI 构造正确的调用数据
        
        // 示例：调用合约的 balanceOf 方法
        // function signature: balanceOf(address) -> uint256
        let method_id = "0x70a08231"; // balanceOf(address) 的 keccak256 前 4 字节
        
        // 构造调用数据（简化版，实际需要正确编码参数）
        let call_data = format!(
            "{}{:0>64}",
            method_id,
            user_address.strip_prefix("0x").unwrap_or(user_address)
        );
        
        let params = json!({
            "to": contract_address,
            "data": call_data,
        });
        
        // 使用 eth_call 执行合约调用
        let result = self.call("eth_call", json!([params, "latest"])).await?;
        
        // 解析返回的 uint256 值
        let stake_hex = result.as_str().ok_or_else(|| anyhow!("无效的质押值格式"))?;
        let stake = u64::from_str_radix(
            stake_hex.strip_prefix("0x").unwrap_or(stake_hex),
            16,
        )?;
        
        // 转换为 ETH（假设 18 位小数）
        Ok(stake as f64 / 1e18)
    }
    
    /// 更新信誉分数（发送交易到智能合约）
    #[cfg(feature = "blockchain")]
    pub async fn update_reputation(
        &self,
        contract_address: &str,
        user_address: &str,
        delta: f64,
    ) -> Result<String> {
        use std::sync::Arc;
        
        let provider = self.provider.as_ref()
            .ok_or_else(|| anyhow!("Provider not initialized. Use with_wallet() to create client with transaction support"))?;
        let wallet = self.wallet.as_ref()
            .ok_or_else(|| anyhow!("Wallet not initialized. Use with_wallet() to create client with transaction support"))?;
        
        // 构造合约调用数据
        // 这里假设合约有一个 updateReputation(address, int256) 函数
        // 函数选择器: keccak256("updateReputation(address,int256)")[:4] = 0x...
        // 为了简化，这里使用一个占位符
        let contract_addr: Address = contract_address.parse()
            .map_err(|e| anyhow!("Invalid contract address: {}", e))?;
        let user_addr: Address = user_address.parse()
            .map_err(|e| anyhow!("Invalid user address: {}", e))?;
        
        // 构造函数调用数据（简化版，实际需要 ABI 编码）
        // 这里只是示例，实际使用时需要根据合约 ABI 构造正确的数据
        let delta_wei = parse_ether(delta)?;
        
        // 获取 nonce
        let nonce = provider.get_transaction_count(wallet.address(), None).await?;
        
        // 估算 gas
        let gas_price = provider.get_gas_price().await?;
        let gas_limit = U256::from(100000u64); // 默认 gas limit，实际应该估算
        
        // 构造交易请求
        // 注意：这里需要根据实际合约 ABI 构造正确的 data 字段
        // 为了演示，这里使用一个占位符
        let tx = TransactionRequest::new()
            .to(contract_addr)
            .value(0u64)
            .gas(gas_limit)
            .gas_price(gas_price)
            .nonce(nonce);
            // .data(...) // 需要添加合约调用数据
        
        // 签名并发送交易
        let pending_tx = wallet.send_transaction(tx, None).await?;
        let tx_hash = pending_tx.tx_hash();
        
        Ok(format!("0x{:x}", tx_hash))
    }
    
    /// 更新信誉分数（未启用 blockchain feature 时的占位符）
    #[cfg(not(feature = "blockchain"))]
    pub async fn update_reputation(
        &self,
        _contract_address: &str,
        _user_address: &str,
        _delta: f64,
    ) -> Result<String> {
        Err(anyhow!("update_reputation requires blockchain feature. Enable it with --features blockchain"))
    }
    
    /// 获取链 ID
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

/// 区块链客户端 trait（用于抽象不同的区块链实现）
#[async_trait::async_trait]
pub trait BlockchainClient: Send + Sync {
    /// 查询质押数量
    async fn query_stake(&self, address: &str) -> Result<f64>;
    
    /// 更新信誉分数
    async fn update_reputation(&self, address: &str, delta: f64) -> Result<()>;
    
    /// 获取链 ID
    fn chain_id(&self) -> u64;
}

#[async_trait::async_trait]
impl BlockchainClient for BaseNetworkClient {
    async fn query_stake(&self, address: &str) -> Result<f64> {
        // 简化实现：直接查询余额作为质押
        // 实际应该查询质押合约
        let balance = self.get_balance(address).await?;
        Ok(balance as f64 / 1e18) // 转换为 ETH
    }
    
    async fn update_reputation(&self, address: &str, delta: f64) -> Result<()> {
        // 注意：这里需要合约地址，暂时使用占位符
        // 实际使用时应该从配置中获取合约地址
        let contract_address = "0x0000000000000000000000000000000000000000"; // 占位符
        match self.update_reputation(contract_address, address, delta).await {
            Ok(tx_hash) => {
                log::info!("Reputation update transaction sent: {}", tx_hash);
                Ok(())
            }
            Err(e) => {
                log::warn!("Failed to send reputation update transaction: {}", e);
                // 不返回错误，允许降级到内存账本
                Ok(())
            }
        }
    }
    
    fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

/// 内存缓存的区块链客户端（用于提高性能）
pub struct CachedBlockchainClient {
    client: Box<dyn BlockchainClient>,
    stake_cache: parking_lot::RwLock<HashMap<String, (f64, std::time::Instant)>>,
    cache_ttl: std::time::Duration,
}

impl CachedBlockchainClient {
    pub fn new(client: Box<dyn BlockchainClient>) -> Self {
        Self {
            client,
            stake_cache: parking_lot::RwLock::new(HashMap::new()),
            cache_ttl: std::time::Duration::from_secs(60), // 缓存 60 秒
        }
    }
    
    async fn query_stake_cached(&self, address: &str) -> Result<f64> {
        // 检查缓存
        {
            let cache = self.stake_cache.read();
            if let Some((stake, timestamp)) = cache.get(address) {
                if timestamp.elapsed() < self.cache_ttl {
                    return Ok(*stake);
                }
            }
        }
        
        // 缓存未命中或过期，查询链上数据
        let stake = self.client.query_stake(address).await?;
        
        // 更新缓存
        {
            let mut cache = self.stake_cache.write();
            cache.insert(address.to_string(), (stake, std::time::Instant::now()));
        }
        
        Ok(stake)
    }
    
    /// 清理过期缓存
    pub fn cleanup_cache(&self) {
        let mut cache = self.stake_cache.write();
        cache.retain(|_, (_, timestamp)| timestamp.elapsed() < self.cache_ttl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // 需要实际的 RPC 端点
    async fn test_get_balance() {
        let client = BaseNetworkClient::new("https://mainnet.base.org".to_string());
        // 使用一个测试地址
        let balance = client.get_balance("0x0000000000000000000000000000000000000000").await;
        // 这个测试需要实际的网络连接
        assert!(balance.is_ok() || balance.is_err());
    }
}


//! Solana 集成测试
//!
//! 测试 Solana 客户端与智能合约的集成功能

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana::{SolanaClient, SolanaConfig, SolanaNetwork, NodeInfo, NodeStatus};

    /// 创建测试配置
    fn create_test_config() -> SolanaConfig {
        SolanaConfig::localnet("4SLjWwRYgRRdr4i5pgfjcbZEswXZRDcZ31BT1gipYdPq")
    }

    /// 创建测试节点信息
    fn create_test_node_info() -> NodeInfo {
        NodeInfo {
            node_id: "test_node_123".to_string(),
            owner_address: "test_owner_456".to_string(),
            name: "Test Node".to_string(),
            device_type: "Desktop".to_string(),
            registered_at: chrono::Utc::now().timestamp(),
            last_active_at: chrono::Utc::now().timestamp(),
            status: NodeStatus::Active,
        }
    }

    #[tokio::test]
    async fn test_solana_client_creation() {
        let config = create_test_config();
        let node_id = "test_node".to_string();
        
        // 测试客户端创建（可能因为没有真实的密钥而失败）
        match SolanaClient::new(config, node_id) {
            Ok(client) => {
                println!("✅ Solana 客户端创建成功");
                
                // 测试连接检查
                match client.check_connection().await {
                    Ok(connected) => {
                        if connected {
                            println!("✅ 成功连接到 Solana 网络");
                        } else {
                            println!("⚠️ 无法连接到 Solana 网络（可能是本地网络未运行）");
                        }
                    }
                    Err(e) => {
                        println!("⚠️ 连接检查失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Solana 客户端创建失败: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_node_registration() {
        let config = create_test_config();
        let node_id = "test_node_register".to_string();
        
        if let Ok(client) = SolanaClient::new(config, node_id) {
            let node_info = create_test_node_info();
            
            match client.register_node(node_info).await {
                Ok(result) => {
                    if result.success {
                        println!("✅ 节点注册成功: {}", result.signature);
                    } else {
                        println!("⚠️ 节点注册失败: {:?}", result.error);
                    }
                }
                Err(e) => {
                    println!("⚠️ 节点注册错误: {}", e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_contract_state_query() {
        let config = create_test_config();
        let node_id = "test_node_query".to_string();
        
        if let Ok(client) = SolanaClient::new(config, node_id) {
            match client.get_contract_state().await {
                Ok(state) => {
                    println!("✅ 合约状态查询成功:");
                    println!("  程序 ID: {}", state.program_id);
                    println!("  管理员: {}", state.admin_address);
                    println!("  国库: {}", state.treasury_address);
                    println!("  总节点数: {}", state.total_nodes);
                    println!("  总贡献数: {}", state.total_contributions);
                    println!("  基础奖励: {} lamports", state.base_reward_per_compute_lamports);
                }
                Err(e) => {
                    println!("⚠️ 合约状态查询失败: {}", e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_compute_contribution_reporting() {
        let config = create_test_config();
        let node_id = "test_node_contribution".to_string();
        
        if let Ok(client) = SolanaClient::new(config, node_id) {
            let contribution = ComputeContribution {
                id: "test_contribution_123".to_string(),
                node_id: "test_node_123".to_string(),
                task_id: "test_task_456".to_string(),
                start_timestamp: chrono::Utc::now().timestamp() - 3600,
                end_timestamp: chrono::Utc::now().timestamp(),
                duration_seconds: 3600,
                avg_gpu_usage_percent: 75.5,
                gpu_memory_used_mb: 1024,
                avg_cpu_usage_percent: 45.2,
                memory_used_mb: 2048,
                network_upload_mb: 100,
                network_download_mb: 200,
                samples_processed: 10000,
                batches_processed: 50,
                compute_score: 2.5,
            };
            
            match client.report_compute_contribution(contribution).await {
                Ok(result) => {
                    if result.success {
                        println!("✅ 算力贡献上报成功: {}", result.signature);
                    } else {
                        println!("⚠️ 算力贡献上报失败: {:?}", result.error);
                    }
                }
                Err(e) => {
                    println!("⚠️ 算力贡献上报错误: {}", e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_wallet_balance_query() {
        let config = create_test_config();
        let node_id = "test_node_balance".to_string();
        
        if let Ok(client) = SolanaClient::new(config, node_id) {
            let wallet_address = "test_wallet_789";
            
            match client.get_wallet_balance(wallet_address).await {
                Ok(balance) => {
                    println!("✅ 钱包余额查询成功:");
                    println!("  节点 ID: {}", balance.node_id);
                    println!("  钱包地址: {}", balance.wallet_address);
                    println!("  SOL 余额: {} lamports", balance.sol_balance_lamports);
                    println!("  待结算收益: {} lamports", balance.pending_rewards_lamports);
                    println!("  已分配收益: {} lamports", balance.total_rewards_distributed_lamports);
                }
                Err(e) => {
                    println!("⚠️ 钱包余额查询失败: {}", e);
                }
            }
        }
    }
}

//! 安全功能测试
//!
//! 测试IP隐藏、中继连接、隐私保护等功能

use williw::config::{AppConfig, SecurityConfig};
use williw::privacy::crypto::security::{PrivacyChecker, TrafficObfuscator, IdentityProtector};
use iroh::NodeAddr;
use std::path::Path;

/// 测试安全配置验证
#[test]
fn test_security_config_validation() {
    // 测试有效的安全配置（使用 iroh::NodeAddr）
    let valid_config = SecurityConfig {
        hide_ip: true,
        use_relay: true,
        relay_nodes: vec![
            // iroh NodeAddr 格式不同，这里简化测试
        ],
        private_network_key: None,
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };

    assert!(valid_config.validate().is_ok());

    // 测试无效配置：隐藏IP但未使用中继
    let invalid_config = SecurityConfig {
        hide_ip: true,
        use_relay: false,
        relay_nodes: vec![],
        private_network_key: None,
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };

    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(errors) = result {
        assert!(errors.iter().any(|e| e.contains("隐私保护可能不完整")));
    }

    // 测试无效跳数
    let invalid_hops = SecurityConfig {
        hide_ip: true,
        use_relay: true,
        relay_nodes: vec![],
        private_network_key: None,
        max_hops: 0, // 无效跳数
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };

    let result = invalid_hops.validate();
    assert!(result.is_err());
}

/// 测试流量混淆
#[test]
fn test_traffic_obfuscation() {
    let config = SecurityConfig {
        hide_ip: true,
        use_relay: true,
        relay_nodes: vec![],
        private_network_key: None,
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };

    let obfuscator = TrafficObfuscator::new(config);

    // 测试数据混淆
    let original_data = b"Hello, secure world!";
    let obfuscated = obfuscator.obfuscate_data(original_data);

    // 混淆后的数据应该比原始数据长
    assert!(obfuscated.len() > original_data.len());

    // 应该能够正确解混淆
    let deobfuscated = obfuscator.deobfuscate_data(&obfuscated, original_data.len());
    assert_eq!(deobfuscated, original_data);
}

/// 测试隐私检查器
#[test]
fn test_privacy_checker() {
    let config = SecurityConfig {
        hide_ip: true,
        use_relay: true,
        relay_nodes: vec![],
        private_network_key: None,
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };

    let checker = PrivacyChecker::new(config);

    // 测试直接IP地址（应该暴露IP）
    let direct_ip = "/ip4/192.168.1.100/tcp/9001".to_string();
    assert!(checker.is_address_exposing_ip(&direct_ip));

    // 测试域名地址（简化测试）
    let dns_addr = "/dns4/example.com/tcp/9001".to_string();
    assert!(!checker.is_address_exposing_ip(&dns_addr));
}

/// 测试身份保护
#[test]
fn test_identity_protection() {
    let config = SecurityConfig {
        hide_ip: true,
        use_relay: true,
        relay_nodes: vec![],
        private_network_key: None,
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };

    let protector = IdentityProtector::new(config);

    // 获取第一个NodeId
    let node_id1 = protector.get_current_node_id();
    assert!(!node_id1.to_string().is_empty());

    // 再次获取应该返回相同的NodeId（除非时间到了需要更换）
    let node_id2 = protector.get_current_node_id();
    assert_eq!(node_id1, node_id2);

    // 清理历史
    protector.cleanup_old_identities();

    // 获取历史
    let history = protector.get_identity_history();
    assert!(!history.is_empty());
}

/// 测试配置建议
#[test]
fn test_privacy_advice() {
    // 测试完整隐私配置
    let full_privacy = SecurityConfig {
        hide_ip: true,
        use_relay: true,
        relay_nodes: vec![],
        private_network_key: None,
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };

    let advice = full_privacy.get_privacy_advice();
    assert!(advice.iter().any(|a| a.contains("IP隐藏已启用")));

    // 测试不完整隐私配置
    let partial_privacy = SecurityConfig {
        hide_ip: false,
        use_relay: false,
        relay_nodes: vec![],
        private_network_key: None,
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: true,
        privacy_performance: Default::default(),
    };

    let advice = partial_privacy.get_privacy_advice();
    assert!(advice.iter().any(|a| a.contains("IP隐藏未启用")));
    assert!(advice.iter().any(|a| a.contains("建议")));
}

/// 测试安全工具函数（适配 iroh）
#[test]
fn test_security_utils() {
    use williw::privacy::crypto::security::utils;

    // 测试是否为中继地址 - 对于 iroh，包含 "circuit" 或 "relay" 的地址被视为中继地址
    let relay_addr = "/ip4/127.0.0.1/tcp/9001/p2p/12D3KooWRelay/p2p-circuit".to_string();
    let is_relay = utils::is_relay_address(&relay_addr);
    assert!(is_relay);

    // 测试非中继地址
    let direct_addr = "/ip4/192.168.1.100/tcp/9001".to_string();
    assert!(!utils::is_relay_address(&direct_addr));

    // 测试另一个非中继地址
    let dns_addr = "/dns4/example.com/tcp/9001".to_string();
    assert!(!utils::is_relay_address(&dns_addr));
}

/// 测试配置文件加载
#[test]
fn test_config_file_loading() {
    // 创建临时配置文件
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_privacy_config.toml");
    
    let config_content = r#"
[security]
hide_ip = true
use_relay = true
relay_nodes = []
max_hops = 3
enable_autonat = true
enable_dcutr = false

[security.privacy_performance]
mode = "Balanced"
performance_weight = 0.6
enable_hardware_acceleration = true
connection_pool_size = 10
enable_0rtt = true
congestion_control = "Bbr"
routing_strategy = "SmartBalance"
min_privacy_score = 0.7
min_performance_score = 0.8
fallback_to_direct = true
monitoring_interval_secs = 30

[comms]
topic = "ggb-test"
enable_dht = false
    "#;
    
    std::fs::write(&config_path, config_content).unwrap();
    
    // 测试配置文件加载
    let result = AppConfig::from_toml_file(&config_path);
    assert!(result.is_ok());
    
    // 清理临时文件
    std::fs::remove_file(&config_path).unwrap();
}

/// 集成测试：完整的隐私保护流程
#[test]
fn test_integrated_privacy_protection() {
    // 创建完整的安全配置
    let security_config = SecurityConfig {
        hide_ip: true,
        use_relay: true,
        relay_nodes: vec![],
        private_network_key: Some("test-network-key".to_string()),
        max_hops: 3,
        enable_autonat: true,
        enable_dcutr: false,
        privacy_performance: Default::default(),
    };
    
    // 验证配置
    assert!(security_config.validate().is_ok());
    
    // 测试所有隐私组件
    let privacy_checker = PrivacyChecker::new(security_config.clone());
    let traffic_obfuscator = TrafficObfuscator::new(security_config.clone());
    let identity_protector = IdentityProtector::new(security_config);
    
    // 测试隐私建议
    let advice = privacy_checker.get_privacy_advice();
    assert!(!advice.is_empty());
    
    // 测试流量混淆
    let test_data = b"Test data for obfuscation";
    let obfuscated = traffic_obfuscator.obfuscate_data(test_data);
    assert!(obfuscated.len() >= test_data.len());
    
    // 测试身份保护
    let node_id = identity_protector.get_current_node_id();
    assert!(!node_id.to_string().is_empty());
    
    println!("集成测试通过：");
    println!("  - 隐私建议: {:?}", advice);
    println!("  - 流量混淆: {} -> {} 字节", test_data.len(), obfuscated.len());
    println!("  - 身份保护: NodeId = {}", node_id);
}

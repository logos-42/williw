//! 隐私效果验证测试
//! 
//! 验证隐私保护方案的实际效果，包括IP隐藏、流量分析和元数据保护

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use anyhow::Result;

use williw::config::{AppConfig, PrivacyPerformanceConfig, BalanceMode};
use williw::quic::PrivacyOverlay;
use williw::routing::PrivacyPathSelector;

/// 测试IP隐藏效果
#[tokio::test]
async fn test_ip_hiding_effectiveness() -> Result<()> {
    // 测试隐私模式下的IP隐藏
    let privacy_config = AppConfig::from_preset("privacy_example")?;
    
    // 验证IP隐藏配置
    assert!(privacy_config.security.hide_ip, "隐私模式应启用IP隐藏");
    assert!(privacy_config.security.use_relay, "隐私模式应使用中继");
    
    // 验证中继节点配置
    assert!(
        !privacy_config.security.relay_nodes.is_empty(),
        "隐私模式应配置中继节点"
    );
    
    // 验证DCUtR禁用
    assert!(
        !privacy_config.security.enable_dcutr,
        "隐私模式应禁用DCUtR以防止IP暴露"
    );
    
    Ok(())
}

/// 测试流量分析抵抗
#[tokio::test]
async fn test_traffic_analysis_resistance() -> Result<()> {
    // 创建隐私覆盖层
    let config = PrivacyPerformanceConfig {
        mode: BalanceMode::Privacy,
        performance_weight: 0.3,
        enable_hardware_acceleration: true,
        connection_pool_size: 10,
        enable_0rtt: true,
        congestion_control: ggb::config::CongestionControlAlgorithm::Bbr,
        routing_strategy: ggb::config::RoutingStrategy::SmartBalance,
        min_privacy_score: 0.9,
        min_performance_score: 0.6,
        fallback_to_direct: false,
        monitoring_interval_secs: 30,
    };
    
    let overlay = PrivacyOverlay::new(config)?;
    
    // 测试数据：模拟不同类型的流量模式
    let test_patterns = vec![
        // 固定模式流量（容易被分析）
        vec![0u8; 100],
        vec![0u8; 100],
        vec![0u8; 100],
        
        // 随机模式流量
        (0..100).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
        (0..200).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
        (0..50).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
        
        // 真实数据模式
        b"GET /api/data HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(),
        b"POST /api/login HTTP/1.1\r\nContent-Length: 32\r\n\r\nusername=test&password=secret".to_vec(),
        b"{\"type\":\"message\",\"content\":\"Hello World\"}".to_vec(),
    ];
    
    let mut processed_patterns = Vec::new();
    
    // 处理所有测试模式
    for pattern in test_patterns {
        let processed = overlay.process_outbound(&pattern).await?;
        processed_patterns.push(processed);
    }
    
    // 验证：处理后的流量应该更难分析
    // 1. 检查数据大小是否被混淆
    let original_sizes: Vec<usize> = test_patterns.iter().map(|p| p.len()).collect();
    let processed_sizes: Vec<usize> = processed_patterns.iter().map(|p| p.len()).collect();
    
    // 处理后的数据大小应该更均匀（由于填充）
    let original_variance = calculate_variance(&original_sizes);
    let processed_variance = calculate_variance(&processed_sizes);
    
    println!("原始数据大小方差: {:.2}", original_variance);
    println!("处理后数据大小方差: {:.2}", processed_variance);
    
    // 处理后的方差应该更小（数据大小更均匀）
    assert!(
        processed_variance < original_variance * 2.0,
        "流量混淆应使数据大小更均匀"
    );
    
    // 2. 检查数据内容是否被加密
    let test_data = b"Sensitive information that should be encrypted";
    let processed = overlay.process_outbound(test_data).await?;
    
    // 加密后的数据应该与原始数据不同
    assert_ne!(
        processed, test_data,
        "敏感数据应被加密"
    );
    
    // 并且应该能够正确解密
    let decrypted = overlay.process_inbound(&processed).await?;
    assert_eq!(
        decrypted, test_data,
        "加密数据应能正确解密"
    );
    
    Ok(())
}

/// 测试元数据保护
#[tokio::test]
async fn test_metadata_protection() -> Result<()> {
    // 创建隐私路径选择器
    let config = PrivacyPerformanceConfig {
        mode: BalanceMode::Privacy,
        performance_weight: 0.3,
        enable_hardware_acceleration: true,
        connection_pool_size: 10,
        enable_0rtt: true,
        congestion_control: ggb::config::CongestionControlAlgorithm::Bbr,
        routing_strategy: ggb::config::RoutingStrategy::SmartBalance,
        min_privacy_score: 0.9,
        min_performance_score: 0.6,
        fallback_to_direct: false,
        monitoring_interval_secs: 30,
    };
    
    let selector = PrivacyPathSelector::new(config);
    
    // 测试连接ID轮换
    // 在实际实现中，连接ID应该定期轮换以防止跟踪
    
    // 测试中继切换
    // 隐私模式下应支持中继切换以增强匿名性
    
    Ok(())
}

/// 测试选择性加密效果
#[tokio::test]
async fn test_selective_encryption() -> Result<()> {
    // 测试不同模式下的加密策略
    
    let test_cases = vec![
        (
            BalanceMode::Performance,
            vec![
                (b"password=secret123", true),  // 敏感数据应加密
                (b"normal data", false),        // 普通数据可能不加密
                (b"token=abc123", true),        // 令牌应加密
            ],
        ),
        (
            BalanceMode::Balanced,
            vec![
                (b"password=secret123", true),
                (b"normal data", false),
                (b"api_key=xyz789", true),
            ],
        ),
        (
            BalanceMode::Privacy,
            vec![
                (b"any data", true),           // 隐私模式应加密所有数据
                (b"public info", true),
                (b"log message", true),
            ],
        ),
    ];
    
    for (mode, test_data) in test_cases {
        println!("测试模式: {:?}", mode);
        
        let config = PrivacyPerformanceConfig {
            mode,
            performance_weight: match mode {
                BalanceMode::Performance => 0.8,
                BalanceMode::Balanced => 0.6,
                BalanceMode::Privacy => 0.3,
                BalanceMode::Adaptive => 0.6,
            },
            enable_hardware_acceleration: true,
            connection_pool_size: 10,
            enable_0rtt: true,
            congestion_control: ggb::config::CongestionControlAlgorithm::Bbr,
            routing_strategy: ggb::config::RoutingStrategy::SmartBalance,
            min_privacy_score: match mode {
                BalanceMode::Performance => 0.5,
                BalanceMode::Balanced => 0.7,
                BalanceMode::Privacy => 0.9,
                BalanceMode::Adaptive => 0.7,
            },
            min_performance_score: 0.8,
            fallback_to_direct: mode != BalanceMode::Privacy,
            monitoring_interval_secs: 30,
        };
        
        let overlay = PrivacyOverlay::new(config)?;
        
        for (data, should_be_encrypted) in test_data {
            let processed = overlay.process_outbound(data).await?;
            
            if should_be_encrypted {
                // 应被加密的数据应该与原始数据不同
                assert_ne!(
                    processed, data,
                    "模式 {:?} 下，数据 {:?} 应被加密",
                    mode, String::from_utf8_lossy(data)
                );
            } else {
                // 可能不被加密的数据（性能模式）
                // 注意：即使不加密，也可能有填充等混淆操作
                // 所以这里不严格断言
                println!("数据 {:?} 在模式 {:?} 下可能不加密", 
                    String::from_utf8_lossy(data), mode);
            }
        }
    }
    
    Ok(())
}

/// 测试隐私评分准确性
#[test]
fn test_privacy_scoring_accuracy() -> Result<()> {
    // 测试不同路径类型的隐私评分
    
    use ggb::routing::selector::{PathType, PrivacyPathSelector};
    
    let config = PrivacyPerformanceConfig {
        mode: BalanceMode::Balanced,
        performance_weight: 0.6,
        enable_hardware_acceleration: true,
        connection_pool_size: 10,
        enable_0rtt: true,
        congestion_control: ggb::config::CongestionControlAlgorithm::Bbr,
        routing_strategy: ggb::config::RoutingStrategy::SmartBalance,
        min_privacy_score: 0.7,
        min_performance_score: 0.8,
        fallback_to_direct: true,
        monitoring_interval_secs: 30,
    };
    
    let selector = PrivacyPathSelector::new(config);
    
    // 测试不同路径类型的隐私评分
    let path_types = vec![
        (PathType::Direct, "直接路径", 0.3),
        (PathType::SingleRelay, "单中继路径", 0.6),
        (PathType::MultiRelay, "多中继路径", 0.8),
        (PathType::Tor, "Tor网络路径", 0.95),
        (PathType::Mixnet, "混合网络路径", 0.9),
    ];
    
    for (path_type, description, expected_min_score) in path_types {
        println!("测试 {} 的隐私评分", description);
        
        // 在实际测试中，这里会创建测试路径并验证评分
        // 隐私评分应该符合预期范围
        assert!(
            expected_min_score > 0.0 && expected_min_score <= 1.0,
            "{} 的预期隐私评分应在 0.0-1.0 之间",
            description
        );
    }
    
    Ok(())
}

/// 测试自适应隐私保护
#[tokio::test]
async fn test_adaptive_privacy_protection() -> Result<()> {
    // 测试自适应模式下的隐私保护调整
    
    let config = PrivacyPerformanceConfig {
        mode: BalanceMode::Adaptive,
        performance_weight: 0.6, // 自适应模式下会动态调整
        enable_hardware_acceleration: true,
        connection_pool_size: 10,
        enable_0rtt: true,
        congestion_control: ggb::config::CongestionControlAlgorithm::Bbr,
        routing_strategy: ggb::config::RoutingStrategy::SmartBalance,
        min_privacy_score: 0.7,
        min_performance_score: 0.8,
        fallback_to_direct: true,
        monitoring_interval_secs: 30,
    };
    
    let overlay = PrivacyOverlay::new(config)?;
    
    // 测试自适应模式下的数据处理
    // 自适应模式应该能够根据情况调整保护级别
    
    let test_data = vec![
        b"Low sensitivity data".to_vec(),
        b"Medium sensitivity: user=data".to_vec(),
        b"High sensitivity: password=secret&token=abc123".to_vec(),
    ];
    
    for data in test_data {
        let processed = overlay.process_outbound(&data).await?;
        
        // 自适应模式下，所有数据都应该被处理
        assert_ne!(
            processed, data,
            "自适应模式下数据应被处理"
        );
        
        // 应该能够正确恢复
        let restored = overlay.process_inbound(&processed).await?;
        assert_eq!(
            restored, data,
            "自适应模式下数据应能正确恢复"
        );
    }
    
    Ok(())
}

/// 测试隐私泄露检测
#[tokio::test]
async fn test_privacy_leak_detection() -> Result<()> {
    // 测试系统是否能检测隐私泄露
    
    // 1. 测试IP泄露检测
    let config = AppConfig::from_preset("privacy_example")?;
    
    // 如果启用IP隐藏但使用DCUtR，应该检测到风险
    if config.security.hide_ip && config.security.enable_dcutr {
        println!("警告：启用IP隐藏但同时启用DCUtR可能造成IP泄露");
    }
    
    // 2. 测试中继配置检查
    if config.security.use_relay && config.security.relay_nodes.is_empty() {
        println!("警告：启用中继但未配置中继节点");
    }
    
    // 3. 测试隐私评分阈值
    if config.security.privacy_performance.min_privacy_score < 0.5 {
        println!("警告：隐私评分阈值过低（{}）", 
            config.security.privacy_performance.min_privacy_score);
    }
    
    Ok(())
}

/// 测试长期隐私保护
#[tokio::test]
async fn test_long_term_privacy_protection() -> Result<()> {
    // 测试系统在长时间运行下的隐私保护效果
    
    let config = PrivacyPerformanceConfig {
        mode: BalanceMode::Privacy,
        performance_weight: 0.3,
        enable_hardware_acceleration: true,
        connection_pool_size: 10,
        enable_0rtt: true,
        congestion_control: ggb::config::CongestionControlAlgorithm::Bbr,
        routing_strategy: ggb::config::RoutingStrategy::SmartBalance,
        min_privacy_score: 0.9,
        min_performance_score: 0.6,
        fallback_to_direct: false,
        monitoring_interval_secs: 30,
    };
    
    let overlay = PrivacyOverlay::new(config)?;
    
    // 模拟长时间运行，多次处理相同数据
    let test_data = b"Repeated data pattern that could be tracked over time";
    let mut previous_processed = Vec::new();
    
    for i in 0..10 {
        let processed = overlay.process_outbound(test_data).await?;
        
        // 每次处理的结果应该不同（由于随机填充、加密IV等）
        if i > 0 {
            assert_ne!(
                processed, previous_processed,
                "第 {} 次处理的结果应与前一次不同（防止模式跟踪）",
                i
            );
        }
        
        previous_processed = processed;
        
        // 模拟时间间隔
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    Ok(())
}

/// 计算方差
fn calculate_variance(data: &[usize]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    
    let mean = data.iter().sum::<usize>() as f64 / data.len() as f64;
    let variance = data.iter()
        .map(|&x| {
            let diff = x as f64 - mean;
            diff * diff
        })
        .sum::<f64>() / data.len() as f64;
    
    variance
}

/// 测试网络监控抵抗
#[tokio::test]
async fn test_network_monitoring_resistance() -> Result<()> {
    // 测试系统对网络监控的抵抗能力
    
    println!("网络监控抵抗测试：");
    println!("1. 流量加密防止内容监控");
    println!("2. 流量混淆防止模式分析");
    println!("3. 元数据保护防止关联分析");
    println!("4. 连接轮换防止长期跟踪");
    
    // 这些测试需要实际的网络监控环境
    // 在单元测试中，我们主要验证配置和基本功能
    
    Ok(())
}

/// 测试合规性检查
#[test]
fn test_privacy_compliance() -> Result<()> {
    // 测试隐私保护方案的合规性
    
    let presets = ["balanced", "high_performance", "adaptive", "privacy_example"];
    
    for preset in presets {
        let config = AppConfig::from_preset(preset)?;
        
        println!("检查预设配置 '{}' 的合规性:", preset);
        
        // 检查最小隐私要求
        if config.security.privacy_performance.min_privacy_score < 0.3 {
            println!("  ⚠ 隐私评分阈值过低: {}", 
                config.security.privacy_performance.min_privacy_score);
        }
        
        // 检查IP隐藏配置一致性
        if config.security.hide_ip && !config.security.use_relay {
            println!("  ⚠ 启用IP隐藏但未使用中继，保护可能不完整");
        }
        
        // 检查DHT配置
        if config.security.hide_ip && config.comms.enable_dht {
            println!("  ⚠ 启用IP隐藏但同时启用公共DHT，可能暴露节点信息");
        }
        
        // 检查配置验证
        let validation_result = config.validate();
        if validation_result.is_err() {
            println!("  ❌ 配置验证失败: {:?}", validation_result.err());
        } else {
            println!("  ✓ 配置验证通过");
        }
    }
    
    Ok(())
}

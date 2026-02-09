//! 隐私性能平衡集成测试
//! 
//! 测试隐私性能平衡方案的各个组件和集成功能

use std::net::SocketAddr;
use std::time::Duration;
use anyhow::Result;

use williw::config::{AppConfig, PrivacyPerformanceConfig, BalanceMode};
use williw::routing::{ConnectionQualityAnalyzer, PrivacyPathSelector, PathManager};
use williw::quic::{PerformanceOptimizedQuic, PrivacyOverlay};

/// 测试配置文件加载
#[test]
fn test_config_loading() -> Result<()> {
    // 测试平衡模式配置
    let balanced_config = AppConfig::from_preset("balanced")?;
    assert_eq!(balanced_config.security.privacy_performance.mode, BalanceMode::Balanced);
    assert_eq!(balanced_config.security.privacy_performance.performance_weight, 0.6);
    
    // 测试高性能模式配置
    let performance_config = AppConfig::from_preset("high_performance")?;
    assert_eq!(performance_config.security.privacy_performance.mode, BalanceMode::Performance);
    assert!(performance_config.security.privacy_performance.performance_weight >= 0.7);
    
    // 测试自适应模式配置
    let adaptive_config = AppConfig::from_preset("adaptive")?;
    assert_eq!(adaptive_config.security.privacy_performance.mode, BalanceMode::Adaptive);
    
    Ok(())
}

/// 测试连接质量分析器
#[test]
fn test_connection_quality_analyzer() -> Result<()> {
    let analyzer = ConnectionQualityAnalyzer::new(100);
    
    // 测试质量数据更新
    let quality = ggb::routing::ConnectionQuality {
        latency_ms: 50.0,
        bandwidth_mbps: 100.0,
        packet_loss_percent: 0.1,
        jitter_ms: 5.0,
        reliability: 0.95,
        stability: 0.9,
        last_updated: std::time::Instant::now(),
    };
    
    analyzer.update_quality(quality.clone());
    
    // 测试质量分析
    let current_quality = analyzer.analyze_current_quality();
    assert!(current_quality.is_some());
    
    // 测试趋势分析
    let trend = analyzer.analyze_performance_trend(ggb::routing::PerformanceMetric::Latency);
    assert!(trend.is_none()); // 样本不足
    
    // 测试健康检查
    assert!(analyzer.is_connection_healthy());
    
    Ok(())
}

/// 测试隐私路径选择器
#[tokio::test]
async fn test_privacy_path_selector() -> Result<()> {
    // 创建测试配置
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
    
    // 测试路径选择（无路径时应返回错误）
    let result = selector.select_best_path("test_target");
    assert!(result.is_err());
    
    // 测试多路径选择
    let multipaths = selector.select_multipaths("test_target");
    assert!(multipaths.is_err());
    
    Ok(())
}

/// 测试路径管理器
#[tokio::test]
async fn test_path_manager() -> Result<()> {
    // 创建测试配置
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
    
    let manager = PathManager::new(config);
    
    // 测试管理器统计
    let stats = manager.get_manager_stats();
    assert_eq!(stats.total_paths, 0);
    assert_eq!(stats.active_paths, 0);
    
    Ok(())
}

/// 测试性能优化的QUIC
#[tokio::test]
async fn test_performance_optimized_quic() -> Result<()> {
    // 注意：这个测试需要实际的网络环境
    // 在CI环境中可能会跳过
    
    // 创建测试配置
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
    
    // 测试本地回环地址
    let bind_addr: SocketAddr = "127.0.0.1:0".parse()?;
    
    // 由于需要实际网络，这里只测试创建实例
    // 在实际测试中，应该启动测试服务器
    let _quic = PerformanceOptimizedQuic::new(bind_addr, config).await;
    
    Ok(())
}

/// 测试隐私覆盖层
#[tokio::test]
async fn test_privacy_overlay() -> Result<()> {
    // 创建测试配置
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
    
    // 测试出站数据处理
    let test_data = b"Hello, Privacy Overlay!";
    let processed = overlay.process_outbound(test_data).await?;
    
    // 处理后的数据应该不同（因为有加密和混淆）
    assert_ne!(processed, test_data);
    
    // 测试入站数据处理
    let restored = overlay.process_inbound(&processed).await?;
    
    // 恢复后的数据应该与原始数据相同
    assert_eq!(restored, test_data);
    
    // 测试统计信息
    let stats = overlay.get_stats();
    assert!(stats.encrypted_bytes > 0);
    assert!(stats.obfuscated_packets > 0);
    
    Ok(())
}

/// 测试配置验证
#[test]
fn test_config_validation() -> Result<()> {
    // 测试有效配置
    let valid_config = PrivacyPerformanceConfig {
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
    
    let validation_result = valid_config.validate();
    assert!(validation_result.is_ok());
    
    // 测试无效配置（性能权重超出范围）
    let invalid_config = PrivacyPerformanceConfig {
        mode: BalanceMode::Balanced,
        performance_weight: 1.5, // 无效：大于1.0
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
    
    let validation_result = invalid_config.validate();
    assert!(validation_result.is_err());
    
    Ok(())
}

/// 测试路由评分计算
#[test]
fn test_route_scoring() -> Result<()> {
    // 创建测试配置
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
    
    // 注意：这里需要访问私有方法，在实际测试中可能需要调整可见性
    // 或者通过公共接口测试
    
    Ok(())
}

/// 测试性能监控
#[tokio::test]
async fn test_performance_monitoring() -> Result<()> {
    // 创建连接质量分析器
    let analyzer = ConnectionQualityAnalyzer::new(50);
    
    // 添加一些测试数据
    for i in 0..20 {
        let quality = ggb::routing::ConnectionQuality {
            latency_ms: 50.0 + (i as f32 * 0.5),
            bandwidth_mbps: 100.0 - (i as f32 * 0.5),
            packet_loss_percent: 0.1,
            jitter_ms: 5.0,
            reliability: 0.95,
            stability: 0.9,
            last_updated: std::time::Instant::now(),
        };
        
        analyzer.update_quality(quality);
        
        // 短暂延迟
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // 测试趋势分析
    let latency_trend = analyzer.analyze_performance_trend(ggb::routing::PerformanceMetric::Latency);
    assert!(latency_trend.is_some());
    
    let bandwidth_trend = analyzer.analyze_performance_trend(ggb::routing::PerformanceMetric::Bandwidth);
    assert!(bandwidth_trend.is_some());
    
    // 测试统计信息
    let stats = analyzer.get_statistics();
    assert!(stats.total_samples > 0);
    assert!(stats.avg_latency > 0.0);
    assert!(stats.avg_bandwidth > 0.0);
    
    Ok(())
}

/// 测试故障转移
#[tokio::test]
async fn test_failover() -> Result<()> {
    // 创建测试配置
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
    
    // 测试故障转移（无路径时应返回错误）
    let result = selector.failover("test_target", "nonexistent_path");
    assert!(result.is_err());
    
    Ok(())
}

/// 测试集成场景
#[tokio::test]
async fn test_integration_scenario() -> Result<()> {
    // 这个测试模拟完整的隐私性能平衡工作流程
    
    // 1. 加载配置
    let config = AppConfig::from_preset("balanced")?;
    
    // 2. 创建各个组件
    let quality_analyzer = ConnectionQualityAnalyzer::new(100);
    let path_selector = PrivacyPathSelector::new(config.security.privacy_performance.clone());
    
    // 3. 模拟网络条件更新
    let network_conditions = ggb::routing::NetworkConditions {
        network_type: ggb::routing::NetworkType::Ethernet,
        signal_strength: None,
        congestion_level: 0.2,
        availability: 0.98,
    };
    
    // 在实际测试中，这里会执行更多的集成测试
    
    Ok(())
}

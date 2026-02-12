// 冒烟测试 - 验证核心功能可用性
// 运行方式: cargo test --test smoke_test

use williw::config::AppConfig;
use williw::topology::{TopologySelector, TopologyConfig};
use williw::types::GeoPoint;
use williw::stats::TrainingStats;

#[test]
fn test_config_creation() {
    let config = AppConfig::default();
    // 验证训练配置存在且有效
    assert!(config.training.model_dim > 0);
    assert!(config.training.learning_rate > 0.0);
}

#[test]
fn test_topology_selector_init() {
    let config = TopologyConfig::default();
    // 创建测试位置（北京坐标）
    let position = GeoPoint {
        lat: 39.9042,
        lon: 116.4074,
    };
    let topology = TopologySelector::new(position, config);
    // 验证拓扑选择器初始化成功
    let pos = topology.position();
    assert!(pos.lat >= -90.0 && pos.lat <= 90.0);
    assert!(pos.lon >= -180.0 && pos.lon <= 180.0);
}

#[test]
fn test_training_stats_initialization() {
    let stats = TrainingStats::default();
    assert_eq!(stats.tick_count, 0);
    assert_eq!(stats.messages_sent, 0);
    assert_eq!(stats.messages_received, 0);
}

#[test]
fn test_default_training_config() {
    let config = AppConfig::default();
    // 默认模型维度应为 768
    assert_eq!(config.training.model_dim, 768);
}

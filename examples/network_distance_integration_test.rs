//! 网络距离感知功能完整集成测试
//!
//! 这个示例演示了如何在实际应用中使用网络距离感知功能

use williw::types::{NetworkDistance, DistanceLevel};
use williw::network::latency::{NetworkLatencyDetector, DistanceCalculator};
use williw::topology::TopologySelector;
use williw::types::GeoPoint;
use iroh::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 网络距离感知功能完整测试 ===");
    
    // 1. 测试 NetworkDistance 类型
    println!("\n1. 测试 NetworkDistance 类型:");
    let mut network_distance = NetworkDistance::new();
    
    // 设置 DERP 节点延迟
    network_distance.relay_delays = vec![
        ("https://ny1.derp.iroh.network:443".to_string(), 45),
        ("https://sg1.derp.iroh.network:443".to_string(), 120),
        ("https://tokyo1.derp.iroh.network:443".to_string(), 200),
    ];
    
    // 设置端到端延迟
    network_distance.end_to_end_delay = Some(60);
    
    // 设置本地 DERP 延迟（模拟）
    network_distance.local_derp_delays = vec![
        ("https://local1.derp.iroh.network:443".to_string(), 30),
        ("https://local2.derp.iroh.network:443".to_string(), 50),
    ];
    
    println!("   网络距离: {:?}", network_distance);
    
    // 测试距离级别分类
    let distance_level = network_distance.distance_level();
    println!("   距离级别: {:?}", distance_level);
    
    // 测试模糊描述
    let description = network_distance.get_distance_description();
    println!("   模糊描述: {}", description);
    
    // 2. 测试网络延迟探测器
    println!("\n2. 测试网络延迟探测器:");
    let endpoint = Endpoint::builder()
        .bind_addr("0.0.0.0:0".parse()?)
        .spawn()?;
    
    let detector = NetworkLatencyDetector::new(endpoint.clone());
    
    // 获取本地 DERP 延迟（仅作演示，实际环境中需要真实网络环境）
    let local_delays = detector.get_net_report().await;
    if let Some(report) = local_delays {
        println!("   本地网络报告: {:?}", report.relay_latency.len());
    } else {
        println!("   无法获取网络报告（无网络连接）");
    }
    
    // 3. 测试距离计算器
    println!("\n3. 测试距离计算器:");
    let calculator = DistanceCalculator::new(endpoint.clone());
    
    // 测试距离估算
    let estimated_distance = calculator.detector.estimate_distance_km(60.0);
    println!("   估算距离 (60ms RTT): {:.2} km", estimated_distance);
    
    // 测试距离级别
    let level = calculator.detector.distance_level_from_rtt(60.0);
    println!("   RTT 60ms 对应距离级别: {:?}", level);
    
    // 4. 测试拓扑选择器
    println!("\n4. 测试拓扑选择器:");
    let config = williw::topology::TopologyConfig::default();
    let selector = TopologySelector::new(
        GeoPoint { lat: 39.9042, lon: 116.4074 }, // 北京
        config
    );
    
    // 更新节点网络距离信息
    selector.update_peer(
        "peer1",
        vec![0.1, 0.2, 0.3], // embedding
        GeoPoint { lat: 31.2304, lon: 121.4737 }, // 上海
        &[0.1, 0.2, 0.3], // self embedding
        network_distance.clone(), // 网络距离
    );
    
    // 获取网络亲和度
    let affinity = selector.get_peer_network_affinity("peer1");
    println!("   peer1 网络亲和度: {:?}", affinity);
    
    // 5. 测试相似度计算
    println!("\n5. 测试相似度计算:");
    let mut other_distance = NetworkDistance::new();
    other_distance.relay_delays = vec![
        ("https://ny1.derp.iroh.network:443".to_string(), 50),
        ("https://sg1.derp.iroh.network:443".to_string(), 110),
    ];
    other_distance.end_to_end_delay = Some(55);
    
    let similarity = network_distance.similarity_to(&other_distance);
    println!("   与另一个节点的相似度: {:.2}", similarity);
    
    println!("\n=== 测试完成 ===");
    
    Ok(())
}
//! 网络距离感知功能测试示例
//!
//! 这个示例演示了如何使用 Iroh 的 Netcheck 机制来实现基于网络延迟的模糊距离感知

use williw::types::{NetworkDistance, DistanceLevel};
use williw::network::latency::DistanceCalculator;
use iroh::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 网络距离感知功能测试 ===");
    
    // 创建一个示例网络距离对象
    let mut network_distance = NetworkDistance::new();
    
    // 模拟设置一些 DERP 节点延迟
    network_distance.relay_delays = vec![
        ("https://ny1.derp.iroh.network:443".to_string(), 45),
        ("https://sg1.derp.iroh.network:443".to_string(), 120),
        ("https://tokyo1.derp.iroh.network:443".to_string(), 200),
    ];
    
    // 设置端到端延迟
    network_distance.end_to_end_delay = Some(60);
    
    // 测试距离级别分类
    let distance_level = network_distance.distance_level();
    println!("网络距离级别: {:?}", distance_level);
    
    // 测试模糊距离描述
    let description = network_distance.get_distance_description();
    println!("模糊距离描述: {}", description);
    
    // 创建距离计算器
    let endpoint = Endpoint::builder()
        .bind_addr("0.0.0.0:0".parse()?)
        .spawn()?;
    
    let calculator = DistanceCalculator::new(endpoint);
    
    // 测试本地 DERP 延迟获取
    let local_delays = calculator.get_local_derp_delays().await;
    println!("本地 DERP 节点延迟: {:?}", local_delays);
    
    println!("=== 测试完成 ===");
    
    Ok(())
}
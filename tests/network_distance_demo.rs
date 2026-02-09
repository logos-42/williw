//! 网络距离估算功能演示
//! 
//! 演示如何使用网络延迟作为地理距离的代理，
//! 在不暴露精确位置的情况下判断节点间的相对距离

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum DistanceLevel {
    VeryClose,  // 非常近（<20ms）- 可能在同一城市
    Close,      // 近（21-100ms）- 同一国家
    Medium,     // 中等（101-300ms）- 跨洲
    Far,        // 远（>300ms）- 全球范围
    Unknown,    // 未知
}

#[derive(Debug, Clone)]
pub struct NetworkDistance {
    /// 到各个中继节点的延迟（毫秒）
    pub relay_delays: Vec<(String, u64)>, // (relay_url, delay_ms)
    /// 端到端延迟（毫秒）
    pub end_to_end_delay: Option<u64>,
}

impl NetworkDistance {
    pub fn new() -> Self {
        Self {
            relay_delays: Vec::new(),
            end_to_end_delay: None,
        }
    }

    /// 根据延迟估算距离级别
    pub fn distance_level(&self) -> DistanceLevel {
        if let Some(delay) = self.end_to_end_delay {
            match delay {
                0..=20 => DistanceLevel::VeryClose,
                21..=100 => DistanceLevel::Close,
                101..=300 => DistanceLevel::Medium,
                _ => DistanceLevel::Far,
            }
        } else if let Some(min_delay) = self.relay_delays.iter().map(|(_, delay)| delay).min() {
            match min_delay {
                0..=50 => DistanceLevel::Close,
                51..=150 => DistanceLevel::Medium,
                _ => DistanceLevel::Far,
            }
        } else {
            DistanceLevel::Unknown
        }
    }

    /// 计算与另一个网络距离的相似度
    pub fn similarity_to(&self, other: &Self) -> f32 {
        // 基于共同中继节点的延迟相似性计算
        let common_relays: HashMap<String, (u64, u64)> = self
            .relay_delays
            .iter()
            .filter_map(|(url, delay1)| {
                other
                    .relay_delays
                    .iter()
                    .find(|(other_url, _)| other_url == url)
                    .map(|(_, delay2)| (url.clone(), (*delay1, *delay2)))
            })
            .collect();

        if common_relays.is_empty() {
            return 0.0;
        }

        let avg_diff: f32 = common_relays
            .values()
            .map(|(d1, d2)| (*d1 as f32 - *d2 as f32).abs())
            .sum::<f32>()
            / common_relays.len() as f32;

        // 将平均差异转换为相似度（差异越小，相似度越高）
        (1.0 / (1.0 + avg_diff / 100.0)).min(1.0)
    }
}

fn main() {
    println!("🌐 网络距离估算演示");
    println!("========================\n");

    // 演示不同场景的网络距离判断
    println!("📍 场景1: 距离级别判断");
    let scenarios = vec![
        (5, "本地/局域网"),
        (15, "同城市"),
        (50, "同国家"),
        (150, "跨洲"),
        (400, "远距离"),
    ];

    for (delay, description) in scenarios {
        let distance = NetworkDistance {
            relay_delays: vec![],
            end_to_end_delay: Some(delay),
        };
        println!("  {}延迟 {:4}ms -> {:?}", description, delay, distance.distance_level());
    }
    println!();

    // 演示通过中继节点估算距离
    println!("📍 场景2: 通过中继节点估算");
    let node_a = NetworkDistance {
        relay_delays: vec![
            ("us-east.relay.com".to_string(), 20),
            ("eu-west.relay.com".to_string(), 120),
            ("ap-southeast.relay.com".to_string(), 180),
        ],
        end_to_end_delay: None,
    };

    let node_b = NetworkDistance {
        relay_delays: vec![
            ("us-east.relay.com".to_string(), 25),
            ("eu-west.relay.com".to_string(), 115),
            ("ap-southeast.relay.com".to_string(), 185),
        ],
        end_to_end_delay: None,
    };

    let node_c = NetworkDistance {
        relay_delays: vec![
            ("us-east.relay.com".to_string(), 200),
            ("eu-west.relay.com".to_string(), 80),
            ("ap-southeast.relay.com".to_string(), 60),
        ],
        end_to_end_delay: None,
    };

    println!("  节点A中继延迟: {:?}", node_a.relay_delays);
    println!("  节点A距离级别: {:?}", node_a.distance_level());
    println!(" 节点B中继延迟: {:?}", node_b.relay_delays);
    println!("  节点B距离级别: {:?}", node_b.distance_level());
    println!("  节点C中继延迟: {:?}", node_c.relay_delays);
    println!("  节点C距离级别: {:?}", node_c.distance_level());
    println!();

    // 演示相似性计算
    println!("📊 场景3: 网络距离相似性");
    println!("  A与B的相似性: {:.2}", node_a.similarity_to(&node_b));
    println!("  A与C的相似性: {:.2}", node_a.similarity_to(&node_c));
    println!("  B与C的相似性: {:.2}", node_b.similarity_to(&node_c));
    println!();

    // 演示隐私保护优势
    println!("🔒 场景4: 隐私保护优势");
    println!("  传统方法: 需要IP地址 -> 精确地理位置（暴露隐私）");
    println!("  Iroh方法: 只需网络延迟 -> 模糊距离级别（保护隐私）");
    println!();

    let distance_with_rtt = NetworkDistance {
        relay_delays: vec![("example.relay.com".to_string(), 40)],
        end_to_end_delay: Some(30),
    };

    println!("  示例: 某节点RTT 30ms");
    println!("  - 距离级别: {:?}", distance_with_rtt.distance_level());
    println!("  - 地理含义: 可能与您在同一国家或相近区域");
    println!("  - 隐私保护: 不暴露具体IP或精确位置");
    println!();

    println!("💡 核心优势:");
    println!("  ✓ 隐私保护 - 不暴露精确位置信息");
    println!(" ✓ 实用性 - 足够判断是否在同一区域");
    println!("  ✓ 效率 - 基于现有网络测量");
    println!("  ✓ 可扩展 - 支持多中继节点验证");
    println!();

    println!("🎯 应用场景:");
    println!(" • P2P网络 - 选择延迟较低的邻居节点");
    println!("  • CDN - 选择地理位置较近的服务器");
    println!(" • 游戏匹配 - 匹配延迟较低的玩家");
    println!("  • 分布式系统 - 优化数据同步路径");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_levels() {
        let very_close = NetworkDistance {
            end_to_end_delay: Some(10),
            relay_delays: vec![],
        };
        assert_eq!(very_close.distance_level(), DistanceLevel::VeryClose);

        let close = NetworkDistance {
            end_to_end_delay: Some(50),
            relay_delays: vec![],
        };
        assert_eq!(close.distance_level(), DistanceLevel::Close);

        let medium = NetworkDistance {
            end_to_end_delay: Some(200),
            relay_delays: vec![],
        };
        assert_eq!(medium.distance_level(), DistanceLevel::Medium);

        let far = NetworkDistance {
            end_to_end_delay: Some(500),
            relay_delays: vec![],
        };
        assert_eq!(far.distance_level(), DistanceLevel::Far);
    }

    #[test]
    fn test_similarity_calculation() {
        let dist1 = NetworkDistance {
            relay_delays: vec![("relay1".to_string(), 50), ("relay2".to_string(), 60)],
            end_to_end_delay: None,
        };

        let dist2 = NetworkDistance {
            relay_delays: vec![("relay1".to_string(), 55), ("relay2".to_string(), 65)],
            end_to_end_delay: None,
        };

        // 应该有较高的相似性（延迟接近）
        let similarity = dist1.similarity_to(&dist2);
        assert!(similarity > 0.8);
    }

    #[test]
    fn test_relay_based_distance() {
        let dist_with_relays = NetworkDistance {
            relay_delays: vec![("relay1".to_string(), 10), ("relay2".to_string(), 20)],
            end_to_end_delay: None, // 没有端到端延迟
        };

        // 应该基于中继延迟判断
        assert_eq!(dist_with_relays.distance_level(), DistanceLevel::Close);
    }
}
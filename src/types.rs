use rand::Rng;
use serde::{Deserialize, Serialize};

/// 网络距离信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDistance {
    /// 到各个中继节点的延迟（毫秒）
    pub relay_delays: Vec<(String, u64)>, // (relay_url, delay_ms)
    /// 端到端延迟（毫秒）
    pub end_to_end_delay: Option<u64>,
    /// 本地网络的DERP节点延迟信息（用于模糊距离分类）
    pub local_derp_delays: Vec<(String, u64)>,
}

impl NetworkDistance {
    pub fn new() -> Self {
        Self {
            relay_delays: Vec::new(),
            end_to_end_delay: None,
            local_derp_delays: Vec::new(),
        }
    }

    /// 根据延迟估算距离级别
    pub fn distance_level(&self) -> DistanceLevel {
        // 优先使用端到端延迟
        if let Some(delay) = self.end_to_end_delay {
            match delay {
                0..=20 => DistanceLevel::VeryClose,
                21..=100 => DistanceLevel::Close,
                101..=300 => DistanceLevel::Medium,
                _ => DistanceLevel::Far,
            }
        }
        // 如果没有端到端延迟，则使用最小的中继延迟
        else if let Some(min_delay) = self.relay_delays.iter().map(|(_, delay)| delay).min() {
            match min_delay {
                0..=50 => DistanceLevel::Close,
                51..=150 => DistanceLevel::Medium,
                _ => DistanceLevel::Far,
            }
        }
        // 如果没有中继延迟，使用本地DERP延迟
        else if let Some(min_delay) = self.local_derp_delays.iter().map(|(_, delay)| delay).min() {
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
        let common_relays: std::collections::HashMap<String, (u64, u64)> = self
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
    
    /// 获取网络距离的模糊描述
    pub fn get_distance_description(&self) -> String {
        match self.distance_level() {
            DistanceLevel::VeryClose => "非常近".to_string(),
            DistanceLevel::Close => "同城".to_string(),
            DistanceLevel::Medium => "同国".to_string(),
            DistanceLevel::Far => "远距离".to_string(),
            DistanceLevel::Unknown => "未知".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DistanceLevel {
    VeryClose,  // 非常近（<20ms）
    Close,      // 近（21-100ms）
    Medium,     // 中等（101-300ms）
    Far,        // 远（>300ms）
    Unknown,    // 未知
}

/// 旧的地理位置点（保留用于向后兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f32,
    pub lon: f32,
}

impl GeoPoint {
    pub fn random(rng: &mut impl Rng) -> Self {
        Self {
            lat: rng.random_range(-60.0..60.0),
            lon: rng.random_range(-180.0..180.0),
        }
    }

    pub fn distance_km(&self, other: &Self) -> f32 {
        const EARTH_RADIUS_KM: f32 = 6_371.0;
        let lat1 = self.lat.to_radians();
        let lat2 = other.lat.to_radians();
        let dlat = (other.lat - self.lat).to_radians();
        let dlon = (other.lon - self.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        EARTH_RADIUS_KM * c
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSnapshot {
    pub dim: usize,
    pub values: Vec<f32>,
    pub version: u64,
}

impl TensorSnapshot {
    pub fn new(values: Vec<f32>, version: u64) -> Self {
        Self {
            dim: values.len(),
            values,
            version,
        }
    }

    pub fn hash(&self) -> String {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(self.dim.to_le_bytes());
        hasher.update(self.version.to_le_bytes());
        for v in &self.values {
            hasher.update(v.to_ne_bytes());
        }
        format!("0x{}", hex::encode(hasher.finalize()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseUpdate {
    pub indices: Vec<u32>,
    pub values: Vec<f32>,
    pub version: u64,
}

pub fn decompress_indices(compressed: &[u32]) -> Vec<usize> {
    let mut out = Vec::with_capacity(compressed.len());
    let mut last = 0usize;
    for diff in compressed {
        let next = last + (*diff as usize);
        out.push(next);
        last = next;
    }
    out
}

/// Gossip 消息体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GgbMessage {
    Heartbeat {
        peer: String,
        model_hash: String,
    },
    SparseUpdate {
        update: SparseUpdate,
        sender: String,
    },
    DenseSnapshot {
        snapshot: TensorSnapshot,
        sender: String,
    },
    SimilarityProbe {
        embedding: Vec<f32>,
        position: GeoPoint,
        sender: String,
    },
}

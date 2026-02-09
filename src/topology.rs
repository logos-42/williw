use crate::types::{GeoPoint, NetworkDistance, DistanceLevel};
use parking_lot::RwLock;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const EMBEDDING_WEIGHT: f32 = 0.5;
const NETWORK_DISTANCE_WEIGHT: f32 = 0.3;
const GEO_WEIGHT: f32 = 0.2; // 保留地理权重用于向后兼容

#[derive(Clone, Debug)]
pub struct PeerProfile {
    pub embedding: Vec<f32>,
    pub position: GeoPoint, // 保留用于向后兼容
    pub network_distance: NetworkDistance,
    pub similarity: f32,
    pub geo_affinity: f32,
    pub network_affinity: f32,
    pub score: f32,
    pub last_seen: Instant,
}

#[derive(Clone)]
pub struct TopologyConfig {
    pub max_neighbors: usize,
    pub failover_pool: usize,
    pub min_score: f32,
    pub geo_scale_km: f32,
    pub peer_stale_secs: u64,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            max_neighbors: 8,
            failover_pool: 4,
            min_score: 0.15,
            geo_scale_km: 500.0,
            peer_stale_secs: 120,
        }
    }
}

pub struct TopologySelector {
    position: GeoPoint,
    peers: RwLock<HashMap<String, PeerProfile>>,
    config: TopologyConfig,
}

#[derive(Debug, Clone)]
pub struct PeerSnapshot {
    pub similarity: f32,
    pub geo_affinity: f32,
    pub position: GeoPoint,
    pub embedding_dim: usize,
    pub network_affinity: f32,  // 添加网络亲和度到快照
    pub network_distance: NetworkDistance,  // 添加网络距离到快照
}

impl TopologySelector {
    pub fn new(position: GeoPoint, config: TopologyConfig) -> Self {
        Self {
            position,
            peers: RwLock::new(HashMap::new()),
            config,
        }
    }

    pub fn position(&self) -> GeoPoint {
        self.position.clone()
    }

    pub fn network_distance(&self) -> NetworkDistance {
        // 为拓扑选择器添加网络距离信息
        NetworkDistance::new()
    }

    pub fn update_peer(
        &self,
        peer_id: &str,
        embedding: Vec<f32>,
        position: GeoPoint,
        self_embedding: &[f32],
        network_distance: NetworkDistance,
    ) {
        let similarity = cosine_sim(self_embedding, &embedding);
        let geo_affinity = self.geo_affinity(&position);
        // 计算网络亲和度（基于距离级别）
        let network_affinity = self.network_affinity(&network_distance);
        let score = EMBEDDING_WEIGHT * similarity + GEO_WEIGHT * geo_affinity + NETWORK_DISTANCE_WEIGHT * network_affinity;
        let profile = PeerProfile {
            embedding,
            position,
            network_distance,
            similarity,
            geo_affinity,
            network_affinity,
            score,
            last_seen: Instant::now(),
        };
        let mut peers = self.peers.write();
        peers.insert(peer_id.to_string(), profile);
        self.cleanup_locked(&mut peers);
    }

    /// 更新节点的网络距离信息
    pub fn update_peer_network_distance(&self, peer_id: &str, network_distance: NetworkDistance) {
        let mut peers = self.peers.write();
        if let Some(profile) = peers.get_mut(peer_id) {
            profile.network_distance = network_distance;
            // 重新计算网络亲和度
            profile.network_affinity = self.network_affinity(&profile.network_distance);
            // 重新计算总分
            profile.score = EMBEDDING_WEIGHT * profile.similarity +
                          GEO_WEIGHT * profile.geo_affinity +
                          NETWORK_DISTANCE_WEIGHT * profile.network_affinity;
        }
    }

    pub fn neighbor_sets(&self) -> (Vec<String>, Vec<String>) {
        let peers = self.peers.read();
        let mut ranked: Vec<_> = peers.iter().collect();
        ranked.sort_by(|(_, a), (_, b)| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        let mut primary = Vec::new();
        let mut backups = Vec::new();
        for (_idx, (peer, profile)) in ranked.into_iter().enumerate() {
            if profile.score < self.config.min_score {
                continue;
            }
            if primary.len() < self.config.max_neighbors {
                primary.push(peer.clone());
            } else if backups.len() < self.config.failover_pool {
                backups.push(peer.clone());
            } else {
                break;
            }
        }
        (primary, backups)
    }

    pub fn select_neighbors(&self) -> Vec<String> {
        self.neighbor_sets().0
    }

    pub fn mark_unreachable(&self, peer_id: &str) {
        let mut peers = self.peers.write();
        peers.remove(peer_id);
    }

    pub fn max_neighbors(&self) -> usize {
        self.config.max_neighbors
    }

    pub fn failover_pool(&self) -> usize {
        self.config.failover_pool
    }

    pub fn peer_snapshot(&self, peer_id: &str) -> Option<PeerSnapshot> {
        self.peers.read().get(peer_id).map(|profile| PeerSnapshot {
            similarity: profile.similarity,
            geo_affinity: profile.geo_affinity,
            position: profile.position.clone(),
            embedding_dim: profile.embedding.len(),
            network_affinity: profile.network_affinity,
            network_distance: profile.network_distance.clone(),
        })
    }

    pub fn geo_affinity(&self, other: &GeoPoint) -> f32 {
        let dist = self.position.distance_km(other);
        (self.config.geo_scale_km / (self.config.geo_scale_km + dist)).clamp(0.0, 1.0)
    }

    /// 计算基于网络距离的亲和度
    pub fn network_affinity(&self, network_distance: &NetworkDistance) -> f32 {
        // 根据距离级别计算亲和度
        match network_distance.distance_level() {
            DistanceLevel::VeryClose => 1.0,   // 非常近 - 最高亲和度
            DistanceLevel::Close => 0.8,       // 近 - 高亲和度
            DistanceLevel::Medium => 0.5,      // 中等 - 中等亲和度
            DistanceLevel::Far => 0.2,         // 远 - 低亲和度
            DistanceLevel::Unknown => 0.1,     // 未知 - 最低亲和度
        }
    }

    /// 获取特定节点的网络亲和度
    pub fn get_peer_network_affinity(&self, peer_id: &str) -> Option<f32> {
        let peers = self.peers.read();
        peers.get(peer_id).map(|profile| profile.network_affinity)
    }

    /// 获取拓扑统计信息
    pub fn get_topology_stats(&self) -> TopologyStats {
        let peers = self.peers.read();
        let (primary, backup) = self.neighbor_sets();

        let avg_similarity = peers.values().map(|p| p.similarity).sum::<f32>() / peers.len() as f32;
        let avg_geo_affinity = peers.values().map(|p| p.geo_affinity).sum::<f32>() / peers.len() as f32;
        let avg_network_affinity = peers.values().map(|p| p.network_affinity).sum::<f32>() / peers.len() as f32;

        TopologyStats {
            total_peers: peers.len(),
            primary_neighbors: primary.len(),
            backup_neighbors: backup.len(),
            avg_similarity,
            avg_geo_affinity,
            avg_network_affinity,
        }
    }

    fn cleanup_locked(&self, peers: &mut HashMap<String, PeerProfile>) {
        let deadline = Instant::now() - Duration::from_secs(self.config.peer_stale_secs);
        peers.retain(|_, profile| profile.last_seen >= deadline);
    }
}

/// 拓扑统计信息
#[derive(Debug, Clone)]
pub struct TopologyStats {
    pub total_peers: usize,
    pub primary_neighbors: usize,
    pub backup_neighbors: usize,
    pub avg_similarity: f32,
    pub avg_geo_affinity: f32,
    pub avg_network_affinity: f32,
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len().min(b.len()) {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

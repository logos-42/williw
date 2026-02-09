use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 训练统计数据
#[derive(Debug, Clone, Serialize)]
pub struct TrainingStats {
    /// 训练开始时间（Unix 时间戳，秒）
    #[serde(skip)]
    pub start_time: Instant,
    /// 训练开始时间戳（用于序列化）
    pub start_time_secs: u64,
    /// 训练轮次（tick count）
    pub tick_count: u64,
    /// 接收的稀疏更新数量
    pub sparse_updates_received: u64,
    /// 接收的密集快照数量
    pub dense_snapshots_received: u64,
    /// 发送的稀疏更新数量
    pub sparse_updates_sent: u64,
    /// 发送的密集快照数量
    pub dense_snapshots_sent: u64,
    /// 发送的心跳数量
    pub heartbeats_sent: u64,
    /// 接收的心跳数量
    pub heartbeats_received: u64,
    /// 发送的相似度探测数量
    pub probes_sent: u64,
    /// 接收的相似度探测数量
    pub probes_received: u64,
    /// 当前连接的节点数量
    pub connected_peers: usize,
    /// 模型版本号
    pub model_version: u64,
    /// 模型 hash（最新）
    pub model_hash: String,
    /// 每个节点的交互统计
    pub peer_stats: HashMap<String, PeerStats>,
}

/// 单个节点的统计信息
#[derive(Debug, Clone, Serialize)]
pub struct PeerStats {
    /// 接收的更新数量
    pub updates_received: u64,
    /// 发送的更新数量
    pub updates_sent: u64,
    /// 最后交互时间（Unix 时间戳，秒）
    #[serde(skip)]
    pub last_interaction: Instant,
    /// 最后交互时间戳（用于序列化）
    pub last_interaction_secs: u64,
}

impl PeerStats {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            updates_received: 0,
            updates_sent: 0,
            last_interaction: now,
            last_interaction_secs: now.elapsed().as_secs(), // 相对时间戳
        }
    }
}

impl Default for PeerStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 训练统计管理器
pub struct TrainingStatsManager {
    stats: Arc<RwLock<TrainingStats>>,
}

impl TrainingStatsManager {
    pub fn new(model_hash: String, model_version: u64) -> Self {
        let start_time = Instant::now();
        Self {
            stats: Arc::new(RwLock::new(TrainingStats {
                start_time,
                start_time_secs: 0, // 将在序列化时计算
                tick_count: 0,
                sparse_updates_received: 0,
                dense_snapshots_received: 0,
                sparse_updates_sent: 0,
                dense_snapshots_sent: 0,
                heartbeats_sent: 0,
                heartbeats_received: 0,
                probes_sent: 0,
                probes_received: 0,
                connected_peers: 0,
                model_version,
                model_hash,
                peer_stats: HashMap::new(),
            })),
        }
    }

    pub fn increment_tick(&self) {
        self.stats.write().tick_count += 1;
    }

    pub fn record_sparse_update_received(&self, peer_id: &str) {
        let mut stats = self.stats.write();
        stats.sparse_updates_received += 1;
        let now = Instant::now();
        let start_time = stats.start_time;
        let peer_stat = stats.peer_stats.entry(peer_id.to_string()).or_insert_with(PeerStats::new);
        peer_stat.updates_received += 1;
        peer_stat.last_interaction = now;
        peer_stat.last_interaction_secs = now.duration_since(start_time).as_secs();
    }

    pub fn record_dense_snapshot_received(&self, peer_id: &str) {
        let mut stats = self.stats.write();
        stats.dense_snapshots_received += 1;
        let now = Instant::now();
        let start_time = stats.start_time;
        let peer_stat = stats.peer_stats.entry(peer_id.to_string()).or_insert_with(PeerStats::new);
        peer_stat.updates_received += 1;
        peer_stat.last_interaction = now;
        peer_stat.last_interaction_secs = now.duration_since(start_time).as_secs();
    }

    pub fn record_sparse_update_sent(&self, peer_id: &str) {
        let mut stats = self.stats.write();
        stats.sparse_updates_sent += 1;
        let now = Instant::now();
        let start_time = stats.start_time;
        let peer_stat = stats.peer_stats.entry(peer_id.to_string()).or_insert_with(PeerStats::new);
        peer_stat.updates_sent += 1;
        peer_stat.last_interaction = now;
        peer_stat.last_interaction_secs = now.duration_since(start_time).as_secs();
    }

    pub fn record_dense_snapshot_sent(&self) {
        self.stats.write().dense_snapshots_sent += 1;
    }

    pub fn record_heartbeat_sent(&self) {
        self.stats.write().heartbeats_sent += 1;
    }

    pub fn record_heartbeat_received(&self, peer_id: &str) {
        let mut stats = self.stats.write();
        stats.heartbeats_received += 1;
        let now = Instant::now();
        let start_time = stats.start_time;
        let peer_stat = stats.peer_stats.entry(peer_id.to_string()).or_insert_with(PeerStats::new);
        peer_stat.last_interaction = now;
        peer_stat.last_interaction_secs = now.duration_since(start_time).as_secs();
    }

    pub fn record_probe_sent(&self) {
        self.stats.write().probes_sent += 1;
    }

    pub fn record_probe_received(&self, peer_id: &str) {
        let mut stats = self.stats.write();
        stats.probes_received += 1;
        let now = Instant::now();
        let start_time = stats.start_time;
        let peer_stat = stats.peer_stats.entry(peer_id.to_string()).or_insert_with(PeerStats::new);
        peer_stat.last_interaction = now;
        peer_stat.last_interaction_secs = now.duration_since(start_time).as_secs();
    }

    pub fn update_connected_peers(&self, count: usize) {
        self.stats.write().connected_peers = count;
    }

    pub fn update_model(&self, hash: String, version: u64) {
        let mut stats = self.stats.write();
        stats.model_hash = hash;
        stats.model_version = version;
    }

    #[allow(dead_code)]
    pub fn get(&self) -> TrainingStats {
        self.stats.read().clone()
    }

    pub fn get_summary(&self) -> StatsSummary {
        let stats = self.stats.read();
        let elapsed = stats.start_time.elapsed();
        StatsSummary {
            elapsed_secs: elapsed.as_secs(),
            tick_count: stats.tick_count,
            sparse_updates_received: stats.sparse_updates_received,
            dense_snapshots_received: stats.dense_snapshots_received,
            sparse_updates_sent: stats.sparse_updates_sent,
            dense_snapshots_sent: stats.dense_snapshots_sent,
            connected_peers: stats.connected_peers,
            model_version: stats.model_version,
            model_hash: stats.model_hash.clone(),
            total_interactions: stats.sparse_updates_received
                + stats.dense_snapshots_received
                + stats.sparse_updates_sent
                + stats.dense_snapshots_sent,
        }
    }

    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        let mut stats = self.stats.read().clone();
        // 更新时间戳
        let elapsed = stats.start_time.elapsed();
        stats.start_time_secs = elapsed.as_secs();
        // 更新所有 peer 的时间戳
        for peer_stat in stats.peer_stats.values_mut() {
            let elapsed = peer_stat.last_interaction.duration_since(stats.start_time);
            peer_stat.last_interaction_secs = elapsed.as_secs();
        }
        serde_json::to_string_pretty(&stats)
    }

    pub fn export_json_to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.export_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// 统计摘要（用于快速显示）
#[derive(Debug, Clone)]
pub struct StatsSummary {
    pub elapsed_secs: u64,
    pub tick_count: u64,
    pub sparse_updates_received: u64,
    pub dense_snapshots_received: u64,
    pub sparse_updates_sent: u64,
    pub dense_snapshots_sent: u64,
    pub connected_peers: usize,
    pub model_version: u64,
    pub model_hash: String,
    #[allow(dead_code)]
    pub total_interactions: u64,
}

impl StatsSummary {
    pub fn format(&self) -> String {
        format!(
            "训练统计 [运行 {}s, {} ticks] | 连接: {} 节点 | 接收: {} 稀疏 + {} 密集 | 发送: {} 稀疏 + {} 密集 | 模型: v{} ({})",
            self.elapsed_secs,
            self.tick_count,
            self.connected_peers,
            self.sparse_updates_received,
            self.dense_snapshots_received,
            self.sparse_updates_sent,
            self.dense_snapshots_sent,
            self.model_version,
            &self.model_hash[..16]
        )
    }
}


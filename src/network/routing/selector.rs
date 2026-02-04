//! 隐私路径选择器模块
//! 
//! 提供基于隐私需求的路由选择、多路径负载均衡和故障转移功能

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use anyhow::Result;

use crate::config::PrivacyPerformanceConfig;
use super::quality::ConnectionQuality;

/// 路径类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    Direct,      // 直接路径
    SingleRelay, // 单中继路径
    MultiRelay,  // 多中继路径
    Tor,         // Tor网络路径
    Mixnet,      // 混合网络路径
    Custom,      // 自定义路径
}

/// 路径选择策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PathSelectionStrategy {
    PrivacyFirst,      // 隐私优先
    PerformanceFirst,  // 性能优先
    Balanced,          // 平衡策略
    Adaptive,          // 自适应策略
    LatencySensitive,  // 延迟敏感
    BandwidthSensitive, // 带宽敏感
}

/// 负载均衡策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoadBalanceStrategy {
    RoundRobin,        // 轮询
    Weighted,          // 加权
    LeastConnections,  // 最少连接
    LatencyBased,      // 基于延迟
    BandwidthBased,    // 基于带宽
    PrivacyAware,      // 隐私感知
}

/// 路径信息
#[derive(Debug, Clone, PartialEq)]
pub struct PathInfo {
    pub path_id: String,
    pub path_type: PathType,
    pub target: String,
    pub hops: Vec<String>,  // 路径上的节点列表
    pub quality: ConnectionQuality,
    pub privacy_level: f32,  // 0.0-1.0
    pub performance_level: f32,  // 0.0-1.0
    pub established_at: Instant,
    pub usage_count: u64,
    pub last_used: Instant,
    pub is_active: bool,
}

/// 路径评分
#[derive(Debug, Clone)]
pub struct PathScore {
    pub privacy_score: f32,      // 0.0-1.0
    pub performance_score: f32,  // 0.0-1.0
    pub reliability_score: f32,  // 0.0-1.0
    pub cost_score: f32,         // 0.0-1.0（成本/资源消耗）
    pub overall_score: f32,      // 加权总分
    pub weighted_scores: HashMap<String, f32>, // 各维度加权分
}

/// 路径选择结果
#[derive(Debug, Clone)]
pub struct PathSelectionResult {
    pub timestamp: Instant,
    pub target: String,
    pub selected_path: PathInfo,
    pub alternative_paths: Vec<PathInfo>,
    pub selection_reason: String,
    pub scores: PathScore,
    pub strategy_used: PathSelectionStrategy,
}

/// 多路径配置
#[derive(Debug, Clone)]
pub struct MultiPathConfig {
    pub enabled: bool,
    pub max_paths: usize,
    pub min_privacy_level: f32,
    pub min_performance_level: f32,
    pub load_balance_strategy: LoadBalanceStrategy,
    pub failover_enabled: bool,
    pub failover_threshold: f32,  // 故障转移阈值
    pub health_check_interval_secs: u64,
}

/// 路径选择器错误类型
#[derive(Debug, thiserror::Error)]
pub enum PathSelectionError {
    #[error("无可用路径")]
    NoPathsAvailable,
    
    #[error("隐私要求不满足: 需要 {required}, 实际 {actual}")]
    PrivacyRequirementNotMet { required: f32, actual: f32 },
    
    #[error("性能要求不满足: 需要 {required}, 实际 {actual}")]
    PerformanceRequirementNotMet { required: f32, actual: f32 },
    
    #[error("路径质量过低: {reason}")]
    PoorPathQuality { reason: String },
    
    #[error("路径发现失败: {reason}")]
    PathDiscoveryFailed { reason: String },
    
    #[error("路径建立失败: {reason}")]
    PathEstablishmentFailed { reason: String },
    
    #[error("负载均衡失败: {reason}")]
    LoadBalanceFailed { reason: String },
}

/// 隐私路径选择器
pub struct PrivacyPathSelector {
    /// 可用路径映射
    available_paths: RwLock<HashMap<String, Vec<PathInfo>>>,
    /// 路径评分缓存
    path_scores: RwLock<HashMap<String, PathScore>>,
    /// 选择历史
    selection_history: RwLock<VecDeque<PathSelectionResult>>,
    /// 配置
    config: Arc<PrivacyPerformanceConfig>,
    /// 多路径配置
    multipath_config: MultiPathConfig,
    /// 当前使用的路径
    active_paths: RwLock<HashMap<String, PathInfo>>,
    /// 路径健康状态
    path_health: RwLock<HashMap<String, bool>>,
}

impl PrivacyPathSelector {
    /// 创建新的隐私路径选择器
    pub fn new(config: PrivacyPerformanceConfig) -> Self {
        let multipath_config = MultiPathConfig {
            enabled: true,
            max_paths: 3,
            min_privacy_level: config.min_privacy_score,
            min_performance_level: config.min_performance_score,
            load_balance_strategy: LoadBalanceStrategy::PrivacyAware,
            failover_enabled: true,
            failover_threshold: 0.3,
            health_check_interval_secs: 30,
        };
        
        Self {
            available_paths: RwLock::new(HashMap::new()),
            path_scores: RwLock::new(HashMap::new()),
            selection_history: RwLock::new(VecDeque::with_capacity(100)),
            config: Arc::new(config),
            multipath_config,
            active_paths: RwLock::new(HashMap::new()),
            path_health: RwLock::new(HashMap::new()),
        }
    }
    
    /// 添加可用路径
    pub fn add_path(&self, target: &str, path: PathInfo) {
        let mut paths = self.available_paths.write();
        paths.entry(target.to_string())
            .or_insert_with(Vec::new)
            .push(path);
    }
    
    /// 移除路径
    pub fn remove_path(&self, target: &str, path_id: &str) {
        let mut paths = self.available_paths.write();
        if let Some(target_paths) = paths.get_mut(target) {
            target_paths.retain(|p| p.path_id != path_id);
        }
    }
    
    /// 选择最佳路径
    pub fn select_best_path(&self, target: &str) -> Result<PathInfo, PathSelectionError> {
        let paths = self.available_paths.read();
        let target_paths = paths.get(target)
            .ok_or(PathSelectionError::NoPathsAvailable)?;
            
        if target_paths.is_empty() {
            return Err(PathSelectionError::NoPathsAvailable);
        }
        
        // 计算每个路径的评分
        let mut scored_paths: Vec<(PathInfo, PathScore)> = target_paths
            .iter()
            .filter(|p| p.is_active)
            .map(|path| {
                let score = self.calculate_path_score(path);
                (path.clone(), score)
            })
            .collect();
            
        if scored_paths.is_empty() {
            return Err(PathSelectionError::NoPathsAvailable);
        }
        
        // 按总分排序
        scored_paths.sort_by(|a, b| {
            b.1.overall_score.partial_cmp(&a.1.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 检查是否满足最小要求
        let best_path = &scored_paths[0].0;
        let best_score = &scored_paths[0].1;
        
        if best_score.privacy_score < self.config.min_privacy_score {
            return Err(PathSelectionError::PrivacyRequirementNotMet {
                required: self.config.min_privacy_score,
                actual: best_score.privacy_score,
            });
        }
        
        if best_score.performance_score < self.config.min_performance_score {
            return Err(PathSelectionError::PerformanceRequirementNotMet {
                required: self.config.min_performance_score,
                actual: best_score.performance_score,
            });
        }
        
        // 记录选择历史
        let selection = PathSelectionResult {
            timestamp: Instant::now(),
            target: target.to_string(),
            selected_path: best_path.clone(),
            alternative_paths: scored_paths[1..].iter()
                .map(|(path, _)| path.clone())
                .collect(),
            selection_reason: format!("最佳评分: {:.2}", best_score.overall_score),
            scores: best_score.clone(),
            strategy_used: self.get_selection_strategy(),
        };
        
        self.record_selection(selection);
        
        // 更新活动路径
        let mut active_paths = self.active_paths.write();
        active_paths.insert(target.to_string(), best_path.clone());
        
        Ok(best_path.clone())
    }
    
    /// 计算路径评分
    fn calculate_path_score(&self, path: &PathInfo) -> PathScore {
        let privacy_score = self.calculate_privacy_score(path);
        let performance_score = self.calculate_performance_score(&path.quality);
        let reliability_score = path.quality.reliability;
        let cost_score = self.calculate_cost_score(path);
        
        // 根据配置的权重计算总分
        let overall_score = 
            performance_score * self.config.performance_weight +
            privacy_score * (1.0 - self.config.performance_weight) +
            reliability_score * 0.2 +
            cost_score * 0.1;
            
        let mut weighted_scores = HashMap::new();
        weighted_scores.insert("privacy".to_string(), privacy_score);
        weighted_scores.insert("performance".to_string(), performance_score);
        weighted_scores.insert("reliability".to_string(), reliability_score);
        weighted_scores.insert("cost".to_string(), cost_score);
        
        PathScore {
            privacy_score,
            performance_score,
            reliability_score,
            cost_score,
            overall_score,
            weighted_scores,
        }
    }
    
    /// 计算隐私评分
    fn calculate_privacy_score(&self, path: &PathInfo) -> f32 {
        let base_privacy = match path.path_type {
            PathType::Direct => 0.3,
            PathType::SingleRelay => 0.6,
            PathType::MultiRelay => 0.8,
            PathType::Tor => 0.95,
            PathType::Mixnet => 0.9,
            PathType::Custom => path.privacy_level,
        };
        
        // 考虑路径长度（跳数越多，隐私性越好）
        let hop_factor = if path.hops.len() > 1 {
            1.0 + (path.hops.len() as f32 * 0.05).min(0.3)
        } else {
            1.0
        };
        
        (base_privacy * hop_factor).min(1.0)
    }
    
    /// 计算性能评分
    fn calculate_performance_score(&self, quality: &ConnectionQuality) -> f32 {
        let latency_score = if quality.latency_ms < 20.0 {
            1.0
        } else if quality.latency_ms < 50.0 {
            0.8
        } else if quality.latency_ms < 100.0 {
            0.6
        } else if quality.latency_ms < 200.0 {
            0.4
        } else {
            0.2
        };
        
        let bandwidth_score = if quality.bandwidth_mbps > 100.0 {
            1.0
        } else if quality.bandwidth_mbps > 50.0 {
            0.8
        } else if quality.bandwidth_mbps > 20.0 {
            0.6
        } else if quality.bandwidth_mbps > 10.0 {
            0.4
        } else {
            0.2
        };
        
        let reliability_score = quality.reliability;
        let stability_score = quality.stability;
        
        // 加权平均
        (latency_score * 0.3 + bandwidth_score * 0.3 + 
         reliability_score * 0.2 + stability_score * 0.2)
            .clamp(0.0, 1.0)
    }
    
    /// 计算成本评分（越低越好，所以用1.0减）
    fn calculate_cost_score(&self, path: &PathInfo) -> f32 {
        match path.path_type {
            PathType::Direct => 0.9,      // 直接连接成本低
            PathType::SingleRelay => 0.7, // 单中继中等成本
            PathType::MultiRelay => 0.5,  // 多中继成本较高
            PathType::Tor => 0.3,         // Tor网络成本高
            PathType::Mixnet => 0.4,      // 混合网络成本较高
            PathType::Custom => 0.6,      // 自定义路径中等成本
        }
    }
    
    /// 获取选择策略
    fn get_selection_strategy(&self) -> PathSelectionStrategy {
        match self.config.mode {
            crate::config::BalanceMode::Performance => PathSelectionStrategy::PerformanceFirst,
            crate::config::BalanceMode::Balanced => PathSelectionStrategy::Balanced,
            crate::config::BalanceMode::Privacy => PathSelectionStrategy::PrivacyFirst,
            crate::config::BalanceMode::Adaptive => PathSelectionStrategy::Adaptive,
        }
    }
    
    /// 记录选择历史
    fn record_selection(&self, selection: PathSelectionResult) {
        let mut history = self.selection_history.write();
        history.push_back(selection);
        
        if history.len() > 100 {
            history.pop_front();
        }
    }
    
    /// 选择多路径（负载均衡）
    pub fn select_multipaths(&self, target: &str) -> Result<Vec<PathInfo>, PathSelectionError> {
        if !self.multipath_config.enabled {
            let best_path = self.select_best_path(target)?;
            return Ok(vec![best_path]);
        }
        
        let paths = self.available_paths.read();
        let target_paths = paths.get(target)
            .ok_or(PathSelectionError::NoPathsAvailable)?;
            
        if target_paths.is_empty() {
            return Err(PathSelectionError::NoPathsAvailable);
        }
        
        // 计算所有路径的评分
        let mut scored_paths: Vec<(PathInfo, PathScore)> = target_paths
            .iter()
            .filter(|p| p.is_active)
            .map(|path| {
                let score = self.calculate_path_score(path);
                (path.clone(), score)
            })
            .collect();
            
        if scored_paths.is_empty() {
            return Err(PathSelectionError::NoPathsAvailable);
        }
        
        // 按总分排序
        scored_paths.sort_by(|a, b| {
            b.1.overall_score.partial_cmp(&a.1.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 过滤满足最小要求的路径
        let qualified_paths: Vec<PathInfo> = scored_paths
            .iter()
            .filter(|(_, score)| {
                score.privacy_score >= self.multipath_config.min_privacy_level &&
                score.performance_score >= self.multipath_config.min_performance_level
            })
            .map(|(path, _)| path.clone())
            .collect();
            
        if qualified_paths.is_empty() {
            return Err(PathSelectionError::NoPathsAvailable);
        }
        
        // 根据负载均衡策略选择路径
        let selected_paths = match self.multipath_config.load_balance_strategy {
            LoadBalanceStrategy::RoundRobin => self.select_round_robin(&qualified_paths),
            LoadBalanceStrategy::Weighted => self.select_weighted(&qualified_paths, &scored_paths),
            LoadBalanceStrategy::LeastConnections => self.select_least_connections(&qualified_paths),
            LoadBalanceStrategy::LatencyBased => self.select_latency_based(&qualified_paths),
            LoadBalanceStrategy::BandwidthBased => self.select_bandwidth_based(&qualified_paths),
            LoadBalanceStrategy::PrivacyAware => self.select_privacy_aware(&qualified_paths, &scored_paths),
        };
        
        // 限制最大路径数
        let max_paths = self.multipath_config.max_paths.min(selected_paths.len());
        let final_paths = selected_paths.into_iter().take(max_paths).collect();
        
        Ok(final_paths)
    }
    
    /// 轮询选择
    fn select_round_robin(&self, paths: &[PathInfo]) -> Vec<PathInfo> {
        // 简单实现：按顺序选择
        paths.to_vec()
    }
    
    /// 加权选择
    fn select_weighted(&self, paths: &[PathInfo], scored_paths: &[(PathInfo, PathScore)]) -> Vec<PathInfo> {
        // 根据评分权重选择
        let mut result = Vec::new();
        let total_weight: f32 = scored_paths.iter()
            .map(|(_, score)| score.overall_score)
            .sum();
            
        if total_weight > 0.0 {
            for (path, score) in scored_paths {
                if paths.contains(path) {
                    // 根据权重比例决定是否选择
                    let selection_probability = score.overall_score / total_weight;
                    if rand::random::<f32>() < selection_probability {
                        result.push(path.clone());
                    }
                }
            }
        }
        
        if result.is_empty() && !paths.is_empty() {
            result.push(paths[0].clone());
        }
        
        result
    }
    
    /// 最少连接选择
    fn select_least_connections(&self, paths: &[PathInfo]) -> Vec<PathInfo> {
        // 按使用次数排序，选择使用最少的
        let mut sorted_paths = paths.to_vec();
        sorted_paths.sort_by_key(|p| p.usage_count);
        sorted_paths
    }
    
    /// 基于延迟选择
    fn select_latency_based(&self, paths: &[PathInfo]) -> Vec<PathInfo> {
        // 按延迟排序，选择延迟最低的
        let mut sorted_paths = paths.to_vec();
        sorted_paths.sort_by(|a, b| {
            a.quality.latency_ms.partial_cmp(&b.quality.latency_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted_paths
    }
    
    /// 基于带宽选择
    fn select_bandwidth_based(&self, paths: &[PathInfo]) -> Vec<PathInfo> {
        // 按带宽排序，选择带宽最高的
        let mut sorted_paths = paths.to_vec();
        sorted_paths.sort_by(|a, b| {
            b.quality.bandwidth_mbps.partial_cmp(&a.quality.bandwidth_mbps)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted_paths
    }
    
    /// 隐私感知选择
    fn select_privacy_aware(&self, paths: &[PathInfo], scored_paths: &[(PathInfo, PathScore)]) -> Vec<PathInfo> {
        // 优先选择隐私评分高的路径
        let mut sorted_paths = Vec::new();
        
        for (path, score) in scored_paths {
            if paths.contains(path) {
                sorted_paths.push((path.clone(), score.privacy_score));
            }
        }
        
        sorted_paths.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        sorted_paths.into_iter().map(|(path, _)| path).collect()
    }
    
    /// 故障转移
    pub fn failover(&self, target: &str, failed_path_id: &str) -> Result<PathInfo, PathSelectionError> {
        if !self.multipath_config.failover_enabled {
            return Err(PathSelectionError::PathEstablishmentFailed {
                reason: "故障转移已禁用".to_string(),
            });
        }
        
        // 标记路径为不健康
        let mut health = self.path_health.write();
        health.insert(failed_path_id.to_string(), false);
        
        // 选择替代路径
        let paths = self.available_paths.read();
        let target_paths = paths.get(target)
            .ok_or(PathSelectionError::NoPathsAvailable)?;
            
        // 寻找健康的替代路径
        let alternative_paths: Vec<&PathInfo> = target_paths
            .iter()
            .filter(|p| p.is_active && p.path_id != failed_path_id)
            .filter(|p| *health.get(&p.path_id).unwrap_or(&true))
            .collect();
            
        if alternative_paths.is_empty() {
            return Err(PathSelectionError::NoPathsAvailable);
        }
        
        // 选择评分最高的替代路径
        let mut scored_alternatives: Vec<(PathInfo, PathScore)> = alternative_paths
            .iter()
            .map(|path| {
                let score = self.calculate_path_score(path);
                ((*path).clone(), score)
            })
            .collect();
            
        scored_alternatives.sort_by(|a, b| {
            b.1.overall_score.partial_cmp(&a.1.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let best_alternative = &scored_alternatives[0].0;
        
        // 更新活动路径
        let mut active_paths = self.active_paths.write();
        active_paths.insert(target.to_string(), best_alternative.clone());
        
        Ok(best_alternative.clone())
    }
    
    /// 检查路径健康状态
    pub fn check_path_health(&self, path_id: &str) -> bool {
        let health = self.path_health.read();
        *health.get(path_id).unwrap_or(&true)
    }
    
    /// 更新路径健康状态
    pub fn update_path_health(&self, path_id: &str, is_healthy: bool) {
        let mut health = self.path_health.write();
        health.insert(path_id.to_string(), is_healthy);
    }
    
    /// 获取选择历史
    pub fn get_selection_history(&self) -> Vec<PathSelectionResult> {
        let history = self.selection_history.read();
        history.iter().cloned().collect()
    }
    
    /// 获取路径统计信息
    pub fn get_path_stats(&self) -> PathStats {
        let paths = self.available_paths.read();
        let total_paths: usize = paths.values().map(|v| v.len()).sum();
        let active_paths: usize = paths.values()
            .flat_map(|v| v.iter())
            .filter(|p| p.is_active)
            .count();
        let unique_targets = paths.len();
        
        PathStats {
            total_paths,
            active_paths,
            unique_targets,
            selection_history_count: self.selection_history.read().len(),
        }
    }
}

/// 路径统计信息
#[derive(Debug, Clone)]
pub struct PathStats {
    pub total_paths: usize,
    pub active_paths: usize,
    pub unique_targets: usize,
    pub selection_history_count: usize,
}

/// 路径发现器
pub struct PathDiscoverer {
    known_relays: Vec<String>,
    tor_nodes: Vec<String>,
    mixnet_nodes: Vec<String>,
}

impl PathDiscoverer {
    /// 创建新的路径发现器
    pub fn new() -> Self {
        Self {
            known_relays: Vec::new(),
            tor_nodes: Vec::new(),
            mixnet_nodes: Vec::new(),
        }
    }
    
    /// 发现可用路径
    pub async fn discover_paths(&self, target: &str) -> Result<Vec<PathInfo>, PathSelectionError> {
        let mut paths = Vec::new();
        
        // 1. 直接路径
        paths.push(self.create_direct_path(target));
        
        // 2. 单中继路径
        for relay in &self.known_relays {
            paths.push(self.create_single_relay_path(target, relay));
        }
        
        // 3. 多中继路径（如果至少有两个中继）
        if self.known_relays.len() >= 2 {
            for i in 0..self.known_relays.len() - 1 {
                for j in i + 1..self.known_relays.len() {
                    let relays = vec![
                        self.known_relays[i].clone(),
                        self.known_relays[j].clone(),
                    ];
                    paths.push(self.create_multi_relay_path(target, &relays));
                }
            }
        }
        
        // 4. Tor网络路径
        for tor_node in &self.tor_nodes {
            paths.push(self.create_tor_path(target, tor_node));
        }
        
        // 5. 混合网络路径
        for mixnet_node in &self.mixnet_nodes {
            paths.push(self.create_mixnet_path(target, mixnet_node));
        }
        
        Ok(paths)
    }
    
    /// 创建直接路径
    fn create_direct_path(&self, target: &str) -> PathInfo {
        PathInfo {
            path_id: format!("direct_{}", target),
            path_type: PathType::Direct,
            target: target.to_string(),
            hops: vec![target.to_string()],
            quality: ConnectionQuality {
                latency_ms: 30.0,
                bandwidth_mbps: 100.0,
                packet_loss_percent: 0.1,
                jitter_ms: 5.0,
                reliability: 0.95,
                stability: 0.9,
                last_updated: Instant::now(),
            },
            privacy_level: 0.3,
            performance_level: 0.9,
            established_at: Instant::now(),
            usage_count: 0,
            last_used: Instant::now(),
            is_active: true,
        }
    }
    
    /// 创建单中继路径
    fn create_single_relay_path(&self, target: &str, relay: &str) -> PathInfo {
        PathInfo {
            path_id: format!("relay_{}_{}", relay, target),
            path_type: PathType::SingleRelay,
            target: target.to_string(),
            hops: vec![relay.to_string(), target.to_string()],
            quality: ConnectionQuality {
                latency_ms: 80.0,
                bandwidth_mbps: 50.0,
                packet_loss_percent: 0.5,
                jitter_ms: 10.0,
                reliability: 0.85,
                stability: 0.8,
                last_updated: Instant::now(),
            },
            privacy_level: 0.6,
            performance_level: 0.7,
            established_at: Instant::now(),
            usage_count: 0,
            last_used: Instant::now(),
            is_active: true,
        }
    }
    
    /// 创建多中继路径
    fn create_multi_relay_path(&self, target: &str, relays: &[String]) -> PathInfo {
        let mut hops = relays.to_vec();
        hops.push(target.to_string());
        
        PathInfo {
            path_id: format!("multirelay_{}_{}", relays.join("_"), target),
            path_type: PathType::MultiRelay,
            target: target.to_string(),
            hops,
            quality: ConnectionQuality {
                latency_ms: 150.0,
                bandwidth_mbps: 30.0,
                packet_loss_percent: 1.0,
                jitter_ms: 20.0,
                reliability: 0.75,
                stability: 0.7,
                last_updated: Instant::now(),
            },
            privacy_level: 0.8,
            performance_level: 0.5,
            established_at: Instant::now(),
            usage_count: 0,
            last_used: Instant::now(),
            is_active: true,
        }
    }
    
    /// 创建Tor网络路径
    fn create_tor_path(&self, target: &str, tor_node: &str) -> PathInfo {
        PathInfo {
            path_id: format!("tor_{}_{}", tor_node, target),
            path_type: PathType::Tor,
            target: target.to_string(),
            hops: vec![tor_node.to_string(), target.to_string()],
            quality: ConnectionQuality {
                latency_ms: 300.0,
                bandwidth_mbps: 10.0,
                packet_loss_percent: 2.0,
                jitter_ms: 30.0,
                reliability: 0.7,
                stability: 0.6,
                last_updated: Instant::now(),
            },
            privacy_level: 0.95,
            performance_level: 0.3,
            established_at: Instant::now(),
            usage_count: 0,
            last_used: Instant::now(),
            is_active: true,
        }
    }
    
    /// 创建混合网络路径
    fn create_mixnet_path(&self, target: &str, mixnet_node: &str) -> PathInfo {
        PathInfo {
            path_id: format!("mixnet_{}_{}", mixnet_node, target),
            path_type: PathType::Mixnet,
            target: target.to_string(),
            hops: vec![mixnet_node.to_string(), target.to_string()],
            quality: ConnectionQuality {
                latency_ms: 200.0,
                bandwidth_mbps: 20.0,
                packet_loss_percent: 1.5,
                jitter_ms: 25.0,
                reliability: 0.8,
                stability: 0.75,
                last_updated: Instant::now(),
            },
            privacy_level: 0.9,
            performance_level: 0.4,
            established_at: Instant::now(),
            usage_count: 0,
            last_used: Instant::now(),
            is_active: true,
        }
    }
    
    /// 添加已知中继
    pub fn add_relay(&mut self, relay: &str) {
        self.known_relays.push(relay.to_string());
    }
    
    /// 添加Tor节点
    pub fn add_tor_node(&mut self, tor_node: &str) {
        self.tor_nodes.push(tor_node.to_string());
    }
    
    /// 添加混合网络节点
    pub fn add_mixnet_node(&mut self, mixnet_node: &str) {
        self.mixnet_nodes.push(mixnet_node.to_string());
    }
}

/// 路径管理器
pub struct PathManager {
    selector: PrivacyPathSelector,
    discoverer: PathDiscoverer,
    quality_analyzer: Arc<crate::network::routing::quality::ConnectionQualityAnalyzer>,
}

impl PathManager {
    /// 创建新的路径管理器
    pub fn new(config: PrivacyPerformanceConfig) -> Self {
        Self {
            selector: PrivacyPathSelector::new(config),
            discoverer: PathDiscoverer::new(),
            quality_analyzer: Arc::new(
                crate::network::routing::quality::ConnectionQualityAnalyzer::new(100)
            ),
        }
    }
    
    /// 发现并选择路径
    pub async fn discover_and_select_path(&self, target: &str) -> Result<PathInfo, PathSelectionError> {
        // 发现可用路径
        let paths = self.discoverer.discover_paths(target).await?;
        
        // 添加到选择器
        for path in paths {
            self.selector.add_path(target, path);
        }
        
        // 选择最佳路径
        self.selector.select_best_path(target)
    }
    
    /// 更新路径质量
    pub fn update_path_quality(&self, _path_id: &str, quality: ConnectionQuality) {
        // 在实际实现中，这里会更新对应路径的质量信息
        // 目前先记录到质量分析器
        self.quality_analyzer.update_quality(quality);
    }
    
    /// 获取路径管理器统计信息
    pub fn get_manager_stats(&self) -> ManagerStats {
        let path_stats = self.selector.get_path_stats();
        
        ManagerStats {
            total_paths: path_stats.total_paths,
            active_paths: path_stats.active_paths,
            unique_targets: path_stats.unique_targets,
            selection_count: path_stats.selection_history_count,
            known_relays: self.discoverer.known_relays.len(),
            known_tor_nodes: self.discoverer.tor_nodes.len(),
            known_mixnet_nodes: self.discoverer.mixnet_nodes.len(),
        }
    }
}

/// 管理器统计信息
#[derive(Debug, Clone)]
pub struct ManagerStats {
    pub total_paths: usize,
    pub active_paths: usize,
    pub unique_targets: usize,
    pub selection_count: usize,
    pub known_relays: usize,
    pub known_tor_nodes: usize,
    pub known_mixnet_nodes: usize,
}

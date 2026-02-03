//! 智能路由系统模块
//! 
//! 提供智能路由选择、性能监控和隐私分析功能

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use anyhow::Result;

/// 路由类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    Direct,      // 直接连接
    Relay,       // 中继连接
    Proxy,       // 代理连接
    Tor,         // Tor网络
    Mixed,       // 混合路由
}

/// 路由质量指标
#[derive(Debug, Clone)]
pub struct RouteQuality {
    pub latency_ms: f32,
    pub bandwidth_mbps: f32,
    pub packet_loss_percent: f32,
    pub jitter_ms: f32,
    pub reliability: f32,  // 0.0-1.0
    pub last_updated: Instant,
}

/// 路由评分
#[derive(Debug, Clone)]
pub struct RouteScore {
    pub performance_score: f32,  // 0.0-1.0
    pub privacy_score: f32,      // 0.0-1.0
    pub cost_score: f32,         // 0.0-1.0（成本/资源消耗）
    pub overall_score: f32,      // 加权总分
}

/// 路由信息
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub route_type: RouteType,
    pub target: String,
    pub next_hop: Option<String>,
    pub quality: RouteQuality,
    pub score: RouteScore,
    pub established_at: Instant,
    pub usage_count: u64,
}

/// 路由选择记录
#[derive(Debug, Clone)]
pub struct RouteSelection {
    pub timestamp: Instant,
    pub target: String,
    pub selected_route: RouteInfo,
    pub alternative_routes: Vec<RouteInfo>,
    pub selection_reason: String,
}

/// 路由错误类型
#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("无可用路由")]
    NoRoutesAvailable,
    
    #[error("路由发现失败: {0}")]
    DiscoveryFailed(String),
    
    #[error("路由质量过低")]
    PoorRouteQuality,
    
    #[error("隐私要求不满足")]
    PrivacyRequirementNotMet,
    
    #[error("性能要求不满足")]
    PerformanceRequirementNotMet,
    
    #[error("路由建立失败: {0}")]
    RouteEstablishmentFailed(String),
}

/// 性能监控器
pub(crate) struct PerformanceMonitor {
    latency_history: VecDeque<f32>,
    bandwidth_history: VecDeque<f32>,
    packet_loss_history: VecDeque<f32>,
}

impl PerformanceMonitor {
    pub(crate) fn new() -> Self {
        Self {
            latency_history: VecDeque::with_capacity(100),
            bandwidth_history: VecDeque::with_capacity(100),
            packet_loss_history: VecDeque::with_capacity(100),
        }
    }
    
    pub(crate) fn update(&mut self, latency: f32, bandwidth: f32, packet_loss: f32) {
        self.latency_history.push_back(latency);
        self.bandwidth_history.push_back(bandwidth);
        self.packet_loss_history.push_back(packet_loss);
        
        // 保持历史数据大小
        if self.latency_history.len() > 100 {
            self.latency_history.pop_front();
        }
        if self.bandwidth_history.len() > 100 {
            self.bandwidth_history.pop_front();
        }
        if self.packet_loss_history.len() > 100 {
            self.packet_loss_history.pop_front();
        }
    }
    
    pub(crate) fn get_average_latency(&self) -> f32 {
        if self.latency_history.is_empty() {
            return 50.0; // 默认值
        }
        self.latency_history.iter().sum::<f32>() / self.latency_history.len() as f32
    }
    
    pub(crate) fn get_average_bandwidth(&self) -> f32 {
        if self.bandwidth_history.is_empty() {
            return 10.0; // 默认10Mbps
        }
        self.bandwidth_history.iter().sum::<f32>() / self.bandwidth_history.len() as f32
    }
}

/// 隐私分析器
pub(crate) struct PrivacyAnalyzer {
    ip_exposure_history: VecDeque<bool>,
    traffic_analysis_resistance: VecDeque<f32>,
}

impl PrivacyAnalyzer {
    pub(crate) fn new() -> Self {
        Self {
            ip_exposure_history: VecDeque::with_capacity(100),
            traffic_analysis_resistance: VecDeque::with_capacity(100),
        }
    }
    
    pub(crate) fn analyze_route_privacy(&self, route_type: RouteType) -> f32 {
        match route_type {
            RouteType::Direct => 0.3,    // 直接连接隐私性低
            RouteType::Relay => 0.7,     // 中继连接中等隐私
            RouteType::Proxy => 0.8,     // 代理连接高隐私
            RouteType::Tor => 0.95,      // Tor网络极高隐私
            RouteType::Mixed => 0.6,     // 混合路由中等隐私
        }
    }
    
    pub(crate) fn update_analysis(&mut self, ip_exposed: bool, resistance: f32) {
        self.ip_exposure_history.push_back(ip_exposed);
        self.traffic_analysis_resistance.push_back(resistance);
        
        if self.ip_exposure_history.len() > 100 {
            self.ip_exposure_history.pop_front();
        }
        if self.traffic_analysis_resistance.len() > 100 {
            self.traffic_analysis_resistance.pop_front();
        }
    }
}

/// 智能路由引擎
pub struct SmartRoutingEngine {
    available_routes: RwLock<HashMap<String, Vec<RouteInfo>>>,
    performance_monitor: Arc<PerformanceMonitor>,
    privacy_analyzer: Arc<PrivacyAnalyzer>,
    selection_history: RwLock<VecDeque<RouteSelection>>,
    config: Arc<crate::config::PrivacyPerformanceConfig>,
}

impl SmartRoutingEngine {
    /// 创建新的智能路由引擎
    pub fn new(config: crate::config::PrivacyPerformanceConfig) -> Self {
        Self {
            available_routes: RwLock::new(HashMap::new()),
            performance_monitor: Arc::new(PerformanceMonitor::new()),
            privacy_analyzer: Arc::new(PrivacyAnalyzer::new()),
            selection_history: RwLock::new(VecDeque::with_capacity(100)),
            config: Arc::new(config),
        }
    }

    /// 添加可用路由
    pub fn add_route(&self, target: &str, route: RouteInfo) {
        let mut routes = self.available_routes.write();
        routes.entry(target.to_string())
            .or_insert_with(Vec::new)
            .push(route);
    }

    /// 移除路由
    pub fn remove_route(&self, target: &str, route_type: RouteType) {
        let mut routes = self.available_routes.write();
        if let Some(target_routes) = routes.get_mut(target) {
            target_routes.retain(|r| r.route_type != route_type);
        }
    }

    /// 选择最佳路由
    pub fn select_best_route(&self, target: &str) -> Result<RouteInfo, RoutingError> {
        let routes = self.available_routes.read();
        let target_routes = routes.get(target)
            .ok_or_else(|| RoutingError::NoRoutesAvailable)?;

        if target_routes.is_empty() {
            return Err(RoutingError::NoRoutesAvailable);
        }

        // 计算每个路由的评分
        let mut scored_routes: Vec<(RouteInfo, RouteScore)> = target_routes
            .iter()
            .map(|route| {
                let score = self.calculate_route_score(route);
                (route.clone(), score)
            })
            .collect();

        // 按总分排序
        scored_routes.sort_by(|a, b| {
            b.1.overall_score.partial_cmp(&a.1.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 检查是否满足最小要求
        let best_route = &scored_routes[0].0;
        let best_score = &scored_routes[0].1;

        if best_score.privacy_score < self.config.min_privacy_score {
            return Err(RoutingError::PrivacyRequirementNotMet);
        }

        if best_score.performance_score < self.config.min_performance_score {
            return Err(RoutingError::PerformanceRequirementNotMet);
        }

        // 记录选择历史
        let selection = RouteSelection {
            timestamp: Instant::now(),
            target: target.to_string(),
            selected_route: best_route.clone(),
            alternative_routes: scored_routes[1..].iter()
                .map(|(route, _)| route.clone())
                .collect(),
            selection_reason: format!("最佳评分: {:.2}", best_score.overall_score),
        };

        self.record_selection(selection);

        Ok(best_route.clone())
    }

    /// 计算路由评分
    fn calculate_route_score(&self, route: &RouteInfo) -> RouteScore {
        let performance_score = self.calculate_performance_score(&route.quality);
        let privacy_score = self.privacy_analyzer.analyze_route_privacy(route.route_type);
        let cost_score = self.calculate_cost_score(route.route_type);

        // 根据配置的权重计算总分
        let overall_score = 
            performance_score * self.config.performance_weight +
            privacy_score * (1.0 - self.config.performance_weight) +
            cost_score * 0.1; // 成本权重固定为0.1

        RouteScore {
            performance_score,
            privacy_score,
            cost_score,
            overall_score,
        }
    }

    /// 计算性能评分
    fn calculate_performance_score(&self, quality: &RouteQuality) -> f32 {
        let latency_score = if quality.latency_ms < 50.0 {
            1.0
        } else if quality.latency_ms < 100.0 {
            0.8
        } else if quality.latency_ms < 200.0 {
            0.6
        } else if quality.latency_ms < 500.0 {
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

        // 加权平均
        (latency_score * 0.4 + bandwidth_score * 0.4 + reliability_score * 0.2)
            .clamp(0.0, 1.0)
    }

    /// 计算成本评分（越低越好，所以用1.0减）
    fn calculate_cost_score(&self, route_type: RouteType) -> f32 {
        match route_type {
            RouteType::Direct => 0.9,    // 直接连接成本低
            RouteType::Relay => 0.7,     // 中继连接中等成本
            RouteType::Proxy => 0.5,     // 代理连接成本较高
            RouteType::Tor => 0.3,       // Tor网络成本高
            RouteType::Mixed => 0.6,     // 混合路由中等成本
        }
    }

    /// 记录路由选择
    fn record_selection(&self, selection: RouteSelection) {
        let mut history = self.selection_history.write();
        history.push_back(selection);
        
        if history.len() > 100 {
            history.pop_front();
        }
    }

    /// 获取选择历史
    pub fn get_selection_history(&self) -> Vec<RouteSelection> {
        let history = self.selection_history.read();
        history.iter().cloned().collect()
    }

    /// 更新性能监控
    pub fn update_performance(&self, latency: f32, bandwidth: f32, packet_loss: f32) {
        // 这里需要获取可变引用，所以需要内部可变性
        // 在实际实现中，可能需要使用Mutex或修改PerformanceMonitor为内部可变
        // 暂时留空，后续实现
    }

    /// 更新隐私分析
    pub fn update_privacy_analysis(&self, ip_exposed: bool, resistance: f32) {
        // 同上，需要内部可变性
        // 暂时留空，后续实现
    }

    /// 获取路由统计信息
    pub fn get_routing_stats(&self) -> RoutingStats {
        let routes = self.available_routes.read();
        let total_routes: usize = routes.values().map(|v| v.len()).sum();
        let unique_targets = routes.len();

        RoutingStats {
            total_routes,
            unique_targets,
            selection_history_count: self.selection_history.read().len(),
        }
    }
}

/// 路由统计信息
#[derive(Debug, Clone)]
pub struct RoutingStats {
    pub total_routes: usize,
    pub unique_targets: usize,
    pub selection_history_count: usize,
}

/// 路由发现器
pub struct RouteDiscoverer {
    known_relays: Vec<String>,
    proxy_servers: Vec<String>,
    tor_nodes: Vec<String>,
}

impl RouteDiscoverer {
    pub fn new() -> Self {
        Self {
            known_relays: Vec::new(),
            proxy_servers: Vec::new(),
            tor_nodes: Vec::new(),
        }
    }

    /// 发现可用路由
    pub async fn discover_routes(&self, target: &str) -> Result<Vec<RouteInfo>, RoutingError> {
        let mut routes = Vec::new();

        // 1. 直接连接
        routes.push(self.create_direct_route(target));

        // 2. 中继连接
        for relay in &self.known_relays {
            routes.push(self.create_relay_route(target, relay));
        }

        // 3. 代理连接
        for proxy in &self.proxy_servers {
            routes.push(self.create_proxy_route(target, proxy));
        }

        // 4. Tor网络连接
        for tor_node in &self.tor_nodes {
            routes.push(self.create_tor_route(target, tor_node));
        }

        Ok(routes)
    }

    fn create_direct_route(&self, target: &str) -> RouteInfo {
        RouteInfo {
            route_type: RouteType::Direct,
            target: target.to_string(),
            next_hop: None,
            quality: RouteQuality {
                latency_ms: 50.0,  // 默认值
                bandwidth_mbps: 100.0,
                packet_loss_percent: 0.1,
                jitter_ms: 5.0,
                reliability: 0.95,
                last_updated: Instant::now(),
            },
            score: RouteScore {
                performance_score: 0.8,
                privacy_score: 0.3,
                cost_score: 0.9,
                overall_score: 0.67,
            },
            established_at: Instant::now(),
            usage_count: 0,
        }
    }

    fn create_relay_route(&self, target: &str, relay: &str) -> RouteInfo {
        RouteInfo {
            route_type: RouteType::Relay,
            target: target.to_string(),
            next_hop: Some(relay.to_string()),
            quality: RouteQuality {
                latency_ms: 100.0,  // 中继会增加延迟
                bandwidth_mbps: 50.0,
                packet_loss_percent: 0.5,
                jitter_ms: 10.0,
                reliability: 0.85,
                last_updated: Instant::now(),
            },
            score: RouteScore {
                performance_score: 0.6,
                privacy_score: 0.7,
                cost_score: 0.7,
                overall_score: 0.67,
            },
            established_at: Instant::now(),
            usage_count: 0,
        }
    }

    fn create_proxy_route(&self, target: &str, proxy: &str) -> RouteInfo {
        RouteInfo {
            route_type: RouteType::Proxy,
            target: target.to_string(),
            next_hop: Some(proxy.to_string()),
            quality: RouteQuality {
                latency_ms: 150.0,
                bandwidth_mbps: 30.0,
                packet_loss_percent: 1.0,
                jitter_ms: 15.0,
                reliability: 0.8,
                last_updated: Instant::now(),
            },
            score: RouteScore {
                performance_score: 0.5,
                privacy_score: 0.8,
                cost_score: 0.5,
                overall_score: 0.65,
            },
            established_at: Instant::now(),
            usage_count: 0,
        }
    }

    fn create_tor_route(&self, target: &str, tor_node: &str) -> RouteInfo {
        RouteInfo {
            route_type: RouteType::Tor,
            target: target.to_string(),
            next_hop: Some(tor_node.to_string()),
            quality: RouteQuality {
                latency_ms: 300.0,  // Tor网络延迟较高
                bandwidth_mbps: 10.0,
                packet_loss_percent: 2.0,
                jitter_ms: 30.0,
                reliability: 0.7,
                last_updated: Instant::now(),
            },
            score: RouteScore {
                performance_score: 0.3,
                privacy_score: 0.95,
                cost_score: 0.3,
                overall_score: 0.62,
            },
            established_at: Instant::now(),
            usage_count: 0,
        }
    }

    /// 添加已知中继
    pub fn add_relay(&mut self, relay: &str) {
        self.known_relays.push(relay.to_string());
    }

    /// 添加代理服务器
    pub fn add_proxy(&mut self, proxy: &str) {
        self.proxy_servers.push(proxy.to_string());
    }

    /// 添加Tor节点
    pub fn add_tor_node(&mut self, tor_node: &str) {
        self.tor_nodes.push(tor_node.to_string());
    }
}
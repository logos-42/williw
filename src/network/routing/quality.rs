//! 连接质量分析器模块
//! 
//! 提供连接质量监控、网络条件分析和性能趋势预测功能

use std::collections::VecDeque;
use std::time::{Instant, Duration};
use parking_lot::RwLock;

/// 连接质量指标
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionQuality {
    /// 延迟（毫秒）
    pub latency_ms: f32,
    /// 带宽（Mbps）
    pub bandwidth_mbps: f32,
    /// 丢包率（百分比）
    pub packet_loss_percent: f32,
    /// 抖动（毫秒）
    pub jitter_ms: f32,
    /// 可靠性评分（0.0-1.0）
    pub reliability: f32,
    /// 连接稳定性评分（0.0-1.0）
    pub stability: f32,
    /// 最后更新时间
    pub last_updated: Instant,
}

/// 网络条件
#[derive(Debug, Clone)]
pub struct NetworkConditions {
    /// 网络类型
    pub network_type: NetworkType,
    /// 信号强度（0.0-1.0，仅适用于无线网络）
    pub signal_strength: Option<f32>,
    /// 网络拥塞程度（0.0-1.0）
    pub congestion_level: f32,
    /// 网络可用性（0.0-1.0）
    pub availability: f32,
}

/// 网络类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    Ethernet,
    WiFi,
    Cellular4G,
    Cellular5G,
    Satellite,
    Unknown,
}

/// 性能趋势
#[derive(Debug, Clone)]
pub struct PerformanceTrend {
    /// 趋势方向：正数表示改善，负数表示恶化
    pub trend_direction: f32,
    /// 趋势强度（0.0-1.0）
    pub trend_strength: f32,
    /// 预测的未来值
    pub predicted_value: f32,
    /// 置信度（0.0-1.0）
    pub confidence: f32,
}

/// 连接质量分析器
pub struct ConnectionQualityAnalyzer {
    /// 历史质量数据
    quality_history: RwLock<VecDeque<ConnectionQuality>>,
    /// 网络条件历史
    network_history: RwLock<VecDeque<NetworkConditions>>,
    /// 分析窗口大小
    window_size: usize,
    /// 最小样本数
    min_samples: usize,
}

impl ConnectionQualityAnalyzer {
    /// 创建新的连接质量分析器
    pub fn new(window_size: usize) -> Self {
        Self {
            quality_history: RwLock::new(VecDeque::with_capacity(window_size)),
            network_history: RwLock::new(VecDeque::with_capacity(window_size)),
            window_size,
            min_samples: 10,
        }
    }

    /// 更新连接质量数据
    pub fn update_quality(&self, quality: ConnectionQuality) {
        let mut history = self.quality_history.write();
        history.push_back(quality);
        
        // 保持窗口大小
        if history.len() > self.window_size {
            history.pop_front();
        }
    }

    /// 更新网络条件
    pub fn update_network_conditions(&self, conditions: NetworkConditions) {
        let mut history = self.network_history.write();
        history.push_back(conditions);
        
        // 保持窗口大小
        if history.len() > self.window_size {
            history.pop_front();
        }
    }

    /// 分析当前连接质量
    pub fn analyze_current_quality(&self) -> Option<ConnectionQuality> {
        let history = self.quality_history.read();
        history.back().cloned()
    }

    /// 获取平均连接质量
    pub fn get_average_quality(&self) -> Option<ConnectionQuality> {
        let history = self.quality_history.read();
        
        if history.len() < self.min_samples {
            return None;
        }

        let count = history.len() as f32;
        
        let avg_latency = history.iter().map(|q| q.latency_ms).sum::<f32>() / count;
        let avg_bandwidth = history.iter().map(|q| q.bandwidth_mbps).sum::<f32>() / count;
        let avg_packet_loss = history.iter().map(|q| q.packet_loss_percent).sum::<f32>() / count;
        let avg_jitter = history.iter().map(|q| q.jitter_ms).sum::<f32>() / count;
        let avg_reliability = history.iter().map(|q| q.reliability).sum::<f32>() / count;
        let avg_stability = history.iter().map(|q| q.stability).sum::<f32>() / count;

        Some(ConnectionQuality {
            latency_ms: avg_latency,
            bandwidth_mbps: avg_bandwidth,
            packet_loss_percent: avg_packet_loss,
            jitter_ms: avg_jitter,
            reliability: avg_reliability,
            stability: avg_stability,
            last_updated: Instant::now(),
        })
    }

    /// 分析性能趋势
    pub fn analyze_performance_trend(&self, metric: PerformanceMetric) -> Option<PerformanceTrend> {
        let history = self.quality_history.read();
        
        if history.len() < self.min_samples {
            return None;
        }

        // 提取指标值
        let values: Vec<f32> = history.iter()
            .map(|q| match metric {
                PerformanceMetric::Latency => q.latency_ms,
                PerformanceMetric::Bandwidth => q.bandwidth_mbps,
                PerformanceMetric::PacketLoss => q.packet_loss_percent,
                PerformanceMetric::Jitter => q.jitter_ms,
                PerformanceMetric::Reliability => q.reliability,
                PerformanceMetric::Stability => q.stability,
            })
            .collect();

        // 简单线性回归分析趋势
        let n = values.len() as f32;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = values.iter().sum::<f32>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, &y) in values.iter().enumerate() {
            let x = i as f32;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean).powi(2);
        }

        let slope = if denominator != 0.0 { numerator / denominator } else { 0.0 };
        
        // 计算趋势强度和置信度
        let trend_strength = slope.abs().min(1.0);
        let confidence = (history.len() as f32 / self.window_size as f32).min(1.0);

        // 预测下一个值
        let predicted_value = if slope != 0.0 {
            let last_value = values.last().unwrap();
            last_value + slope
        } else {
            y_mean
        };

        Some(PerformanceTrend {
            trend_direction: slope,
            trend_strength,
            predicted_value,
            confidence,
        })
    }

    /// 评估网络条件对性能的影响
    pub fn evaluate_network_impact(&self) -> NetworkImpact {
        let network_history = self.network_history.read();
        let quality_history = self.quality_history.read();

        if network_history.is_empty() || quality_history.is_empty() {
            return NetworkImpact::Unknown;
        }

        let current_network = network_history.back().unwrap();
        let current_quality = quality_history.back().unwrap();

        match current_network.network_type {
            NetworkType::Ethernet => {
                if current_quality.latency_ms < 20.0 && current_quality.bandwidth_mbps > 50.0 {
                    NetworkImpact::Positive
                } else {
                    NetworkImpact::Neutral
                }
            }
            NetworkType::WiFi => {
                if let Some(signal) = current_network.signal_strength {
                    if signal > 0.7 && current_quality.latency_ms < 50.0 {
                        NetworkImpact::Positive
                    } else if signal < 0.3 || current_quality.latency_ms > 100.0 {
                        NetworkImpact::Negative
                    } else {
                        NetworkImpact::Neutral
                    }
                } else {
                    NetworkImpact::Neutral
                }
            }
            NetworkType::Cellular5G => {
                if current_quality.latency_ms < 30.0 && current_quality.bandwidth_mbps > 100.0 {
                    NetworkImpact::Positive
                } else {
                    NetworkImpact::Neutral
                }
            }
            NetworkType::Cellular4G => {
                if current_quality.latency_ms < 50.0 && current_quality.bandwidth_mbps > 20.0 {
                    NetworkImpact::Neutral
                } else {
                    NetworkImpact::Negative
                }
            }
            NetworkType::Satellite => {
                NetworkImpact::Negative // 卫星网络通常延迟高
            }
            NetworkType::Unknown => NetworkImpact::Unknown,
        }
    }

    /// 生成连接质量报告
    pub fn generate_quality_report(&self) -> QualityReport {
        let current_quality = self.analyze_current_quality();
        let average_quality = self.get_average_quality();
        let latency_trend = self.analyze_performance_trend(PerformanceMetric::Latency);
        let bandwidth_trend = self.analyze_performance_trend(PerformanceMetric::Bandwidth);
        let network_impact = self.evaluate_network_impact();

        QualityReport {
            timestamp: Instant::now(),
            current_quality,
            average_quality,
            latency_trend,
            bandwidth_trend,
            network_impact,
            sample_count: self.quality_history.read().len(),
        }
    }

    /// 检查连接是否健康
    pub fn is_connection_healthy(&self) -> bool {
        let history = self.quality_history.read();
        
        if history.len() < self.min_samples {
            return true; // 样本不足时假设健康
        }

        // 检查最近几个样本
        let recent_samples: Vec<&ConnectionQuality> = history.iter()
            .rev()
            .take(5.min(history.len()))
            .collect();

        // 如果最近样本中有超过一半的可靠性低于0.7，则认为不健康
        let unhealthy_count = recent_samples.iter()
            .filter(|q| q.reliability < 0.7)
            .count();

        unhealthy_count < recent_samples.len() / 2
    }

    /// 获取历史数据统计
    pub fn get_statistics(&self) -> QualityStatistics {
        let history = self.quality_history.read();
        
        if history.is_empty() {
            return QualityStatistics::default();
        }

        let latencies: Vec<f32> = history.iter().map(|q| q.latency_ms).collect();
        let bandwidths: Vec<f32> = history.iter().map(|q| q.bandwidth_mbps).collect();
        let reliabilities: Vec<f32> = history.iter().map(|q| q.reliability).collect();

        QualityStatistics {
            min_latency: latencies.iter().cloned().fold(f32::INFINITY, f32::min),
            max_latency: latencies.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            avg_latency: latencies.iter().sum::<f32>() / latencies.len() as f32,
            min_bandwidth: bandwidths.iter().cloned().fold(f32::INFINITY, f32::min),
            max_bandwidth: bandwidths.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            avg_bandwidth: bandwidths.iter().sum::<f32>() / bandwidths.len() as f32,
            min_reliability: reliabilities.iter().cloned().fold(f32::INFINITY, f32::min),
            max_reliability: reliabilities.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            avg_reliability: reliabilities.iter().sum::<f32>() / reliabilities.len() as f32,
            total_samples: history.len(),
        }
    }
}

/// 性能指标枚举
#[derive(Debug, Clone, Copy)]
pub enum PerformanceMetric {
    Latency,
    Bandwidth,
    PacketLoss,
    Jitter,
    Reliability,
    Stability,
}

/// 网络影响评估
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkImpact {
    Positive,   // 网络条件对性能有正面影响
    Neutral,    // 网络条件对性能无显著影响
    Negative,   // 网络条件对性能有负面影响
    Unknown,    // 无法评估
}

/// 连接质量报告
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub timestamp: Instant,
    pub current_quality: Option<ConnectionQuality>,
    pub average_quality: Option<ConnectionQuality>,
    pub latency_trend: Option<PerformanceTrend>,
    pub bandwidth_trend: Option<PerformanceTrend>,
    pub network_impact: NetworkImpact,
    pub sample_count: usize,
}

/// 质量统计信息
#[derive(Debug, Clone)]
pub struct QualityStatistics {
    pub min_latency: f32,
    pub max_latency: f32,
    pub avg_latency: f32,
    pub min_bandwidth: f32,
    pub max_bandwidth: f32,
    pub avg_bandwidth: f32,
    pub min_reliability: f32,
    pub max_reliability: f32,
    pub avg_reliability: f32,
    pub total_samples: usize,
}

impl Default for QualityStatistics {
    fn default() -> Self {
        Self {
            min_latency: 0.0,
            max_latency: 0.0,
            avg_latency: 0.0,
            min_bandwidth: 0.0,
            max_bandwidth: 0.0,
            avg_bandwidth: 0.0,
            min_reliability: 0.0,
            max_reliability: 0.0,
            avg_reliability: 0.0,
            total_samples: 0,
        }
    }
}

/// 网络探测器
pub struct NetworkProbe {
    /// 探测间隔（秒）
    probe_interval: Duration,
    /// 最后探测时间
    last_probe: Instant,
}

impl NetworkProbe {
    /// 创建新的网络探测器
    pub fn new(probe_interval_secs: u64) -> Self {
        Self {
            probe_interval: Duration::from_secs(probe_interval_secs),
            last_probe: Instant::now(),
        }
    }

    /// 执行网络探测
    pub fn probe_network(&mut self) -> NetworkConditions {
        self.last_probe = Instant::now();
        
        // 在实际实现中，这里会执行实际的网络探测
        // 目前返回模拟数据
        NetworkConditions {
            network_type: NetworkType::Unknown,
            signal_strength: None,
            congestion_level: 0.3, // 假设中等拥塞
            availability: 0.95,    // 假设95%可用性
        }
    }

    /// 检查是否需要执行探测
    pub fn should_probe(&self) -> bool {
        Instant::now().duration_since(self.last_probe) >= self.probe_interval
    }
}

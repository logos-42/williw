use crate::comms::{CommsConfig, BandwidthBudgetConfig};
use crate::consensus::ConsensusConfig;
use crate::crypto::CryptoConfig;
use crate::device::{DeviceCapabilities, DeviceManager};
// use crate::topology::TopologyConfig;
// use iroh::NodeAddr;  // 注释掉，因为API可能已改变
use serde::{Deserialize, Serialize};

/// 训练配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// 模型维度
    pub model_dim: usize,
    /// 学习率
    pub learning_rate: f64,
    /// 批量大小
    pub batch_size: usize,
    /// 训练轮数
    pub epochs: u32,
    /// 是否启用分布式训练
    pub enable_distributed: bool,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            model_dim: 768,
            learning_rate: 0.001,
            batch_size: 32,
            epochs: 10,
            enable_distributed: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub hide_ip: bool,
    pub use_relay: bool,
    pub relay_nodes: Vec<String>,  // 用字符串代替NodeAddr
    pub private_network_key: Option<String>,
    pub max_hops: u8,
    pub enable_autonat: bool,
    pub enable_dcutr: bool,
    // 新增：隐私-性能平衡配置
    pub privacy_performance: PrivacyPerformanceConfig,
}

/// 隐私-性能平衡配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyPerformanceConfig {
    /// 平衡模式：performance（性能优先）、balanced（平衡）、privacy（隐私优先）、adaptive（自适应）
    pub mode: BalanceMode,
    /// 性能权重（0.0-1.0），隐私权重 = 1.0 - performance_weight
    pub performance_weight: f32,
    /// 是否启用硬件加速加密
    pub enable_hardware_acceleration: bool,
    /// 连接池大小
    pub connection_pool_size: usize,
    /// 是否启用0-RTT连接
    pub enable_0rtt: bool,
    /// 拥塞控制算法
    pub congestion_control: CongestionControlAlgorithm,
    /// 路由选择策略
    pub routing_strategy: RoutingStrategy,
    /// 最小隐私评分要求（0.0-1.0）
    pub min_privacy_score: f32,
    /// 最小性能评分要求（0.0-1.0）
    pub min_performance_score: f32,
    /// 是否允许回退到直接连接
    pub fallback_to_direct: bool,
    /// 性能监控间隔（秒）
    pub monitoring_interval_secs: u64,
}

/// 平衡模式枚举
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum BalanceMode {
    Performance,
    Balanced,
    Privacy,
    Adaptive,
}

/// 拥塞控制算法枚举
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum CongestionControlAlgorithm {
    Bbr,
    Cubic,
    Reno,
}

/// 路由选择策略枚举
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RoutingStrategy {
    SmartBalance,      // 智能平衡
    PrivacyFirst,      // 隐私优先
    PerformanceFirst,  // 性能优先
    LatencyBased,      // 延迟优先
    BandwidthBased,    // 带宽优先
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            hide_ip: false,  // 默认不隐藏IP，保持向后兼容
            use_relay: false,
            relay_nodes: Vec::new(),
            private_network_key: None,
            max_hops: 3,
            enable_autonat: true,
            enable_dcutr: true,
            privacy_performance: PrivacyPerformanceConfig::default(),
        }
    }
}

impl Default for PrivacyPerformanceConfig {
    fn default() -> Self {
        Self {
            mode: BalanceMode::Balanced,
            performance_weight: 0.6,
            enable_hardware_acceleration: true,
            connection_pool_size: 10,
            enable_0rtt: true,
            congestion_control: CongestionControlAlgorithm::Bbr,
            routing_strategy: RoutingStrategy::SmartBalance,
            min_privacy_score: 0.7,
            min_performance_score: 0.8,
            fallback_to_direct: true,
            monitoring_interval_secs: 30,
        }
    }
}

impl PrivacyPerformanceConfig {
    /// 验证隐私-性能平衡配置的合理性
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // 验证性能权重
        if self.performance_weight < 0.0 || self.performance_weight > 1.0 {
            errors.push(format!("性能权重必须在0.0-1.0之间，当前值: {}", self.performance_weight));
        }
        
        // 验证连接池大小
        if self.connection_pool_size == 0 {
            errors.push("连接池大小必须大于0".to_string());
        }
        if self.connection_pool_size > 100 {
            errors.push(format!("连接池大小过大: {}，建议不超过100", self.connection_pool_size));
        }
        
        // 验证监控间隔
        if self.monitoring_interval_secs == 0 {
            errors.push("性能监控间隔必须大于0秒".to_string());
        }
        if self.monitoring_interval_secs > 300 {
            errors.push(format!("性能监控间隔过长: {}秒，建议不超过300秒", self.monitoring_interval_secs));
        }
        
        // 验证评分要求
        if self.min_privacy_score < 0.0 || self.min_privacy_score > 1.0 {
            errors.push(format!("最小隐私评分必须在0.0-1.0之间，当前值: {}", self.min_privacy_score));
        }
        
        if self.min_performance_score < 0.0 || self.min_performance_score > 1.0 {
            errors.push(format!("最小性能评分必须在0.0-1.0之间，当前值: {}", self.min_performance_score));
        }
        
        // 自适应模式特定验证
        if let BalanceMode::Adaptive = self.mode {
            if self.performance_weight != 0.6 {
                errors.push("自适应模式下性能权重应保持默认值0.6，系统会自动调整".to_string());
            }
        }
        
        // 隐私优先模式验证
        if let BalanceMode::Privacy = self.mode {
            if self.performance_weight > 0.4 {
                errors.push("隐私优先模式下性能权重应小于等于0.4".to_string());
            }
            if !self.fallback_to_direct {
                errors.push("隐私优先模式下应禁用回退到直接连接".to_string());
            }
        }
        
        // 性能优先模式验证
        if let BalanceMode::Performance = self.mode {
            if self.performance_weight < 0.7 {
                errors.push("性能优先模式下性能权重应大于等于0.7".to_string());
            }
            if !self.enable_hardware_acceleration {
                errors.push("性能优先模式下应启用硬件加速".to_string());
            }
            if !self.enable_0rtt {
                errors.push("性能优先模式下应启用0-RTT连接".to_string());
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// 获取配置建议
    pub fn get_config_advice(&self) -> Vec<String> {
        let mut advice = Vec::new();
        
        match self.mode {
            BalanceMode::Performance => {
                advice.push("模式: 性能优先".to_string());
                advice.push("建议: 适用于对延迟敏感的应用，如实时通信".to_string());
                if !self.enable_hardware_acceleration {
                    advice.push("警告: 性能模式下建议启用硬件加速".to_string());
                }
            }
            BalanceMode::Balanced => {
                advice.push("模式: 平衡".to_string());
                advice.push("建议: 适用于大多数场景，在隐私和性能之间取得平衡".to_string());
            }
            BalanceMode::Privacy => {
                advice.push("模式: 隐私优先".to_string());
                advice.push("建议: 适用于高隐私要求的场景，如敏感数据传输".to_string());
                if self.fallback_to_direct {
                    advice.push("警告: 隐私模式下建议禁用回退到直接连接".to_string());
                }
            }
            BalanceMode::Adaptive => {
                advice.push("模式: 自适应".to_string());
                advice.push("建议: 系统会根据实时条件自动调整平衡".to_string());
                advice.push("注意: 需要足够的运行时间以学习最佳策略".to_string());
            }
        }
        
        // 硬件加速建议
        if self.enable_hardware_acceleration {
            advice.push("✓ 硬件加速已启用".to_string());
        } else {
            advice.push("⚠ 硬件加速未启用，可能影响性能".to_string());
        }
        
        // 0-RTT建议
        if self.enable_0rtt {
            advice.push("✓ 0-RTT连接已启用".to_string());
        } else {
            advice.push("⚠ 0-RTT连接未启用，可能增加连接延迟".to_string());
        }
        
        // 路由策略建议
        match self.routing_strategy {
            RoutingStrategy::SmartBalance => {
                advice.push("路由策略: 智能平衡".to_string());
            }
            RoutingStrategy::PrivacyFirst => {
                advice.push("路由策略: 隐私优先".to_string());
            }
            RoutingStrategy::PerformanceFirst => {
                advice.push("路由策略: 性能优先".to_string());
            }
            RoutingStrategy::LatencyBased => {
                advice.push("路由策略: 延迟优先".to_string());
            }
            RoutingStrategy::BandwidthBased => {
                advice.push("路由策略: 带宽优先".to_string());
            }
        }
        
        advice
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // pub inference: InferenceConfig,
    pub comms: CommsConfig,
    // pub topology: TopologyConfig,
    pub crypto: CryptoConfig,
    pub consensus: ConsensusConfig,
    // 移除DeviceManager的序列化，改为存储DeviceCapabilities
    pub device_capabilities: DeviceCapabilities,
    pub security: SecurityConfig,
    pub training: TrainingConfig,
}

impl AppConfig {
    /// 根据设备能力自动调整配置
    pub fn from_device_capabilities(capabilities: DeviceCapabilities) -> Self {
        // 如果检测失败，使用默认配置作为回退
        let capabilities = if capabilities.max_memory_mb == 0 || capabilities.cpu_cores == 0 {
            println!("[警告] 设备检测失败，使用默认配置");
            DeviceCapabilities::default()
        } else {
            capabilities
        };
        let network_type = capabilities.network_type;

        // 根据网络类型调整带宽预算
        let bandwidth_factor = network_type.bandwidth_factor();
        let comms = CommsConfig {
            topic: "ggb-training".into(),
            // 使用随机可用端口监听
            listen_addr: Some("0.0.0.0:0".parse().unwrap()),
            quic_bind: Some(std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                9234,
            )),
            quic_bootstrap: Vec::new(),
            bandwidth: BandwidthBudgetConfig {
                sparse_per_window: (12.0 * bandwidth_factor) as u32,
                dense_bytes_per_window: ((256 * 1024) as f32 * bandwidth_factor) as usize,
                window_secs: 60,
            },
            enable_dht: true,
            bootstrap_peers_file: Some(std::path::PathBuf::from("bootstrap_peers.txt")),
            security: SecurityConfig::default(),
        };

        Self {
            comms,
            crypto: CryptoConfig::default(),
            consensus: ConsensusConfig::default(),
            device_capabilities: capabilities,
            security: SecurityConfig::default(),
            training: TrainingConfig::default(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let device_manager = DeviceManager::new();
        let capabilities = device_manager.get();

        Self {
            comms: CommsConfig::default(),
            crypto: CryptoConfig::default(),
            consensus: ConsensusConfig::default(),
            device_capabilities: capabilities,
            security: SecurityConfig::default(),
            training: TrainingConfig::default(),
        }
    }
}

impl SecurityConfig {
    /// 验证安全配置的合理性
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // 检查IP隐藏配置
        if self.hide_ip {
            if self.use_relay && self.relay_nodes.is_empty() {
                errors.push("启用IP隐藏和中继时，必须提供至少一个中继节点".to_string());
            }
            
            if !self.use_relay {
                errors.push("警告：启用IP隐藏但未使用中继，隐私保护可能不完整".to_string());
            }
            
            if self.enable_dcutr {
                errors.push("警告：启用IP隐藏时使用DCUtR可能暴露IP".to_string());
            }
        }
        
        // 检查中继跳数
        if self.max_hops == 0 || self.max_hops > 5 {
            errors.push("中继跳数必须在1-5之间".to_string());
        }
        
        // 验证隐私-性能平衡配置
        if let Err(pp_errors) = self.privacy_performance.validate() {
            errors.extend(pp_errors);
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// 获取隐私建议
    pub fn get_privacy_advice(&self) -> Vec<String> {
        let mut advice = Vec::new();
        
        if self.hide_ip {
            advice.push("✓ IP隐藏已启用".to_string());
            
            if self.use_relay && !self.relay_nodes.is_empty() {
                advice.push(format!("✓ 使用 {} 个中继节点", self.relay_nodes.len()));
            } else if self.use_relay {
                advice.push("⚠ 启用中继但未配置中继节点".to_string());
            }
            
            if self.enable_dcutr {
                advice.push("⚠ DCUtR已启用，可能尝试建立直接连接".to_string());
            }
        } else {
            advice.push("⚠ IP隐藏未启用，节点IP可能暴露".to_string());
            advice.push("建议：设置 hide_ip = true 并配置中继节点".to_string());
        }
        
        advice
    }
}

impl AppConfig {
    /// 从TOML文件加载配置
    pub fn from_toml_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        
        // 验证配置
        if let Err(errors) = config.security.validate() {
            println!("[配置验证] 发现配置问题：");
            for error in &errors {
                println!("  - {}", error);
            }
        }
        
        // 显示隐私建议
        let advice = config.security.get_privacy_advice();
        if !advice.is_empty() {
            println!("[隐私配置] 当前设置：");
            for item in advice {
                println!("  {}", item);
            }
        }
        
        // 显示隐私-性能平衡配置信息
        config.display_privacy_performance_info();
        
        Ok(config)
    }
    
    /// 加载预设配置模板
    pub fn from_preset(preset_name: &str) -> anyhow::Result<Self> {
        let config_path = match preset_name {
            "balanced" => "config/balanced_privacy.toml",
            "high_performance" => "config/high_performance_privacy.toml",
            "adaptive" => "config/adaptive_balance.toml",
            "privacy_example" => "config/privacy_example.toml",
            _ => return Err(anyhow::anyhow!("未知的预设配置: {}", preset_name)),
        };
        
        println!("[配置] 加载预设配置: {} -> {}", preset_name, config_path);
        Self::from_toml_file(std::path::Path::new(config_path))
    }
    
    /// 显示隐私-性能平衡配置信息
    pub fn display_privacy_performance_info(&self) {
        let pp_config = &self.security.privacy_performance;
        
        println!("[隐私-性能平衡] 配置信息：");
        println!("  模式: {:?}", pp_config.mode);
        println!("  性能权重: {:.1}", pp_config.performance_weight);
        println!("  隐私权重: {:.1}", 1.0 - pp_config.performance_weight);
        println!("  硬件加速: {}", pp_config.enable_hardware_acceleration);
        println!("  连接池大小: {}", pp_config.connection_pool_size);
        println!("  0-RTT连接: {}", pp_config.enable_0rtt);
        println!("  拥塞控制: {:?}", pp_config.congestion_control);
        println!("  路由策略: {:?}", pp_config.routing_strategy);
        println!("  最小隐私评分: {:.1}", pp_config.min_privacy_score);
        println!("  最小性能评分: {:.1}", pp_config.min_performance_score);
        println!("  允许回退到直接连接: {}", pp_config.fallback_to_direct);
        
        // 根据模式给出建议
        match pp_config.mode {
            BalanceMode::Performance => {
                println!("  [建议] 性能优先模式：适用于对延迟敏感的应用");
            }
            BalanceMode::Balanced => {
                println!("  [建议] 平衡模式：在隐私和性能之间取得平衡");
            }
            BalanceMode::Privacy => {
                println!("  [建议] 隐私优先模式：适用于高隐私要求的场景");
            }
            BalanceMode::Adaptive => {
                println!("  [建议] 自适应模式：根据实时条件动态调整");
            }
        }
    }
    
    /// 验证整个应用配置
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // 验证安全配置
        if let Err(security_errors) = self.security.validate() {
            errors.extend(security_errors);
        }
        
        // 验证通信配置一致性
        if self.security.hide_ip && self.comms.enable_dht {
            errors.push("启用IP隐藏时应禁用公共DHT (enable_dht = false)".to_string());
        }
        
        if self.security.use_relay && self.comms.security.relay_nodes.is_empty() {
            errors.push("启用中继但未配置中继节点".to_string());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
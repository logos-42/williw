//! 加密模块 - 提供高性能的加密功能
//!
//! 本模块包含：
//! 1. Solana 基础加密功能（签名、验证）
//! 2. 高性能加密引擎
//! 3. 硬件加速加密
//! 4. 批量加密
//! 5. 零拷贝加密

// 导出子模块
pub mod base;
pub mod high_performance;
// pub mod batch;
pub mod hardware;
pub mod zero_copy;

// 重新导出常用类型
pub use base::*;
pub use high_performance::*;
// pub use batch::*;
pub use hardware::*;
pub use zero_copy::*;

/// 隐私级别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PrivacyLevel {
    /// 性能优先：最小化加密
    Performance,
    /// 平衡模式：选择性加密
    Balanced,
    /// 最大隐私：完整加密+混淆
    Maximum,
}

/// 加密算法枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EncryptionAlgorithm {
    /// ChaCha20-Poly1305 (移动设备友好)
    ChaCha20Poly1305,
    /// AES-256-CBC (硬件加速)
    Aes256Cbc,
    /// Blake3 哈希加密
    Blake3,
}

/// 性能指标结构体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceMetrics {
    /// 加密延迟（毫秒）
    pub encryption_latency_ms: f64,
    /// 解密延迟（毫秒）
    pub decryption_latency_ms: f64,
    /// 吞吐量（MB/s）
    pub throughput_mbps: f64,
    /// CPU 使用率（%）
    pub cpu_usage_percent: f64,
    /// 内存使用（MB）
    pub memory_usage_mb: f64,
}

/// 隐私性能指标结构体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrivacyPerformanceMetrics {
    /// 加密开销（毫秒）
    pub encryption_overhead_ms: f64,
    /// 带宽开销百分比
    pub bandwidth_overhead_percent: f64,
    /// 隐私评分（0-1）
    pub privacy_score: f64,
    /// 性能评分（0-1）
    pub performance_score: f64,
}

/// 性能比较结构体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceComparison {
    pub original_quic_latency_ms: f64,
    pub privacy_enhanced_latency_ms: f64,
    pub original_quic_throughput_mbps: f64,
    pub privacy_enhanced_throughput_mbps: f64,
    pub privacy_protection_level: f64,
}

/// 平衡模式枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BalanceMode {
    PerformancePriority,
    Balanced,
    PrivacyPriority,
}

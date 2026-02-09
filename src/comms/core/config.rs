//! 通信配置模块
//!
//! 包含通信相关的配置结构和带宽预算管理

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// 通信配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommsConfig {
    pub topic: String,
    pub listen_addr: Option<SocketAddr>,
    pub quic_bind: Option<SocketAddr>,
    pub quic_bootstrap: Vec<SocketAddr>,
    pub bandwidth: BandwidthBudgetConfig,
    pub enable_dht: bool,
    pub bootstrap_peers_file: Option<PathBuf>,
    pub security: crate::config::SecurityConfig,
}

impl Default for CommsConfig {
    fn default() -> Self {
        Self {
            topic: "ggb-training".into(),
            listen_addr: None,
            quic_bind: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 9234)),
            quic_bootstrap: Vec::new(),
            bandwidth: BandwidthBudgetConfig::default(),
            enable_dht: true,
            bootstrap_peers_file: None,
            security: crate::config::SecurityConfig::default(),
        }
    }
}

/// 带宽预算配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthBudgetConfig {
    pub sparse_per_window: u32,
    pub dense_bytes_per_window: usize,
    pub window_secs: u64,
}

impl Default for BandwidthBudgetConfig {
    fn default() -> Self {
        Self {
            sparse_per_window: 12,
            dense_bytes_per_window: 256 * 1024,
            window_secs: 60,
        }
    }
}

/// 带宽预算管理器
pub(crate) struct BandwidthBudget {
    config: BandwidthBudgetConfig,
    window_start: Instant,
    sparse_sent: u32,
    dense_sent: usize,
}

impl BandwidthBudget {
    pub(crate) fn new(config: BandwidthBudgetConfig) -> Self {
        Self {
            config,
            window_start: Instant::now(),
            sparse_sent: 0,
            dense_sent: 0,
        }
    }

    fn rotate(&mut self) {
        if self.window_start.elapsed() >= Duration::from_secs(self.config.window_secs) {
            self.window_start = Instant::now();
            self.sparse_sent = 0;
            self.dense_sent = 0;
        }
    }

    pub(crate) fn allow_sparse(&mut self) -> bool {
        self.rotate();
        if self.sparse_sent < self.config.sparse_per_window {
            self.sparse_sent += 1;
            true
        } else {
            false
        }
    }

    pub(crate) fn allow_dense(&mut self, bytes: usize) -> bool {
        self.rotate();
        if self.dense_sent + bytes <= self.config.dense_bytes_per_window {
            self.dense_sent += bytes;
            true
        } else {
            false
        }
    }
}

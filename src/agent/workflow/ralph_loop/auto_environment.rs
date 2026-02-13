//! Ralph Loop 自动环境配置模块
//!
//! 实现AI自动检测、配置和管理执行环境

use super::super::AsyncWorkflowExecutor;
use crate::comms::transport::iroh::IrohConnectionManager;
use crate::agent::context::{ContextEntry, ContextType};
use serde::{Serialize, Deserialize};
use serde_json::json;

/// 环境配置状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub is_configured: bool,
    pub gpu_available: bool,
    pub gpu_devices: Vec<GpuDeviceInfo>,
    pub python_environment: Option<String>,
    pub required_packages: Vec<String>,
    pub missing_packages: Vec<String>,
    pub system_resources: SystemResources,
    pub network_config: NetworkConfig,
    pub node_id: Option<String>,
    pub peer_nodes: Vec<String>,
}

/// GPU设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDeviceInfo {
    pub index: u32,
    pub name: String,
    pub memory_mb: u64,
    pub compute_capability: String,
    pub is_available: bool,
}

/// 系统资源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub cpu_cores: u32,
    pub cpu_usage_percent: f32,
    pub disk_free_gb: f64,
}

/// 网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub is_online: bool,
    pub local_ip: Option<String>,
    pub public_ip: Option<String>,
    pub iroh_node_id: Option<String>,
    pub connected_peers: Vec<String>,
    pub bandwidth_mbps: f64,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            is_configured: false,
            gpu_available: false,
            gpu_devices: Vec::new(),
            python_environment: None,
            required_packages: vec![
                "torch".to_string(),
                "numpy".to_string(),
                "transformers".to_string(),
                "safetensors".to_string(),
            ],
            missing_packages: Vec::new(),
            system_resources: SystemResources {
                total_memory_mb: 0,
                available_memory_mb: 0,
                cpu_cores: 0,
                cpu_usage_percent: 0.0,
                disk_free_gb: 0.0,
            },
            network_config: NetworkConfig {
                is_online: false,
                local_ip: None,
                public_ip: None,
                iroh_node_id: None,
                connected_peers: Vec::new(),
                bandwidth_mbps: 0.0,
            },
            node_id: None,
            peer_nodes: Vec::new(),
        }
    }
}

impl AsyncWorkflowExecutor {
    /// 自动配置环境（Ralph Loop入口）
    pub async fn auto_configure_environment(
        &self,
        execution_id: &str,
        api_key: &str,
    ) -> Result<EnvironmentConfig, String> {
        println!("🔧 [AUTO-ENV] Starting automatic environment configuration for {}", execution_id);

        let mut config = EnvironmentConfig::default();

        // 1. 检测系统资源
        self.detect_system_resources(&mut config).await?;
        println!("📊 [AUTO-ENV] System resources detected: {:?}", config.system_resources);

        // 2. 检测GPU
        self.detect_gpu_devices(&mut config).await?;
        println!("🎮 [AUTO-ENV] GPU detection complete: {} devices found", config.gpu_devices.len());

        // 3. 检测Python环境
        self.detect_python_environment(&mut config).await?;
        println!("🐍 [AUTO-ENV] Python environment: {:?}", config.python_environment);

        // 4. 检查必要的包
        self.check_required_packages(&mut config).await?;
        println!("📦 [AUTO-ENV] Missing packages: {:?}", config.missing_packages);

        // 5. 配置网络（Iroh）
        self.configure_network(&mut config).await?;
        println!("🌐 [AUTO-ENV] Network configured, node_id: {:?}", config.node_id);

        // 6. 使用AI决定是否需要安装缺失的包
        if !config.missing_packages.is_empty() {
            let should_install = self.ai_decide_package_installation(&config, api_key).await?;
            if should_install {
                self.install_missing_packages(&mut config).await?;
            }
        }

        // 7. 发现对等节点（去中心化网络）
        self.discover_peer_nodes(&mut config).await?;
        println!("🔍 [AUTO-ENV] Discovered {} peer nodes", config.peer_nodes.len());

        config.is_configured = true;

        // 记录配置到执行历史
        self.record_environment_config(execution_id, &config).await;

        println!("✅ [AUTO-ENV] Environment configuration complete for {}", execution_id);
        Ok(config)
    }

    /// 检测系统资源
    async fn detect_system_resources(&self, config: &mut EnvironmentConfig) -> Result<(), String> {
        // 获取系统信息
        let sys_info = Self::get_system_info().await?;
        
        config.system_resources = SystemResources {
            total_memory_mb: sys_info.total_memory_mb,
            available_memory_mb: sys_info.available_memory_mb,
            cpu_cores: sys_info.cpu_cores,
            cpu_usage_percent: sys_info.cpu_usage_percent,
            disk_free_gb: sys_info.disk_free_gb,
        };

        Ok(())
    }

    /// 获取系统信息
    async fn get_system_info() -> Result<SystemInfo, String> {
        // 使用系统命令获取信息
        let mut sys_info = SystemInfo::default();

        // CPU核心数
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("sysctl")
                .args(["-n", "hw.ncpu"])
                .output() 
            {
                if let Ok(cores_str) = String::from_utf8(output.stdout) {
                    if let Ok(cores) = cores_str.trim().parse::<u32>() {
                        sys_info.cpu_cores = cores;
                    }
                }
            }

            // 内存信息
            if let Ok(output) = std::process::Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output() 
            {
                if let Ok(mem_str) = String::from_utf8(output.stdout) {
                    if let Ok(mem_bytes) = mem_str.trim().parse::<u64>() {
                        sys_info.total_memory_mb = mem_bytes / 1024 / 1024;
                    }
                }
            }

            // 磁盘空间
            if let Ok(output) = std::process::Command::new("df")
                .args(["-h", "/"])
                .output() 
            {
                // 解析df输出
                if let Ok(df_output) = String::from_utf8(output.stdout) {
                    for line in df_output.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            if let Some(size_str) = parts.get(3) {
                                sys_info.disk_free_gb = Self::parse_size_to_gb(size_str);
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // CPU核心数
            if let Ok(output) = std::process::Command::new("nproc")
                .output() 
            {
                if let Ok(cores_str) = String::from_utf8(output.stdout) {
                    if let Ok(cores) = cores_str.trim().parse::<u32>() {
                        sys_info.cpu_cores = cores;
                    }
                }
            }

            // 内存信息
            if let Ok(output) = std::process::Command::new("cat")
                .arg("/proc/meminfo")
                .output() 
            {
                if let Ok(meminfo) = String::from_utf8(output.stdout) {
                    for line in meminfo.lines() {
                        if line.starts_with("MemTotal:") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                if let Ok(kb) = parts[1].parse::<u64>() {
                                    sys_info.total_memory_mb = kb / 1024;
                                }
                            }
                        } else if line.starts_with("MemAvailable:") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                if let Ok(kb) = parts[1].parse::<u64>() {
                                    sys_info.available_memory_mb = kb / 1024;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(sys_info)
    }

    /// 解析大小字符串为GB
    fn parse_size_to_gb(size_str: &str) -> f64 {
        let size_str = size_str.trim();
        if size_str.ends_with('G') || size_str.ends_with('g') {
            size_str[..size_str.len()-1].parse().unwrap_or(0.0)
        } else if size_str.ends_with('T') || size_str.ends_with('t') {
            size_str[..size_str.len()-1].parse::<f64>().unwrap_or(0.0) * 1024.0
        } else if size_str.ends_with('M') || size_str.ends_with('m') {
            size_str[..size_str.len()-1].parse::<f64>().unwrap_or(0.0) / 1024.0
        } else {
            0.0
        }
    }

    /// 检测GPU设备
    async fn detect_gpu_devices(&self, config: &mut EnvironmentConfig) -> Result<(), String> {
        let mut devices = Vec::new();

        // 检测NVIDIA GPU
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=index,name,memory.total,compute_cap", "--format=csv,noheader"])
            .output() 
        {
            if let Ok(gpu_info) = String::from_utf8(output.stdout) {
                for line in gpu_info.lines() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 4 {
                        if let Ok(index) = parts[0].parse::<u32>() {
                            let memory_str = parts[2].replace("MiB", "").replace("MB", "");
                            let memory_mb = memory_str.parse::<u64>().unwrap_or(0);
                            
                            devices.push(GpuDeviceInfo {
                                index,
                                name: parts[1].to_string(),
                                memory_mb,
                                compute_capability: parts[3].to_string(),
                                is_available: true,
                            });
                        }
                    }
                }
            }
        }

        // 检测Apple Silicon GPU (macOS)
        #[cfg(target_os = "macos")]
        {
            if devices.is_empty() {
                // 检查是否为Apple Silicon
                if let Ok(output) = std::process::Command::new("sysctl")
                    .args(["-n", "machdep.cpu.brand_string"])
                    .output() 
                {
                    if let Ok(cpu_brand) = String::from_utf8(output.stdout) {
                        if cpu_brand.contains("Apple") {
                            devices.push(GpuDeviceInfo {
                                index: 0,
                                name: "Apple Silicon GPU".to_string(),
                                memory_mb: 0, // 共享内存，动态分配
                                compute_capability: "Metal".to_string(),
                                is_available: true,
                            });
                        }
                    }
                }
            }
        }

        config.gpu_devices = devices;
        config.gpu_available = !config.gpu_devices.is_empty();

        Ok(())
    }

    /// 检测Python环境
    async fn detect_python_environment(&self, config: &mut EnvironmentConfig) -> Result<(), String> {
        // 检查Python版本
        let python_commands = vec!["python3", "python"];
        
        for cmd in &python_commands {
            if let Ok(output) = std::process::Command::new(cmd)
                .args(["--version"])
                .output() 
            {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    let version = version.trim();
                    if !version.is_empty() {
                        config.python_environment = Some(format!("{} ({})", cmd, version));
                        break;
                    }
                }
                // 有些Python版本输出到stderr
                if let Ok(version) = String::from_utf8(output.stderr) {
                    let version = version.trim();
                    if !version.is_empty() {
                        config.python_environment = Some(format!("{} ({})", cmd, version));
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查必要的Python包
    async fn check_required_packages(&self, config: &mut EnvironmentConfig) -> Result<(), String> {
        let python_cmd = if config.python_environment.as_ref().map(|s| s.contains("python3")).unwrap_or(false) {
            "python3"
        } else {
            "python"
        };

        let mut missing = Vec::new();

        for package in &config.required_packages {
            let check_script = format!(
                "import {}\nprint('OK')",
                package.replace("-", "_")
            );

            if let Ok(output) = std::process::Command::new(python_cmd)
                .args(["-c", &check_script])
                .output() 
            {
                if !output.status.success() {
                    missing.push(package.clone());
                }
            } else {
                missing.push(package.clone());
            }
        }

        config.missing_packages = missing;
        Ok(())
    }

    /// 配置网络（Iroh）
    async fn configure_network(&self, config: &mut EnvironmentConfig) -> Result<(), String> {
        // 检查网络连接
        config.network_config.is_online = self.check_internet_connection().await;

        // 获取本地IP
        if let Ok(ip) = local_ip_address::local_ip() {
            config.network_config.local_ip = Some(ip.to_string());
        }

        // 初始化Iroh节点（如果可用）
        match IrohConnectionManager::new(Default::default()).await {
            Ok(manager) => {
                config.network_config.iroh_node_id = Some(manager.node_id());
                config.node_id = Some(manager.node_id());
            }
            Err(e) => {
                println!("⚠️ [AUTO-ENV] Iroh initialization failed: {}", e);
            }
        }

        Ok(())
    }

    /// 检查网络连接
    async fn check_internet_connection(&self) -> bool {
        // 尝试连接到知名的DNS服务器
        let addrs = vec![
            "8.8.8.8:53",      // Google DNS
            "1.1.1.1:53",      // Cloudflare DNS
        ];

        for addr in addrs {
            if let Ok(_) = tokio::net::TcpStream::connect(addr).await {
                return true;
            }
        }

        false
    }

    /// AI决定是否安装缺失的包
    async fn ai_decide_package_installation(
        &self,
        config: &EnvironmentConfig,
        _api_key: &str,
    ) -> Result<bool, String> {
        let decision_prompt = format!(
            r#"环境配置决策：

系统资源：
- 可用内存：{}MB
- CPU核心：{}
- 磁盘空间：{:.1}GB
- GPU可用：{}

缺失的Python包：{:?}

是否应该自动安装这些包？
回复 "YES" 或 "NO"，并简要说明理由。
"#,
            config.system_resources.available_memory_mb,
            config.system_resources.cpu_cores,
            config.system_resources.disk_free_gb,
            config.gpu_available,
            config.missing_packages
        );

        let request = crate::agent::bridges::ToolCallRequest {
            session_id: "env_config_decision".to_string(),
            user_id: None,
            tool_id: "claude".to_string(),
            args: json!({
                "prompt": decision_prompt,
                "max_tokens": 50,
                "temperature": 0.3
            }),
            working_directory: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string())),
            environment: std::env::vars().collect(),
            timeout_seconds: Some(30),
            permissions: vec!["read".to_string()],
        };

        match self.bridge_manager.tool_bridge().handle_request(request).await {
            Ok(response) => {
                if response.success {
                    if let Some(result) = response.result {
                        let decision = result.as_str().unwrap_or("NO").to_uppercase();
                        return Ok(decision.starts_with("YES"));
                    }
                }
                Ok(false) // 默认不安装
            }
            Err(e) => {
                println!("⚠️ [AUTO-ENV] AI decision failed: {}, defaulting to NO", e);
                Ok(false)
            }
        }
    }

    /// 安装缺失的包
    async fn install_missing_packages(&self, config: &mut EnvironmentConfig) -> Result<(), String> {
        let python_cmd = if config.python_environment.as_ref().map(|s| s.contains("python3")).unwrap_or(false) {
            "python3"
        } else {
            "python"
        };

        println!("📦 [AUTO-ENV] Installing missing packages: {:?}", config.missing_packages);

        for package in &config.missing_packages.clone() {
            println!("⬇️ [AUTO-ENV] Installing {}...", package);
            
            let result = std::process::Command::new(python_cmd)
                .args(["-m", "pip", "install", package])
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        println!("✅ [AUTO-ENV] {} installed successfully", package);
                        config.missing_packages.retain(|p| p != package);
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        println!("❌ [AUTO-ENV] Failed to install {}: {}", package, error);
                    }
                }
                Err(e) => {
                    println!("❌ [AUTO-ENV] Failed to execute pip for {}: {}", package, e);
                }
            }
        }

        Ok(())
    }

    /// 发现对等节点
    async fn discover_peer_nodes(&self, config: &mut EnvironmentConfig) -> Result<(), String> {
        // 这里可以实现节点发现逻辑
        // 例如通过DHT、种子节点、本地网络广播等方式
        
        // 示例：从配置文件或环境变量读取种子节点
        if let Ok(seed_nodes) = std::env::var("WILLIW_SEED_NODES") {
            let nodes: Vec<String> = seed_nodes.split(',').map(|s| s.trim().to_string()).collect();
            config.peer_nodes.extend(nodes);
            config.network_config.connected_peers = config.peer_nodes.clone();
        }

        // 尝试连接到发现的节点
        if let Some(node_id) = &config.node_id {
            println!("🌐 [AUTO-ENV] Local node ID: {}", node_id);
        }

        Ok(())
    }

    /// 记录环境配置到执行历史
    async fn record_environment_config(&self, execution_id: &str, config: &EnvironmentConfig) {
        let entry = ContextEntry {
            id: format!("env-config-{}", chrono::Utc::now().timestamp()),
            entry_type: ContextType::StatusUpdate,
            content: format!(
                "Environment configured: GPU={}, Python={}, Peers={}",
                config.gpu_available,
                config.python_environment.as_ref().unwrap_or(&"unknown".to_string()),
                config.peer_nodes.len()
            ),
            importance: 9,
            timestamp: chrono::Utc::now().timestamp(),
            task_id: Some(execution_id.to_string()),
        };

        // 如果有上下文管理器，可以在这里记录
        println!("📝 [AUTO-ENV] Environment configuration recorded: {:?}", entry);
    }
}

/// 系统信息结构
#[derive(Debug, Default)]
struct SystemInfo {
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub cpu_cores: u32,
    pub cpu_usage_percent: f32,
    pub disk_free_gb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_size_to_gb() {
        assert_eq!(AsyncWorkflowExecutor::parse_size_to_gb("100G"), 100.0);
        assert_eq!(AsyncWorkflowExecutor::parse_size_to_gb("1T"), 1024.0);
        assert_eq!(AsyncWorkflowExecutor::parse_size_to_gb("512M"), 0.5);
    }

    #[tokio::test]
    async fn test_environment_config_default() {
        let config = EnvironmentConfig::default();
        assert!(!config.is_configured);
        assert!(!config.gpu_available);
        assert!(!config.required_packages.is_empty());
    }
}

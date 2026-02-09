use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use williw::Node;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub privacy_level: String,  // high, medium, low
    pub bandwidth_budget: u32,  // MB/s
    pub network_config: NetworkConfig,
    pub checkpoint_settings: CheckpointSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub max_peers: u32,
    pub bootstrap_nodes: Vec<String>,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSettings {
    pub enabled: bool,
    pub interval_minutes: u32,
    pub max_checkpoints: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            privacy_level: "medium".to_string(),
            bandwidth_budget: 10,
            network_config: NetworkConfig {
                max_peers: 10,
                bootstrap_nodes: vec![],
                port: 9000,
            },
            checkpoint_settings: CheckpointSettings {
                enabled: true,
                interval_minutes: 5,
                max_checkpoints: 10,
            },
        }
    }
}

/// Available model configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dimensions: usize,
    pub learning_rate: f64,
    pub batch_size: usize,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default Model".to_string(),
            description: "Standard model configuration".to_string(),
            dimensions: 784,
            learning_rate: 0.01,
            batch_size: 32,
        }
    }
}

/// Training status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStatus {
    pub is_running: bool,
    pub current_epoch: u32,
    pub total_epochs: u32,
    pub accuracy: f64,
    pub loss: f64,
    pub samples_processed: u64,
}

impl Default for TrainingStatus {
    fn default() -> Self {
        Self {
            is_running: false,
            current_epoch: 0,
            total_epochs: 100,
            accuracy: 0.0,
            loss: 1.0,
            samples_processed: 0,
        }
    }
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub gpu_type: Option<String>,
    pub gpu_usage: Option<f64>,  // 0-100
    pub gpu_memory_total: Option<f64>,  // GB
    pub gpu_memory_used: Option<f64>,  // GB
    pub cpu_cores: u32,
    pub total_memory_gb: f64,
    pub battery_level: Option<f64>,  // 0-100
    pub is_charging: Option<bool>,
}

/// API Key entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub id: String,
    pub name: String,
    pub key: String,
    pub created_at: String,
}

/// Global application state
pub struct AppState {
    pub settings: Arc<Mutex<AppSettings>>,
    pub training_status: Arc<Mutex<TrainingStatus>>,
    pub node: Arc<Mutex<Option<Node>>>,  // 使用真实的Node
    pub available_models: Arc<Mutex<Vec<ModelConfig>>>,
    pub device_info: Arc<Mutex<Option<DeviceInfo>>>,
    pub api_keys: Arc<Mutex<Vec<ApiKeyEntry>>>,
    pub api_client: crate::api_client::WorkersApiClient,
}

impl AppState {
    pub async fn new() -> Self {
        // Initialize available models
        let models = vec![
            ModelConfig {
                id: "lfm2.5-1.2b-thinking".to_string(),
                name: "LFM2.5 1.2B Thinking".to_string(),
                description: "LiquidAI LFM2.5 1.2B parameter thinking model for advanced reasoning".to_string(),
                dimensions: 2048,
                learning_rate: 1e-5,
                batch_size: 8,
            },
            ModelConfig {
                id: "bert-base-uncased".to_string(),
                name: "BERT Base".to_string(),
                description: "Google BERT (Bidirectional Encoder Representations from Transformers) 12-layer, 768-hidden".to_string(),
                dimensions: 768,
                learning_rate: 2e-5,
                batch_size: 32,
            },
            ModelConfig {
                id: "gpt2-medium".to_string(),
                name: "GPT-2 Medium".to_string(),
                description: "OpenAI GPT-2 Medium model with 345M parameters".to_string(),
                dimensions: 1024,
                learning_rate: 5e-5,
                batch_size: 16,
            },
            ModelConfig {
                id: "llama2-7b".to_string(),
                name: "LLaMA 2 7B".to_string(),
                description: "Meta LLaMA 2 7B parameter model for text generation".to_string(),
                dimensions: 4096,
                learning_rate: 1e-5,
                batch_size: 8,
            },
            ModelConfig {
                id: "resnet50".to_string(),
                name: "ResNet-50".to_string(),
                description: "Microsoft ResNet-50 for image classification with 50 layers".to_string(),
                dimensions: 2048,
                learning_rate: 0.1,
                batch_size: 64,
            },
            ModelConfig {
                id: "stable-diffusion-v1-5".to_string(),
                name: "Stable Diffusion 1.5".to_string(),
                description: "Stability AI text-to-image model with CLIP text encoder".to_string(),
                dimensions: 768,
                learning_rate: 1e-4,
                batch_size: 4,
            },
            ModelConfig {
                id: "whisper-medium".to_string(),
                name: "Whisper Medium".to_string(),
                description: "OpenAI Whisper medium model for speech recognition".to_string(),
                dimensions: 1024,
                learning_rate: 1e-4,
                batch_size: 16,
            },
            ModelConfig {
                id: "t5-base".to_string(),
                name: "T5 Base".to_string(),
                description: "Google T5 (Text-to-Text Transfer Transformer) 220M parameters".to_string(),
                dimensions: 768,
                learning_rate: 3e-4,
                batch_size: 32,
            },
        ];

        // Get device info
        let device_info = Self::get_device_info_internal();

        Self {
            settings: Arc::new(Mutex::new(AppSettings::default())),
            training_status: Arc::new(Mutex::new(TrainingStatus::default())),
            node: Arc::new(Mutex::new(None)),  // 真实的Node，初始为None
            available_models: Arc::new(Mutex::new(models)),
            device_info: Arc::new(Mutex::new(Some(device_info))),
            api_keys: Arc::new(Mutex::new(vec![])),
            api_client: crate::api_client::WorkersApiClient::new(
                "https://williw.sirazede725.workers.dev".to_string()
            ),
        }
    }

    fn get_device_info_internal() -> DeviceInfo {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();

        let cpu_cores = sys.cpus().len() as u32;
        let total_memory = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

        // GPU info (使用真实的系统检测)
        let gpu_info = williw::device::DeviceDetector::detect_gpu_usage();
        let (gpu_type, gpu_usage, gpu_memory_total, gpu_memory_used) = if let Some(gpu) = gpu_info.first() {
            (
                Some(gpu.gpu_name.clone()),
                Some(gpu.usage_percent as f64),
                gpu.memory_total_mb.map(|v| v as f64 / 1024.0), // MB to GB
                gpu.memory_used_mb.map(|v| v as f64 / 1024.0), // MB to GB
            )
        } else {
            (None, None, None, None)
        };

        // Battery info (simplified - may not work on all platforms)
        let battery_level: Option<f64> = None;
        let is_charging: Option<bool> = None;

        DeviceInfo {
            gpu_type,
            gpu_usage,
            gpu_memory_total,
            gpu_memory_used,
            cpu_cores,
            total_memory_gb: total_memory,
            battery_level,
            is_charging,
        }
    }

    /// Refresh device info - should be called periodically
    pub fn refresh_device_info(&self) {
        let device_info = Self::get_device_info_internal();
        *self.device_info.lock() = Some(device_info);
    }
}

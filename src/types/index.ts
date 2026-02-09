/// Model configuration from backend
export interface ModelConfig {
  id: string;
  name: string;
  description: string;
  dimensions: number;
  learning_rate: number;
  batch_size: number;
  type?: string;
  size?: string;
  path?: string;
}

/// Training status from backend
export interface TrainingStatus {
  is_running: boolean;
  current_epoch: number;
  total_epochs: number;
  accuracy: number;
  loss: number;
  samples_processed: number;
}

/// Device information from backend
export interface DeviceInfo {
  gpu_type: string | null;
  gpu_usage: number | null;
  gpu_memory_total: number | null;
  gpu_memory_used: number | null;
  cpu_cores: number;
  total_memory_gb: number;
  battery_level: number | null;
  is_charging: boolean | null;
}

/// Application settings
export interface AppSettings {
  privacy_level: string;
  bandwidth_budget: number;
  network_config: NetworkConfig;
  checkpoint_settings: CheckpointSettings;
}

export interface NetworkConfig {
  max_peers: number;
  bootstrap_nodes: string[];
  port: number;
}

export interface CheckpointSettings {
  enabled: boolean;
  interval_minutes: number;
  max_checkpoints: number;
}

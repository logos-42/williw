use crate::types::{decompress_indices, SparseUpdate, TensorSnapshot};
use crate::training::{TrainingData, LossFunction, Optimizer, MSE, SGD, CrossEntropy, MAE, SyntheticData, ArrayData};
use anyhow::{anyhow, Result};
use ndarray::Array1;
use ndarray_npy::{ReadNpyExt, WriteNpyExt};
use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 损失函数类型
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LossType {
    MSE,
    CrossEntropy,
    MAE,
}

#[derive(Clone)]
pub struct InferenceConfig {
    pub model_dim: usize,
    pub model_path: Option<PathBuf>,
    pub checkpoint_dir: Option<PathBuf>,
    // 训练配置
    pub learning_rate: f32,
    pub use_training: bool,
    pub loss_type: LossType,
}


/// Checkpoint 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub version: u64,
    pub model_dim: usize,
    pub timestamp: u64,
    pub model_hash: String,
    pub convergence_score: f32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            model_dim: 256,
            model_path: None,
            checkpoint_dir: None,
            learning_rate: 0.001,
            use_training: false,
            loss_type: LossType::MSE,
        }
    }
}

#[derive(Clone)]
pub struct InferenceEngine {
    state: Arc<RwLock<ModelState>>,
    config: InferenceConfig,
    memory_pressure: Arc<RwLock<MemoryPressure>>,
    optimizer: Arc<RwLock<Box<dyn Optimizer>>>,
    loss_fn: Arc<Box<dyn LossFunction>>,
    training_data: Option<Arc<parking_lot::Mutex<Box<dyn TrainingData>>>>,
}

struct MemoryPressure {
    current_usage_mb: usize,
    pressure_threshold_mb: usize,
}

struct ModelState {
    params: Array1<f32>,
    residual: Array1<f32>,
    version: u64,
    // 收敛度追踪
    previous_params: Option<Array1<f32>>,
    hash_history: Vec<String>,
}

impl InferenceEngine {
    pub fn new(config: InferenceConfig) -> Result<Self> {
        Self::with_training_data(config, None)
    }
    
    /// 使用合成数据创建 InferenceEngine（用于测试和演示）
    pub fn with_synthetic_data(
        config: InferenceConfig,
        seed: u64,
    ) -> Result<Self> {
        let training_data: Box<dyn TrainingData> = Box::new(
            SyntheticData::new(config.model_dim, 1, seed)
        );
        Self::with_training_data(config, Some(training_data))
    }
    
    /// 使用数组数据创建 InferenceEngine
    pub fn with_array_data(
        config: InferenceConfig,
        inputs: Vec<Array1<f32>>,
        outputs: Vec<Array1<f32>>,
    ) -> Result<Self> {
        let training_data: Box<dyn TrainingData> = Box::new(
            ArrayData::new(inputs, outputs)?
        );
        Self::with_training_data(config, Some(training_data))
    }
    
    /// 使用训练数据创建 InferenceEngine
    pub fn with_training_data(
        config: InferenceConfig,
        training_data: Option<Box<dyn TrainingData>>,
    ) -> Result<Self> {
        let params = load_or_random(config.model_dim, config.model_path.as_deref())?;
        let residual = Array1::<f32>::zeros(params.len());
        
        // 估算内存使用：参数 + residual，每个 f32 4 字节
        let estimated_mb = (params.len() * 2 * 4) / (1024 * 1024);
        
        // 创建优化器
        let optimizer: Box<dyn Optimizer> = Box::new(SGD::new(config.learning_rate));
        
        // 根据配置创建损失函数
        let loss_fn: Box<dyn LossFunction> = match config.loss_type {
            LossType::MSE => Box::new(MSE),
            LossType::CrossEntropy => Box::new(CrossEntropy),
            LossType::MAE => Box::new(MAE),
        };
        
        // 包装训练数据
        let training_data_wrapped = training_data.map(|d| Arc::new(parking_lot::Mutex::new(d)));
        
        Ok(Self {
            state: Arc::new(RwLock::new(ModelState {
                params: params.clone(),
                residual,
                version: 1,
                previous_params: Some(params),
                hash_history: Vec::new(),
            })),
            config,
            memory_pressure: Arc::new(RwLock::new(MemoryPressure {
                current_usage_mb: estimated_mb,
                pressure_threshold_mb: estimated_mb * 2, // 阈值设为当前使用的 2 倍
            })),
            optimizer: Arc::new(RwLock::new(optimizer)),
            loss_fn: Arc::new(loss_fn),
            training_data: training_data_wrapped,
        })
    }

    pub fn model_dim(&self) -> usize {
        self.config.model_dim
    }

    pub fn embedding(&self) -> Vec<f32> {
        self.state.read().params.to_vec()
    }

    pub fn tensor_snapshot(&self) -> TensorSnapshot {
        let state = self.state.read();
        TensorSnapshot::new(state.params.to_vec(), state.version)
    }

    pub fn tensor_hash(&self) -> String {
        self.tensor_snapshot().hash()
    }

    pub fn make_sparse_update(&self, k: usize) -> SparseUpdate {
        // 检查内存压力，如果压力大则减少 Top-K
        let effective_k = if self.is_memory_pressured() {
            (k / 2).max(4) // 内存压力时减少到一半，最少 4
        } else {
            k
        };
        
        let mut state = self.state.write();
        let dim = state.params.len();
        if dim == 0 {
            return SparseUpdate {
                indices: Vec::new(),
                values: Vec::new(),
                version: state.version,
            };
        }
        let mut delta = vec![0f32; dim];
        for i in 0..dim {
            delta[i] = state.params[i] + state.residual[i];
        }
        let mut idx_val: Vec<(usize, f32)> =
            delta.iter().enumerate().map(|(i, v)| (i, *v)).collect();
        idx_val.sort_by(|a, b| {
            let av = a.1.abs();
            let bv = b.1.abs();
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        });
        let take = effective_k.min(dim);
        let topk = &idx_val[..take];
        let mut sparse_vals = Vec::with_capacity(take);
        let mut sparse_idx = Vec::with_capacity(take);
        let mut last = 0usize;
        for (i, v) in topk {
            let diff = if sparse_idx.is_empty() {
                *i as u32
            } else {
                (*i - last) as u32
            };
            sparse_idx.push(diff);
            sparse_vals.push(*v);
            state.residual[*i] = delta[*i] - *v;
            last = *i;
        }
        state.version = state.version.saturating_add(1);
        SparseUpdate {
            indices: sparse_idx,
            values: sparse_vals,
            version: state.version,
        }
    }

    /// 检查是否处于内存压力状态
    pub fn is_memory_pressured(&self) -> bool {
        let pressure = self.memory_pressure.read();
        pressure.current_usage_mb >= pressure.pressure_threshold_mb
    }

    /// 获取当前内存使用（MB）
    #[allow(dead_code)]
    pub fn memory_usage_mb(&self) -> usize {
        self.memory_pressure.read().current_usage_mb
    }

    /// 更新内存压力阈值
    #[allow(dead_code)]
    pub fn set_memory_threshold(&self, threshold_mb: usize) {
        self.memory_pressure.write().pressure_threshold_mb = threshold_mb;
    }

    pub fn apply_sparse_update(&self, update: &SparseUpdate) {
        if update.indices.is_empty() {
            return;
        }
        let idxs = decompress_indices(&update.indices);
        let mut state = self.state.write();
        
        // 保存当前参数用于收敛度计算
        state.previous_params = Some(state.params.clone());
        
        for (pos, &v) in idxs.iter().zip(update.values.iter()) {
            if *pos < state.params.len() {
                let old = state.params[*pos];
                let merged = 0.5 * old + 0.5 * v;
                state.params[*pos] = merged;
                state.residual[*pos] += old - merged;
            }
        }
        state.version = state.version.max(update.version);
        
        // 更新 hash 历史
        drop(state);
        let hash = self.tensor_hash();
        state = self.state.write();
        state.hash_history.push(hash);
        if state.hash_history.len() > 10 {
            state.hash_history.remove(0);
        }
    }

    pub fn apply_dense_snapshot(&self, snapshot: &TensorSnapshot) {
        let mut state = self.state.write();
        
        // 保存当前参数用于收敛度计算
        state.previous_params = Some(state.params.clone());
        
        let len = state.params.len().min(snapshot.values.len());
        for i in 0..len {
            state.params[i] = 0.8 * state.params[i] + 0.2 * snapshot.values[i];
        }
        state.version = state.version.max(snapshot.version);
        
        // 更新 hash 历史
        drop(state);
        let hash = self.tensor_hash();
        state = self.state.write();
        state.hash_history.push(hash);
        if state.hash_history.len() > 10 {
            state.hash_history.remove(0);
        }
    }

    /// 计算梯度（数值梯度方法）
    /// 
    /// 使用有限差分法计算梯度，适用于简单模型
    fn compute_gradient_numerical(
        &self,
        input: &Array1<f32>,
        target: &Array1<f32>,
        epsilon: f32,
    ) -> Array1<f32> {
        let state = self.state.read();
        let params = &state.params;
        let mut gradients = Array1::<f32>::zeros(params.len());
        
        // 计算当前损失
        let current_output = self.forward_simple(input);
        let current_loss = self.loss_fn.compute(&current_output, target);
        
        // 对每个参数计算数值梯度
        for i in 0..params.len() {
            // 创建扰动后的参数
            let mut perturbed_params = params.clone();
            perturbed_params[i] += epsilon;
            
            // 使用扰动后的参数计算输出
            let perturbed_output = self.forward_with_params(input, &perturbed_params);
            let perturbed_loss = self.loss_fn.compute(&perturbed_output, target);
            
            // 数值梯度: (f(x + eps) - f(x)) / eps
            gradients[i] = (perturbed_loss - current_loss) / epsilon;
        }
        
        gradients
    }
    
    /// 简单前向传播（使用当前参数）
    fn forward_simple(&self, input: &Array1<f32>) -> Array1<f32> {
        let state = self.state.read();
        self.forward_with_params(input, &state.params)
    }
    
    /// 使用指定参数进行前向传播
    /// 
    /// 这是一个简化的线性模型: output = params * input（点积）
    /// 对于更复杂的模型，需要根据实际架构实现
    fn forward_with_params(&self, input: &Array1<f32>, params: &Array1<f32>) -> Array1<f32> {
        // 简化实现：假设模型是线性变换
        // 如果 input_dim == model_dim，则 output = params * input（逐元素乘积后求和）
        // 如果 input_dim != model_dim，则使用点积
        
        if input.len() == params.len() {
            // 逐元素乘积后求和，返回标量（包装为数组）
            let output: f32 = input.iter()
                .zip(params.iter())
                .map(|(x, p)| x * p)
                .sum();
            Array1::from_vec(vec![output])
        } else {
            // 如果维度不匹配，返回参数的一部分作为输出
            let output_dim = (params.len() / input.len().max(1)).min(params.len());
            Array1::from_vec(params.iter().take(output_dim).cloned().collect())
        }
    }
    
    /// 计算梯度（使用损失函数的解析梯度）
    fn compute_gradient_analytical(
        &self,
        input: &Array1<f32>,
        target: &Array1<f32>,
    ) -> Array1<f32> {
        // 前向传播
        let output = self.forward_simple(input);
        
        // 计算损失对输出的梯度
        let output_grad = self.loss_fn.gradient(&output, target);
        
        // 对于简单的线性模型，梯度是 input * output_grad
        // 这里简化处理：假设模型是 output = sum(params * input)
        let state = self.state.read();
        let mut param_grad = Array1::<f32>::zeros(state.params.len());
        
        if input.len() == state.params.len() {
            // 对于逐元素乘积模型，梯度是 input * output_grad[0]
            let scale = output_grad[0];
            for i in 0..param_grad.len() {
                param_grad[i] = input[i] * scale;
            }
        } else {
            // 简化：将 output_grad 分散到参数上
            for i in 0..param_grad.len().min(output_grad.len()) {
                param_grad[i] = output_grad[i];
            }
        }
        
        param_grad
    }
    
    pub fn local_train_step(&self) {
        // 如果启用了训练且有训练数据，使用真实训练
        if self.config.use_training {
            if let Some(ref training_data) = self.training_data {
                let mut data = training_data.lock();
                if let Some((input, target)) = data.next_sample() {
                    // 使用真实训练逻辑
                    self.train_step_with_data(&input, &target);
                    return;
                }
            }
        }
        
        // Fallback: 如果没有训练数据，使用随机扰动（保持向后兼容）
        let mut rng = rand::thread_rng();
        let mut state = self.state.write();
        
        // 保存当前参数用于收敛度计算
        state.previous_params = Some(state.params.clone());

        for v in state.params.iter_mut() {
            *v += rng.random_range(-1e-3..1e-3);
        }
        state.version = state.version.saturating_add(1);
        
        // 更新 hash 历史（保留最近 10 个）
        drop(state);
        let hash = self.tensor_hash();
        let mut state = self.state.write();
        state.hash_history.push(hash);
        if state.hash_history.len() > 10 {
            state.hash_history.remove(0);
        }
    }
    
    /// 使用训练数据进行一步训练
    fn train_step_with_data(&self, input: &Array1<f32>, target: &Array1<f32>) {
        // 计算梯度（优先使用解析梯度，否则使用数值梯度）
        let gradients = if input.len() == self.state.read().params.len() {
            // 如果维度匹配，使用解析梯度（更快）
            self.compute_gradient_analytical(input, target)
        } else {
            // 否则使用数值梯度（更通用但更慢）
            self.compute_gradient_numerical(input, target, 1e-5)
        };
        
        // 更新参数
        let mut state = self.state.write();
        
        // 保存当前参数用于收敛度计算
        state.previous_params = Some(state.params.clone());
        
        // 使用优化器更新参数
        let mut optimizer = self.optimizer.write();
        optimizer.update(&mut state.params, &gradients);
        
        state.version = state.version.saturating_add(1);
        
        // 更新 hash 历史
        drop(state);
        let hash = self.tensor_hash();
        let mut state = self.state.write();
        state.hash_history.push(hash);
        if state.hash_history.len() > 10 {
            state.hash_history.remove(0);
        }
    }

    /// 计算模型收敛度（0.0-1.0）
    /// 1.0 表示完全收敛（参数不再变化），0.0 表示完全不收敛
    pub fn convergence_score(&self) -> f32 {
        let state = self.state.read();
        
        // 如果没有历史数据，返回 0.0
        if state.previous_params.is_none() || state.params.len() == 0 {
            return 0.0;
        }
        
        let prev = state.previous_params.as_ref().unwrap();
        
        // 计算参数的平均变化幅度
        let mut total_change = 0.0f32;
        let mut count = 0;
        for i in 0..state.params.len().min(prev.len()) {
            let change = (state.params[i] - prev[i]).abs();
            total_change += change;
            count += 1;
        }
        
        if count == 0 {
            return 0.0;
        }
        
        let avg_change = total_change / count as f32;
        
        // 计算参数的标准差（衡量参数分布）
        let mean = state.params.iter().sum::<f32>() / state.params.len() as f32;
        let variance = state.params.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f32>() / state.params.len() as f32;
        let std_dev = variance.sqrt();
        
        // 收敛度计算：
        // - 变化幅度越小，收敛度越高
        // - 标准差越小（参数更集中），收敛度越高
        // - hash 变化频率越低，收敛度越高
        
        let change_score = (1.0 - (avg_change * 1000.0).min(1.0)).max(0.0);
        let std_score = if std_dev > 0.0 {
            (1.0 - (std_dev * 10.0).min(1.0)).max(0.0)
        } else {
            1.0
        };
        
        // hash 稳定性（如果最近 5 个 hash 都相同，说明模型稳定）
        let hash_stability = if state.hash_history.len() >= 5 {
            let recent = &state.hash_history[state.hash_history.len() - 5..];
            let all_same = recent.windows(2).all(|w| w[0] == w[1]);
            if all_same { 1.0 } else { 0.5 }
        } else {
            0.0
        };
        
        // 加权平均
        (change_score * 0.4 + std_score * 0.3 + hash_stability * 0.3).clamp(0.0, 1.0)
    }

    /// 计算参数的平均变化幅度
    pub fn parameter_change_magnitude(&self) -> f32 {
        let state = self.state.read();
        if let Some(prev) = &state.previous_params {
            let mut total_change = 0.0f32;
            let mut count = 0;
            for i in 0..state.params.len().min(prev.len()) {
                total_change += (state.params[i] - prev[i]).abs();
                count += 1;
            }
            if count > 0 {
                total_change / count as f32
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// 计算参数的标准差
    pub fn parameter_std_dev(&self) -> f32 {
        let state = self.state.read();
        if state.params.len() == 0 {
            return 0.0;
        }
        let mean = state.params.iter().sum::<f32>() / state.params.len() as f32;
        let variance = state.params.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f32>() / state.params.len() as f32;
        variance.sqrt()
    }

    /// 保存 checkpoint 为 .npy 格式
    #[allow(dead_code)]
    pub fn save_checkpoint(&self, path: &Path) -> Result<()> {
        let state = self.state.read();
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        state.params.write_npy(&mut writer)?;
        
        Ok(())
    }

    /// 保存结构化 checkpoint（JSON 元数据 + .npy 参数）
    pub fn save_checkpoint_structured(&self, base_path: &Path) -> Result<()> {
        let state = self.state.read();
        
        // 确保目录存在
        if let Some(parent) = base_path.parent() {
            create_dir_all(parent)?;
        }
        
        // 获取时间戳
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 生成文件路径
        let params_path = base_path.with_extension("npy");
        let metadata_path = base_path.with_extension("json");
        
        // 保存参数文件
        let file = File::create(&params_path)?;
        let mut writer = BufWriter::new(file);
        state.params.write_npy(&mut writer)?;
        
        // 计算模型 hash
        let snapshot = TensorSnapshot::new(state.params.to_vec(), state.version);
        let model_hash = snapshot.hash();
        
        // 计算收敛度
        drop(state);
        let convergence_score = self.convergence_score();
        let state = self.state.read();
        
        // 创建元数据
        let metadata = CheckpointMetadata {
            version: state.version,
            model_dim: state.params.len(),
            timestamp,
            model_hash,
            convergence_score,
        };
        
        // 保存元数据
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&metadata_path, metadata_json)?;
        
        Ok(())
    }

    /// 加载 checkpoint（自动检测格式）
    pub fn load_checkpoint(&self, path: &Path) -> Result<()> {
        // 检查是否存在结构化 checkpoint（.json 文件）
        let json_path = path.with_extension("json");
        let npy_path = if json_path.exists() {
            path.with_extension("npy")
        } else {
            path.to_path_buf()
        };
        
        if !npy_path.exists() {
            return Err(anyhow!("Checkpoint 文件不存在: {:?}", npy_path));
        }
        
        // 加载参数
        let file = File::open(&npy_path)?;
        let params: Array1<f32> = Array1::read_npy(file)?;
        
        // 验证维度
        if params.len() != self.config.model_dim {
            return Err(anyhow!(
                "Checkpoint 维度不匹配: 期望 {}, 实际 {}",
                self.config.model_dim,
                params.len()
            ));
        }
        
        // 如果存在元数据，读取版本信息
        let version = if json_path.exists() {
            let metadata_json = std::fs::read_to_string(&json_path)?;
            let metadata: CheckpointMetadata = serde_json::from_str(&metadata_json)?;
            metadata.version
        } else {
            // 如果没有元数据，使用当前版本 + 1
            self.state.read().version + 1
        };
        
        // 更新模型状态
        let mut state = self.state.write();
        state.previous_params = Some(state.params.clone());
        state.params = params.clone();
        state.residual = Array1::<f32>::zeros(params.len());
        state.version = version;
        
        // 更新 hash 历史
        drop(state);
        let hash = self.tensor_hash();
        let mut state = self.state.write();
        state.hash_history.push(hash);
        if state.hash_history.len() > 10 {
            state.hash_history.remove(0);
        }
        
        Ok(())
    }

    /// 查找最新的 checkpoint
    pub fn find_latest_checkpoint(checkpoint_dir: &Path) -> Result<Option<PathBuf>> {
        if !checkpoint_dir.exists() {
            return Ok(None);
        }
        
        let mut latest: Option<(u64, PathBuf)> = None;
        
        // 遍历目录查找所有 .json 文件（结构化 checkpoint）
        for entry in std::fs::read_dir(checkpoint_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // 读取元数据获取时间戳
                if let Ok(metadata_json) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<CheckpointMetadata>(&metadata_json) {
                        let base_path = path.with_extension("");
                        if let Some((latest_ts, _)) = latest {
                            if metadata.timestamp > latest_ts {
                                latest = Some((metadata.timestamp, base_path));
                            }
                        } else {
                            latest = Some((metadata.timestamp, base_path));
                        }
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("npy") {
                // 对于简单的 .npy 文件，使用文件修改时间
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let timestamp = modified
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let base_path = path.with_extension("");
                        if let Some((latest_ts, _)) = latest {
                            if timestamp > latest_ts {
                                latest = Some((timestamp, base_path));
                            }
                        } else {
                            latest = Some((timestamp, base_path));
                        }
                    }
                }
            }
        }
        
        Ok(latest.map(|(_, path)| path))
    }
}

/// 验证模型文件
pub fn validate_model_file(path: &Path, expected_dim: Option<usize>) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("模型文件不存在: {:?}", path));
    }
    
    // 检查文件扩展名
    let ext = path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        "npy" => {
            // .npy 格式，直接验证
        }
        "pt" | "pth" => {
            // PyTorch 格式，提示用户使用转换工具
            return Err(anyhow!(
                "PyTorch 模型文件 (.pt/.pth) 需要先转换为 .npy 格式。\n\
                请使用转换工具: python tools/convert_pytorch_model.py {:?} <output.npy>",
                path
            ));
        }
        _ => {
            return Err(anyhow!(
                "不支持的模型文件格式: {}。支持格式: .npy, .pt, .pth",
                ext
            ));
        }
    }
    
    // 尝试加载文件
    let file = File::open(path)?;
    let arr: Array1<f32> = Array1::read_npy(file)?;
    
    // 检查维度
    if let Some(expected) = expected_dim {
        if arr.len() != expected {
            return Err(anyhow!(
                "模型维度不匹配: 期望 {}, 实际 {}",
                expected,
                arr.len()
            ));
        }
    }
    
    // 检查参数值是否在合理范围内
    let min_val = arr.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = arr.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    
    if min_val.is_infinite() || max_val.is_infinite() || min_val.is_nan() || max_val.is_nan() {
        return Err(anyhow!("模型包含无效值 (NaN 或 Infinity)"));
    }
    
    // 检查参数范围是否合理（通常在 -10 到 10 之间）
    if min_val < -100.0 || max_val > 100.0 {
        return Err(anyhow!(
            "模型参数值超出合理范围: [{}, {}]",
            min_val,
            max_val
        ));
    }
    
    Ok(())
}

fn load_or_random(dim: usize, path: Option<&Path>) -> Result<Array1<f32>> {
    if let Some(path) = path {
        if path.exists() {
            // 检查文件扩展名
            let ext = path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            match ext.as_str() {
                "npy" => {
                    // .npy 格式，直接加载
                    validate_model_file(path, Some(dim))?;
                    
                    let file = File::open(path)?;
                    let arr: Array1<f32> = Array1::read_npy(file)?;
                    
                    // 再次检查维度（双重验证）
                    if arr.len() != dim {
                        return Err(anyhow!(
                            "模型维度不匹配: 配置 {}, 文件 {}",
                            dim,
                            arr.len()
                        ));
                    }
                    
                    return Ok(arr);
                }
                "pt" | "pth" => {
                    // PyTorch 格式，提示用户使用转换工具
                    return Err(anyhow!(
                        "PyTorch 模型文件 (.pt/.pth) 需要先转换为 .npy 格式。\n\
                        请使用转换工具: python tools/convert_pytorch_model.py {:?} <output.npy>\n\
                        然后使用转换后的 .npy 文件作为 model_path。",
                        path
                    ));
                }
                _ => {
                    return Err(anyhow!(
                        "不支持的模型文件格式: {}。支持格式: .npy, .pt, .pth",
                        ext
                    ));
                }
            }
        } else {
            return Err(anyhow!("model file {:?} not found", path));
        }
    }
    let mut rng = rand::thread_rng();
    let data: Vec<f32> = (0..dim).map(|_| rng.random_range(-0.1..0.1)).collect();
    Ok(Array1::from_vec(data))
}

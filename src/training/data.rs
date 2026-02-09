use anyhow::Result;
use ndarray::Array1;

/// 训练数据接口
/// 
/// 支持不同的数据源，包括合成数据、数组数据等
pub trait TrainingData: Send + Sync {
    /// 获取下一个训练样本
    /// 返回 (输入, 目标输出)
    fn next_sample(&mut self) -> Option<(Array1<f32>, Array1<f32>)>;
    
    /// 获取批量样本
    fn next_batch(&mut self, batch_size: usize) -> Vec<(Array1<f32>, Array1<f32>)> {
        let mut batch = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            if let Some(sample) = self.next_sample() {
                batch.push(sample);
            } else {
                break;
            }
        }
        batch
    }
    
    /// 重置数据源（用于新的 epoch）
    fn reset(&mut self);
    
    /// 获取数据维度信息
    fn input_dim(&self) -> usize;
    fn output_dim(&self) -> usize;
}

/// 合成数据生成器
/// 
/// 用于测试和演示，生成简单的线性关系数据
/// 
/// # 示例
/// 
/// ```rust
/// use GGB::training::SyntheticData;
/// 
/// let mut data = SyntheticData::new(64, 1, 12345);
/// data = data.with_noise_scale(0.02);
/// 
/// if let Some((input, output)) = data.next_sample() {
///     println!("输入维度: {}, 输出维度: {}", input.len(), output.len());
/// }
/// ```
pub struct SyntheticData {
    input_dim: usize,
    output_dim: usize,
    seed: u64,
    counter: u64,
    // 用于生成线性关系: y = W * x + b + noise
    weight: Array1<f32>,
    bias: Array1<f32>,
    noise_scale: f32,
}

impl SyntheticData {
    /// 创建新的合成数据生成器
    pub fn new(input_dim: usize, output_dim: usize, seed: u64) -> Self {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        use rand::Rng;

        let mut rng = StdRng::seed_from_u64(seed);

        // 生成随机权重和偏置
        let weight: Vec<f32> = (0..input_dim * output_dim)
            .map(|_| rng.random_range(-0.1..0.1))
            .collect();
        let weight = Array1::from_vec(weight);

        let bias: Vec<f32> = (0..output_dim)
            .map(|_| rng.random_range(-0.05..0.05))
            .collect();
        let bias = Array1::from_vec(bias);
        
        Self {
            input_dim,
            output_dim,
            seed,
            counter: 0,
            weight,
            bias,
            noise_scale: 0.01,
        }
    }
    
    /// 设置噪声比例
    pub fn with_noise_scale(mut self, noise_scale: f32) -> Self {
        self.noise_scale = noise_scale;
        self
    }
}

impl TrainingData for SyntheticData {
    fn next_sample(&mut self) -> Option<(Array1<f32>, Array1<f32>)> {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        use rand::Rng;
        
        // 使用 counter 作为随机种子的一部分，确保可重复性
        let mut rng = StdRng::seed_from_u64(self.seed.wrapping_add(self.counter));
        self.counter = self.counter.wrapping_add(1);

        // 生成随机输入
        let input: Vec<f32> = (0..self.input_dim)
            .map(|_| rng.random_range(-1.0..1.0))
            .collect();
        let input = Array1::from_vec(input);

        // 计算输出: y = W * x + b + noise
        let mut output = Array1::<f32>::zeros(self.output_dim);
        for i in 0..self.output_dim {
            let mut sum = self.bias[i];
            for j in 0..self.input_dim {
                sum += self.weight[i * self.input_dim + j] * input[j];
            }
            // 添加噪声
            sum += rng.random_range(-self.noise_scale..self.noise_scale);
            output[i] = sum;
        }
        
        Some((input, output))
    }
    
    fn reset(&mut self) {
        self.counter = 0;
    }
    
    fn input_dim(&self) -> usize {
        self.input_dim
    }
    
    fn output_dim(&self) -> usize {
        self.output_dim
    }
}

/// 数组数据加载器
/// 
/// 从预加载的数组数据中提供训练样本
/// 
/// # 示例
/// 
/// ```rust
/// use GGB::training::ArrayData;
/// use ndarray::Array1;
/// 
/// let inputs: Vec<Array1<f32>> = vec![
///     Array1::from_vec(vec![1.0, 2.0]),
///     Array1::from_vec(vec![3.0, 4.0]),
/// ];
/// let outputs: Vec<Array1<f32>> = vec![
///     Array1::from_vec(vec![2.0]),
///     Array1::from_vec(vec![6.0]),
/// ];
/// 
/// let mut data = ArrayData::new(inputs, outputs).unwrap();
/// assert_eq!(data.input_dim(), 2);
/// assert_eq!(data.output_dim(), 1);
/// ```
pub struct ArrayData {
    inputs: Vec<Array1<f32>>,
    outputs: Vec<Array1<f32>>,
    current_index: usize,
    input_dim: usize,
    output_dim: usize,
}

impl ArrayData {
    /// 从输入和输出数组创建数据加载器
    pub fn new(inputs: Vec<Array1<f32>>, outputs: Vec<Array1<f32>>) -> Result<Self> {
        if inputs.is_empty() || outputs.is_empty() {
            return Err(anyhow::anyhow!("输入或输出数组不能为空"));
        }
        
        if inputs.len() != outputs.len() {
            return Err(anyhow::anyhow!(
                "输入和输出数组长度不匹配: {} != {}",
                inputs.len(),
                outputs.len()
            ));
        }
        
        let input_dim = inputs[0].len();
        let output_dim = outputs[0].len();
        
        // 验证所有数组维度一致
        for (i, input) in inputs.iter().enumerate() {
            if input.len() != input_dim {
                return Err(anyhow::anyhow!(
                    "输入数组 {} 维度不匹配: 期望 {}, 实际 {}",
                    i,
                    input_dim,
                    input.len()
                ));
            }
        }
        
        for (i, output) in outputs.iter().enumerate() {
            if output.len() != output_dim {
                return Err(anyhow::anyhow!(
                    "输出数组 {} 维度不匹配: 期望 {}, 实际 {}",
                    i,
                    output_dim,
                    output.len()
                ));
            }
        }
        
        Ok(Self {
            inputs,
            outputs,
            current_index: 0,
            input_dim,
            output_dim,
        })
    }
    
    /// 从文件加载数据（未来扩展）
    #[allow(dead_code)]
    pub fn from_file(_path: &std::path::Path) -> Result<Self> {
        // TODO: 实现从文件加载
        Err(anyhow::anyhow!("尚未实现"))
    }
}

impl TrainingData for ArrayData {
    fn next_sample(&mut self) -> Option<(Array1<f32>, Array1<f32>)> {
        if self.current_index >= self.inputs.len() {
            return None;
        }
        
        let input = self.inputs[self.current_index].clone();
        let output = self.outputs[self.current_index].clone();
        self.current_index += 1;
        
        Some((input, output))
    }
    
    fn reset(&mut self) {
        self.current_index = 0;
    }
    
    fn input_dim(&self) -> usize {
        self.input_dim
    }
    
    fn output_dim(&self) -> usize {
        self.output_dim
    }
}


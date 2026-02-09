use ndarray::Array1;

/// 优化器接口
pub trait Optimizer: Send + Sync {
    /// 更新参数
    /// 
    /// Args:
    ///   - params: 当前参数（会被修改）
    ///   - gradients: 梯度
    fn update(&mut self, params: &mut Array1<f32>, gradients: &Array1<f32>);
    
    /// 重置优化器状态（用于新的训练周期）
    #[allow(dead_code)]
    fn reset(&mut self);
}

/// 随机梯度下降 (SGD) 优化器
pub struct SGD {
    learning_rate: f32,
    momentum: f32,
    velocity: Option<Array1<f32>>,
}

impl SGD {
    /// 创建新的 SGD 优化器
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            momentum: 0.0,
            velocity: None,
        }
    }
    
    /// 创建带动量的 SGD 优化器
    #[allow(dead_code)]
    pub fn with_momentum(learning_rate: f32, momentum: f32) -> Self {
        Self {
            learning_rate,
            momentum: momentum.clamp(0.0, 1.0),
            velocity: None,
        }
    }
    
    /// 设置学习率
    #[allow(dead_code)]
    pub fn set_learning_rate(&mut self, lr: f32) {
        self.learning_rate = lr;
    }
}

impl Optimizer for SGD {
    fn update(&mut self, params: &mut Array1<f32>, gradients: &Array1<f32>) {
        if params.len() != gradients.len() {
            return;
        }
        
        if self.momentum > 0.0 {
            // 使用动量
            let velocity = self.velocity.get_or_insert_with(|| Array1::<f32>::zeros(params.len()));
            
            // 更新速度: v = momentum * v - learning_rate * grad
            for i in 0..params.len() {
                velocity[i] = self.momentum * velocity[i] - self.learning_rate * gradients[i];
                params[i] += velocity[i];
            }
        } else {
            // 标准 SGD: params = params - learning_rate * grad
            for i in 0..params.len() {
                params[i] -= self.learning_rate * gradients[i];
            }
        }
    }
    
    fn reset(&mut self) {
        self.velocity = None;
    }
}


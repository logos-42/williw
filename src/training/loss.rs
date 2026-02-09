use ndarray::Array1;

/// 损失函数接口
pub trait LossFunction: Send + Sync {
    /// 计算损失值
    fn compute(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> f32;
    
    /// 计算损失对预测值的梯度
    fn gradient(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> Array1<f32>;
}

/// 均方误差 (MSE)
pub struct MSE;

impl LossFunction for MSE {
    fn compute(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> f32 {
        if predicted.len() != target.len() {
            return f32::INFINITY;
        }
        
        let mut sum = 0.0;
        for i in 0..predicted.len() {
            let diff = predicted[i] - target[i];
            sum += diff * diff;
        }
        
        sum / predicted.len() as f32
    }
    
    fn gradient(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> Array1<f32> {
        if predicted.len() != target.len() {
            return Array1::<f32>::zeros(predicted.len());
        }
        
        let mut grad = Array1::<f32>::zeros(predicted.len());
        let scale = 2.0 / predicted.len() as f32;
        
        for i in 0..predicted.len() {
            grad[i] = scale * (predicted[i] - target[i]);
        }
        
        grad
    }
}

/// 交叉熵损失
/// 
/// 注意：假设预测值已经过 softmax 处理
pub struct CrossEntropy;

impl LossFunction for CrossEntropy {
    fn compute(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> f32 {
        if predicted.len() != target.len() {
            return f32::INFINITY;
        }
        
        let mut loss = 0.0;
        for i in 0..predicted.len() {
            // 避免 log(0)
            let pred = predicted[i].max(1e-10);
            loss -= target[i] * pred.ln();
        }
        
        loss
    }
    
    fn gradient(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> Array1<f32> {
        if predicted.len() != target.len() {
            return Array1::<f32>::zeros(predicted.len());
        }
        
        let mut grad = Array1::<f32>::zeros(predicted.len());
        
        for i in 0..predicted.len() {
            // 对于交叉熵，梯度是 predicted - target（假设预测值已 softmax）
            grad[i] = predicted[i] - target[i];
        }
        
        grad
    }
}

/// 平均绝对误差 (MAE)
pub struct MAE;

impl LossFunction for MAE {
    fn compute(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> f32 {
        if predicted.len() != target.len() {
            return f32::INFINITY;
        }
        
        let mut sum = 0.0;
        for i in 0..predicted.len() {
            sum += (predicted[i] - target[i]).abs();
        }
        
        sum / predicted.len() as f32
    }
    
    fn gradient(&self, predicted: &Array1<f32>, target: &Array1<f32>) -> Array1<f32> {
        if predicted.len() != target.len() {
            return Array1::<f32>::zeros(predicted.len());
        }
        
        let mut grad = Array1::<f32>::zeros(predicted.len());
        let scale = 1.0 / predicted.len() as f32;
        
        for i in 0..predicted.len() {
            let diff = predicted[i] - target[i];
            // MAE 的梯度是 sign(diff)
            grad[i] = scale * diff.signum();
        }
        
        grad
    }
}


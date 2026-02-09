//! 零知识证明模块
//! 
//! 提供基于nori库的零知识证明功能，用于算力验证和隐私保护

pub mod nori;
pub mod circuits;
pub mod verification;

// 重新导出常用类型
pub use nori::{ZKProof, ZKProver, ZKVerifier};
pub use circuits::{ComputeCircuit, TaskCircuit};
pub use verification::{ProofVerifier, VerificationResult};

/// ZK配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZKConfig {
    /// 是否启用ZK证明
    pub enabled: bool,
    /// 证明系统类型
    pub proof_system: ProofSystem,
    /// 安全参数
    pub security_level: SecurityLevel,
    /// 证明生成超时时间（毫秒）
    pub proof_generation_timeout_ms: u64,
    /// 验证超时时间（毫秒）
    pub verification_timeout_ms: u64,
}

/// 证明系统类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProofSystem {
    /// Groth16证明系统
    Groth16,
    /// Plonk证明系统
    Plonk,
    /// Marlin证明系统
    Marlin,
}

/// 安全级别
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SecurityLevel {
    /// 低安全性（测试用）
    Low,
    /// 中等安全性
    Medium,
    /// 高安全性
    High,
    /// 最高安全性
    Maximum,
}

impl Default for ZKConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            proof_system: ProofSystem::Groth16,
            security_level: SecurityLevel::Medium,
            proof_generation_timeout_ms: 5000,
            verification_timeout_ms: 1000,
        }
    }
}

/// 计算任务证明
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComputeProof {
    /// 证明ID
    pub proof_id: String,
    /// 任务ID
    pub task_id: String,
    /// 节点ID
    pub node_id: String,
    /// 证明数据
    pub proof_data: Vec<u8>,
    /// 公开输入
    pub public_inputs: Vec<u8>,
    /// 证明生成时间戳
    pub timestamp: i64,
    /// 证明有效性
    pub valid: bool,
}

/// 证明生成器
pub struct ProofGenerator {
    config: ZKConfig,
    prover: ZKProver,
}

impl ProofGenerator {
    /// 创建新的证明生成器
    pub fn new(config: ZKConfig) -> anyhow::Result<Self> {
        let prover = ZKProver::new(config.clone())?;
        Ok(Self { config, prover })
    }
    
    /// 为计算任务生成证明
    pub fn generate_proof(&self, task: &ComputeTask, result: &TaskResult) -> anyhow::Result<ComputeProof> {
        // 创建计算电路
        let circuit = ComputeCircuit::from_task_and_result(task, result);
        
        // 生成证明
        let (proof_data, public_inputs) = self.prover.prove(&circuit)?;
        
        Ok(ComputeProof {
            proof_id: format!("proof_{}_{}", task.id, chrono::Utc::now().timestamp()),
            task_id: task.id.clone(),
            node_id: result.node_id.clone(),
            proof_data,
            public_inputs,
            timestamp: chrono::Utc::now().timestamp(),
            valid: true,
        })
    }
    
    /// 批量生成证明
    pub fn generate_batch_proofs(
        &self,
        tasks: &[ComputeTask],
        results: &[TaskResult],
    ) -> anyhow::Result<Vec<ComputeProof>> {
        let mut proofs = Vec::new();
        
        for (task, result) in tasks.iter().zip(results.iter()) {
            let proof = self.generate_proof(task, result)?;
            proofs.push(proof);
        }
        
        Ok(proofs)
    }
}

/// 计算任务（示例结构）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComputeTask {
    pub id: String,
    pub description: String,
    pub input_data: Vec<u8>,
    pub expected_output: Vec<u8>,
}

/// 任务结果（示例结构）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskResult {
    pub node_id: String,
    pub task_id: String,
    pub output_data: Vec<u8>,
    pub execution_time_ms: u64,
    pub success: bool,
}

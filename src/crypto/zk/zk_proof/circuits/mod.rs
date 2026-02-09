//! ZK电路定义
//! 
//! 定义用于算力验证的零知识证明电路

use crate::zk_proof::{ComputeTask, TaskResult};

/// 计算电路接口
pub trait ComputeCircuit {
    /// 获取电路输入
    fn inputs(&self) -> Vec<u8>;
    
    /// 获取电路输出
    fn outputs(&self) -> Vec<u8>;
    
    /// 获取公开输入
    fn public_inputs(&self) -> Vec<u8>;
    
    /// 获取私有输入
    fn private_inputs(&self) -> Vec<u8>;
}

/// 简单计算电路
pub struct SimpleComputeCircuit {
    task: ComputeTask,
    result: TaskResult,
}

impl SimpleComputeCircuit {
    /// 从任务和结果创建电路
    pub fn from_task_and_result(task: &ComputeTask, result: &TaskResult) -> Self {
        Self {
            task: task.clone(),
            result: result.clone(),
        }
    }
}

impl ComputeCircuit for SimpleComputeCircuit {
    fn inputs(&self) -> Vec<u8> {
        self.task.input_data.clone()
    }
    
    fn outputs(&self) -> Vec<u8> {
        self.result.output_data.clone()
    }
    
    fn public_inputs(&self) -> Vec<u8> {
        // 公开输入：任务ID、预期输出
        let mut public = Vec::new();
        public.extend(self.task.id.as_bytes());
        public.extend(&self.task.expected_output);
        public
    }
    
    fn private_inputs(&self) -> Vec<u8> {
        // 私有输入：实际计算过程
        let mut private = Vec::new();
        private.extend(self.result.output_data.clone());
        private.extend(&self.result.execution_time_ms.to_le_bytes());
        private
    }
}

/// 任务验证电路
pub struct TaskCircuit {
    task_id: String,
    input_hash: [u8; 32],
    output_hash: [u8; 32],
    proof_of_work: [u8; 32],
}

impl TaskCircuit {
    /// 创建新的任务电路
    pub fn new(
        task_id: String,
        input_data: &[u8],
        output_data: &[u8],
        proof_of_work: &[u8],
    ) -> Self {
        use sha3::{Digest, Sha3_256};
        
        let mut hasher = Sha3_256::new();
        hasher.update(input_data);
        let input_hash: [u8; 32] = hasher.finalize().into();
        
        let mut hasher = Sha3_256::new();
        hasher.update(output_data);
        let output_hash: [u8; 32] = hasher.finalize().into();
        
        let mut proof_hash = [0u8; 32];
        proof_hash.copy_from_slice(&proof_of_work[..32]);
        
        Self {
            task_id,
            input_hash,
            output_hash,
            proof_of_work: proof_hash,
        }
    }
}

impl ComputeCircuit for TaskCircuit {
    fn inputs(&self) -> Vec<u8> {
        let mut inputs = Vec::new();
        inputs.extend(&self.input_hash);
        inputs
    }
    
    fn outputs(&self) -> Vec<u8> {
        let mut outputs = Vec::new();
        outputs.extend(&self.output_hash);
        outputs
    }
    
    fn public_inputs(&self) -> Vec<u8> {
        // 公开输入：任务ID、输入哈希、输出哈希
        let mut public = Vec::new();
        public.extend(self.task_id.as_bytes());
        public.extend(&self.input_hash);
        public.extend(&self.output_hash);
        public
    }
    
    fn private_inputs(&self) -> Vec<u8> {
        // 私有输入：工作量证明
        self.proof_of_work.to_vec()
    }
}

/// 批量计算电路
pub struct BatchComputeCircuit {
    circuits: Vec<SimpleComputeCircuit>,
}

impl BatchComputeCircuit {
    /// 创建批量电路
    pub fn new(circuits: Vec<SimpleComputeCircuit>) -> Self {
        Self { circuits }
    }
}

impl ComputeCircuit for BatchComputeCircuit {
    fn inputs(&self) -> Vec<u8> {
        let mut all_inputs = Vec::new();
        for circuit in &self.circuits {
            all_inputs.extend(circuit.inputs());
        }
        all_inputs
    }
    
    fn outputs(&self) -> Vec<u8> {
        let mut all_outputs = Vec::new();
        for circuit in &self.circuits {
            all_outputs.extend(circuit.outputs());
        }
        all_outputs
    }
    
    fn public_inputs(&self) -> Vec<u8> {
        let mut all_public = Vec::new();
        for circuit in &self.circuits {
            all_public.extend(circuit.public_inputs());
        }
        all_public
    }
    
    fn private_inputs(&self) -> Vec<u8> {
        let mut all_private = Vec::new();
        for circuit in &self.circuits {
            all_private.extend(circuit.private_inputs());
        }
        all_private
    }
}

#[cfg(feature = "zk_proof")]
impl nori::Circuit for SimpleComputeCircuit {
    fn synthesize<CS: nori::ConstraintSystem>(&self, cs: &mut CS) -> Result<(), nori::Error> {
        // 这里实现具体的电路约束
        // 由于nori API的具体细节未知，这里提供框架
        
        // 示例：验证输出与预期匹配
        let expected_output = cs.alloc_input(|| "expected_output", || Ok(self.task.expected_output.clone()))?;
        let actual_output = cs.alloc_private(|| "actual_output", || Ok(self.result.output_data.clone()))?;
        
        // 添加约束：实际输出等于预期输出
        cs.enforce(
            || "output_match",
            |lc| lc + actual_output,
            |lc| lc + CS::one(),
            |lc| lc + expected_output,
        );
        
        Ok(())
    }
}

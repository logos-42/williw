//! 证明验证模块
//! 
//! 提供零知识证明的验证功能

use crate::zk_proof::{ZKConfig, ComputeProof, ZKVerifier};
use anyhow::Result;

/// 验证结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationResult {
    /// 证明ID
    pub proof_id: String,
    /// 是否有效
    pub valid: bool,
    /// 验证时间（毫秒）
    pub verification_time_ms: u64,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 验证时间戳
    pub timestamp: i64,
}

/// 批量验证结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchVerificationResult {
    /// 总证明数
    pub total_proofs: usize,
    /// 有效证明数
    pub valid_proofs: usize,
    /// 无效证明数
    pub invalid_proofs: usize,
    /// 详细结果
    pub results: Vec<VerificationResult>,
    /// 总验证时间（毫秒）
    pub total_verification_time_ms: u64,
}

/// 证明验证器
pub struct ProofVerifier {
    config: ZKConfig,
    verifier: ZKVerifier,
}

impl ProofVerifier {
    /// 创建新的证明验证器
    pub fn new(config: ZKConfig) -> Result<Self> {
        let verifier = ZKVerifier::new(config.clone())?;
        Ok(Self { config, verifier })
    }
    
    /// 验证单个证明
    pub fn verify_proof(&self, proof: &ComputeProof) -> VerificationResult {
        let start_time = std::time::Instant::now();
        
        let verification_result = self.verifier.verify(
            &proof.proof_data,
            &proof.public_inputs,
            &[], // 这里需要验证密钥，暂时留空
        );
        
        let verification_time_ms = start_time.elapsed().as_millis() as u64;
        
        match verification_result {
            Ok(valid) => VerificationResult {
                proof_id: proof.proof_id.clone(),
                valid,
                verification_time_ms,
                error: None,
                timestamp: chrono::Utc::now().timestamp(),
            },
            Err(e) => VerificationResult {
                proof_id: proof.proof_id.clone(),
                valid: false,
                verification_time_ms,
                error: Some(format!("验证错误: {}", e)),
                timestamp: chrono::Utc::now().timestamp(),
            },
        }
    }
    
    /// 批量验证证明
    pub fn verify_batch(&self, proofs: &[ComputeProof]) -> BatchVerificationResult {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut valid_count = 0;
        let mut invalid_count = 0;
        
        for proof in proofs {
            let result = self.verify_proof(proof);
            if result.valid {
                valid_count += 1;
            } else {
                invalid_count += 1;
            }
            results.push(result);
        }
        
        let total_verification_time_ms = start_time.elapsed().as_millis() as u64;
        
        BatchVerificationResult {
            total_proofs: proofs.len(),
            valid_proofs: valid_count,
            invalid_proofs: invalid_count,
            results,
            total_verification_time_ms,
        }
    }
    
    /// 异步验证证明（用于WASM环境）
    pub async fn verify_proof_async(&self, proof: ComputeProof) -> VerificationResult {
        // 在WASM环境中使用异步执行
        let proof_clone = proof.clone();
        let verifier_clone = self.verifier.clone();
        
        tokio::task::spawn_blocking(move || {
            let start_time = std::time::Instant::now();
            
            let verification_result = verifier_clone.verify(
                &proof_clone.proof_data,
                &proof_clone.public_inputs,
                &[], // 验证密钥
            );
            
            let verification_time_ms = start_time.elapsed().as_millis() as u64;
            
            match verification_result {
                Ok(valid) => VerificationResult {
                    proof_id: proof_clone.proof_id,
                    valid,
                    verification_time_ms,
                    error: None,
                    timestamp: chrono::Utc::now().timestamp(),
                },
                Err(e) => VerificationResult {
                    proof_id: proof_clone.proof_id,
                    valid: false,
                    verification_time_ms,
                    error: Some(format!("验证错误: {}", e)),
                    timestamp: chrono::Utc::now().timestamp(),
                },
            }
        })
        .await
        .unwrap_or_else(|_| VerificationResult {
            proof_id: proof.proof_id,
            valid: false,
            verification_time_ms: 0,
            error: Some("任务执行失败".to_string()),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
    
    /// 验证证明并检查超时
    pub fn verify_with_timeout(&self, proof: &ComputeProof) -> Result<VerificationResult> {
        let timeout = std::time::Duration::from_millis(self.config.verification_timeout_ms);
        
        // 使用线程实现超时控制
        let proof_clone = proof.clone();
        let verifier_clone = self.verifier.clone();
        
        let handle = std::thread::spawn(move || {
            let start_time = std::time::Instant::now();
            
            let verification_result = verifier_clone.verify(
                &proof_clone.proof_data,
                &proof_clone.public_inputs,
                &[], // 验证密钥
            );
            
            let verification_time_ms = start_time.elapsed().as_millis() as u64;
            
            match verification_result {
                Ok(valid) => VerificationResult {
                    proof_id: proof_clone.proof_id,
                    valid,
                    verification_time_ms,
                    error: None,
                    timestamp: chrono::Utc::now().timestamp(),
                },
                Err(e) => VerificationResult {
                    proof_id: proof_clone.proof_id,
                    valid: false,
                    verification_time_ms,
                    error: Some(format!("验证错误: {}", e)),
                    timestamp: chrono::Utc::now().timestamp(),
                },
            }
        });
        
        match handle.join_timeout(timeout) {
            Ok(result) => Ok(result),
            Err(_) => Ok(VerificationResult {
                proof_id: proof.proof_id.clone(),
                valid: false,
                verification_time_ms: timeout.as_millis() as u64,
                error: Some("验证超时".to_string()),
                timestamp: chrono::Utc::now().timestamp(),
            }),
        }
    }
}

/// WASM兼容的验证接口
#[cfg(feature = "wasm")]
pub mod wasm {
    use wasm_bindgen::prelude::*;
    use super::*;
    
    #[wasm_bindgen]
    pub struct WasmProofVerifier {
        verifier: ProofVerifier,
    }
    
    #[wasm_bindgen]
    impl WasmProofVerifier {
        #[wasm_bindgen(constructor)]
        pub fn new(config_js: JsValue) -> Result<WasmProofVerifier, JsValue> {
            let config: ZKConfig = serde_wasm_bindgen::from_value(config_js)
                .map_err(|e| JsValue::from_str(&format!("配置解析失败: {}", e)))?;
            
            let verifier = ProofVerifier::new(config)
                .map_err(|e| JsValue::from_str(&format!("验证器创建失败: {}", e)))?;
            
            Ok(WasmProofVerifier { verifier })
        }
        
        #[wasm_bindgen]
        pub async fn verify(&self, proof_js: JsValue) -> Result<JsValue, JsValue> {
            let proof: ComputeProof = serde_wasm_bindgen::from_value(proof_js)
                .map_err(|e| JsValue::from_str(&format!("证明解析失败: {}", e)))?;
            
            let result = self.verifier.verify_proof_async(proof).await;
            
            serde_wasm_bindgen::to_value(&result)
                .map_err(|e| JsValue::from_str(&format!("结果序列化失败: {}", e)))
        }
        
        #[wasm_bindgen]
        pub async fn verify_batch(&self, proofs_js: JsValue) -> Result<JsValue, JsValue> {
            let proofs: Vec<ComputeProof> = serde_wasm_bindgen::from_value(proofs_js)
                .map_err(|e| JsValue::from_str(&format!("证明列表解析失败: {}", e)))?;
            
            let result = self.verifier.verify_batch(&proofs);
            
            serde_wasm_bindgen::to_value(&result)
                .map_err(|e| JsValue::from_str(&format!("结果序列化失败: {}", e)))
        }
    }
}

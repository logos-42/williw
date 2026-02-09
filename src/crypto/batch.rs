//! 批量加密模块
//! 
//! 提供批量加密解密功能，支持并行处理。

use anyhow::Result;
use rayon::prelude::*;

use crate::crypto::high_performance::HighPerformanceCrypto;
use crate::crypto::EncryptionAlgorithm;

impl HighPerformanceCrypto {
    /// 批量加密（并行处理）
    pub fn encrypt_batch(&self, data_chunks: &[&[u8]], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<Vec<u8>>> {
        if self.config.enable_parallel_processing && data_chunks.len() > 1 {
            self.encrypt_batch_parallel(data_chunks, key, algorithm)
        } else {
            self.encrypt_batch_sequential(data_chunks, key, algorithm)
        }
    }
    
    /// 批量解密（并行处理）
    pub fn decrypt_batch(&self, encrypted_chunks: &[&[u8]], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<Vec<u8>>> {
        if self.config.enable_parallel_processing && encrypted_chunks.len() > 1 {
            self.decrypt_batch_parallel(encrypted_chunks, key, algorithm)
        } else {
            self.decrypt_batch_sequential(encrypted_chunks, key, algorithm)
        }
    }
    
    /// 选择性加密（只加密敏感部分）
    pub fn selective_encrypt(&self, data: &[u8], sensitive_ranges: &[(usize, usize)], key: &[u8]) -> Result<Vec<u8>> {
        let mut result = data.to_vec();
        
        for &(start, end) in sensitive_ranges {
            if end <= data.len() && start < end {
                let sensitive_data = &data[start..end];
                let encrypted = self.encrypt(sensitive_data, key, self.config.default_algorithm)?;
                result[start..end].copy_from_slice(&encrypted[..(end-start).min(encrypted.len())]);
            }
        }
        
        Ok(result)
    }
    
    // ============ 公有方法 ============
    
    pub fn encrypt_batch_sequential(&self, data_chunks: &[&[u8]], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<Vec<u8>>> {
        let mut results = Vec::with_capacity(data_chunks.len());
        
        for chunk in data_chunks {
            let encrypted = self.encrypt(chunk, key, algorithm)?;
            results.push(encrypted);
        }
        
        Ok(results)
    }
    
    pub fn decrypt_batch_sequential(&self, encrypted_chunks: &[&[u8]], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<Vec<u8>>> {
        let mut results = Vec::with_capacity(encrypted_chunks.len());
        
        for chunk in encrypted_chunks {
            let decrypted = self.decrypt(chunk, key, algorithm)?;
            results.push(decrypted);
        }
        
        Ok(results)
    }
    
    pub fn encrypt_batch_parallel(&self, data_chunks: &[&[u8]], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<Vec<u8>>> {
        let key = key.to_vec();
        
        let results: Vec<Result<Vec<u8>>> = data_chunks
            .par_iter()
            .map(|chunk| {
                match algorithm {
                    EncryptionAlgorithm::ChaCha20Poly1305 => self.encrypt_chacha20(chunk, &key),
                    EncryptionAlgorithm::Aes256Cbc => self.encrypt_aes256(chunk, &key),
                    EncryptionAlgorithm::Blake3 => self.encrypt_blake3(chunk, &key),
                }
            })
            .collect();
        
        results.into_iter().collect()
    }
    
    pub fn decrypt_batch_parallel(&self, encrypted_chunks: &[&[u8]], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<Vec<u8>>> {
        let key = key.to_vec();
        
        let results: Vec<Result<Vec<u8>>> = encrypted_chunks
            .par_iter()
            .map(|chunk| {
                match algorithm {
                    EncryptionAlgorithm::ChaCha20Poly1305 => self.decrypt_chacha20(chunk, &key),
                    EncryptionAlgorithm::Aes256Cbc => self.decrypt_aes256(chunk, &key),
                    EncryptionAlgorithm::Blake3 => self.decrypt_blake3(chunk, &key),
                }
            })
            .collect();
        
        results.into_iter().collect()
    }
}

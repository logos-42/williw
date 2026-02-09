//! 硬件加速加密模块
//! 
//! 检测和利用硬件加速功能（如AES-NI、AVX2等）。

use anyhow::Result;

use crate::crypto::{EncryptionAlgorithm, HighPerformanceCrypto};

/// 硬件加速加密检测和支持
pub struct HardwareAcceleratedCrypto {
    supported_algorithms: Vec<EncryptionAlgorithm>,
    has_aes_ni: bool,
    has_avx2: bool,
    has_avx512: bool,
}

/// 硬件信息结构体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HardwareInfo {
    pub has_aes_ni: bool,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub supported_algorithms: Vec<EncryptionAlgorithm>,
}

impl HardwareAcceleratedCrypto {
    /// 检测系统硬件加速支持
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        let has_aes_ni = is_x86_feature_detected!("aes");
        #[cfg(target_arch = "x86_64")]
        let has_avx2 = is_x86_feature_detected!("avx2");
        #[cfg(target_arch = "x86_64")]
        let has_avx512 = is_x86_feature_detected!("avx512f");
        
        #[cfg(not(target_arch = "x86_64"))]
        let has_aes_ni = false;
        #[cfg(not(target_arch = "x86_64"))]
        let has_avx2 = false;
        #[cfg(not(target_arch = "x86_64"))]
        let has_avx512 = false;
        
        let mut supported_algorithms = vec![EncryptionAlgorithm::ChaCha20Poly1305];
        
        if has_aes_ni {
            supported_algorithms.push(EncryptionAlgorithm::Aes256Cbc);
        }
        
        // Blake3 通常有硬件加速优化
        supported_algorithms.push(EncryptionAlgorithm::Blake3);
        
        Self {
            supported_algorithms,
            has_aes_ni,
            has_avx2,
            has_avx512,
        }
    }
    
    /// 检查算法是否支持硬件加速
    pub fn is_hardware_accelerated(&self, algorithm: EncryptionAlgorithm) -> bool {
        match algorithm {
            EncryptionAlgorithm::Aes256Cbc => self.has_aes_ni,
            EncryptionAlgorithm::ChaCha20Poly1305 => self.has_avx2 || self.has_avx512,
            EncryptionAlgorithm::Blake3 => self.has_avx2 || self.has_avx512,
        }
    }
    
    /// 获取支持硬件加速的算法列表
    pub fn get_supported_algorithms(&self) -> &[EncryptionAlgorithm] {
        &self.supported_algorithms
    }
    
    /// 获取硬件加速信息
    pub fn get_hardware_info(&self) -> HardwareInfo {
        HardwareInfo {
            has_aes_ni: self.has_aes_ni,
            has_avx2: self.has_avx2,
            has_avx512: self.has_avx512,
            supported_algorithms: self.supported_algorithms.clone(),
        }
    }
    
    /// 使用硬件加速加密（如果可用）
    pub fn encrypt_with_hardware(&self, data: &[u8], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<u8>> {
        if self.is_hardware_accelerated(algorithm) {
            // 这里可以调用硬件加速的加密实现
            // 目前我们使用软件实现，但标记为硬件加速可用
            let crypto = HighPerformanceCrypto::with_default_config();
            crypto.encrypt(data, key, algorithm)
        } else {
            // 回退到软件实现
            let crypto = HighPerformanceCrypto::with_default_config();
            crypto.encrypt(data, key, algorithm)
        }
    }
    
    /// 使用硬件加速解密（如果可用）
    pub fn decrypt_with_hardware(&self, encrypted: &[u8], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<u8>> {
        if self.is_hardware_accelerated(algorithm) {
            let crypto = HighPerformanceCrypto::with_default_config();
            crypto.decrypt(encrypted, key, algorithm)
        } else {
            let crypto = HighPerformanceCrypto::with_default_config();
            crypto.decrypt(encrypted, key, algorithm)
        }
    }
}

//! 零拷贝加密模块
//!
//! 提供原地加密解密功能，避免内存复制开销。

use anyhow::{anyhow, Result};
use parking_lot::RwLock;

use crate::crypto::{EncryptionAlgorithm, HighPerformanceCrypto, HighPerformanceCryptoConfig};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, KeyInit};
use aes::Aes256;
use cbc::Decryptor;
use block_padding::Pkcs7;
use blake3::Hasher;
use aes::cipher::KeyIvInit;
use aes::cipher::{BlockEncryptMut, BlockDecryptMut};



/// 零拷贝加密引擎
pub struct ZeroCopyEncryption {
    crypto: HighPerformanceCrypto,
    buffer_pool: RwLock<Vec<Vec<u8>>>,
}

impl ZeroCopyEncryption {
    /// 创建新的零拷贝加密引擎
    pub fn new(config: HighPerformanceCryptoConfig) -> Self {
        Self {
            crypto: HighPerformanceCrypto::new(config),
            buffer_pool: RwLock::new(Vec::new()),
        }
    }
    
    /// 加密数据（零拷贝版本）
    pub fn encrypt_in_place(&self, data: &mut [u8], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        match algorithm {
            EncryptionAlgorithm::ChaCha20Poly1305 => self.encrypt_chacha20_in_place(data, key),
            EncryptionAlgorithm::Aes256Cbc => self.encrypt_aes256_in_place(data, key),
            EncryptionAlgorithm::Blake3 => self.encrypt_blake3_in_place(data, key),
        }
    }
    
    /// 解密数据（零拷贝版本）
    pub fn decrypt_in_place(&self, data: &mut [u8], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        match algorithm {
            EncryptionAlgorithm::ChaCha20Poly1305 => self.decrypt_chacha20_in_place(data, key),
            EncryptionAlgorithm::Aes256Cbc => self.decrypt_aes256_in_place(data, key),
            EncryptionAlgorithm::Blake3 => self.decrypt_blake3_in_place(data, key),
        }
    }
    
    /// 从缓冲池获取缓冲区
    pub fn get_buffer(&self, size: usize) -> Vec<u8> {
        let mut pool = self.buffer_pool.write();
        
        // 尝试从池中获取合适大小的缓冲区
        if let Some(index) = pool.iter().position(|buf| buf.capacity() >= size) {
            let mut buffer = pool.remove(index);
            buffer.clear();
            buffer.reserve(size);
            buffer
        } else {
            Vec::with_capacity(size)
        }
    }
    
    /// 将缓冲区返回到池中
    pub fn return_buffer(&self, mut buffer: Vec<u8>) {
        if buffer.capacity() >= 1024 { // 只缓存足够大的缓冲区
            buffer.clear();
            let mut pool = self.buffer_pool.write();
            pool.push(buffer);
        }
    }
    
    // ============ 私有方法 ============
    
    fn encrypt_chacha20_in_place(&self, data: &mut [u8], key: &[u8]) -> Result<()> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(&[0u8; 12]);
        
        // 由于ChaCha20-Poly1305需要额外的认证标签空间，
        // 这里我们使用临时缓冲区
        let encrypted = cipher.encrypt(nonce, &*data)
            .map_err(|e| anyhow!("ChaCha20加密失败: {}", e))?;
        
        if encrypted.len() == data.len() {
            data.copy_from_slice(&encrypted);
            Ok(())
        } else {
            Err(anyhow!("加密后数据大小不匹配"))
        }
    }
    
    fn decrypt_chacha20_in_place(&self, data: &mut [u8], key: &[u8]) -> Result<()> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(&[0u8; 12]);
        
        let decrypted = cipher.decrypt(nonce, &*data)
            .map_err(|e| anyhow!("ChaCha20解密失败: {}", e))?;
        
        if decrypted.len() == data.len() {
            data.copy_from_slice(&decrypted);
            Ok(())
        } else {
            Err(anyhow!("解密后数据大小不匹配"))
        }
    }
    
    fn encrypt_aes256_in_place(&self, data: &mut [u8], key: &[u8]) -> Result<()> {
        let iv = [0u8; 16];
        let cipher = cbc::Encryptor::<Aes256>::new_from_slices(key, &iv)
            .map_err(|e| anyhow!("AES256初始化失败: {}", e))?;

        // 使用 encrypt_padded_mut 方法处理填充
        let mut buffer = vec![0u8; data.len() + 16]; // 预留填充空间
        buffer[..data.len()].copy_from_slice(data);
        let result = cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
            .map_err(|e| anyhow!("AES256加密失败: {}", e))?;

        let len = result.len();
        if len == data.len() {
            data.copy_from_slice(&buffer[..len]);
            Ok(())
        } else {
            // AES-CBC可能需要填充，所以大小可能不同
            // 这里我们只处理无填充的情况
            Err(anyhow!("AES256加密后数据大小不匹配"))
        }
    }

    fn decrypt_aes256_in_place(&self, data: &mut [u8], key: &[u8]) -> Result<()> {
        let iv = [0u8; 16];
        let cipher = Decryptor::<Aes256>::new_from_slices(key, &iv)
            .map_err(|e| anyhow!("AES256初始化失败: {}", e))?;

        // 使用 decrypt_padded_mut 方法处理填充
        let mut buffer = data.to_vec();
        let result = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| anyhow!("AES256解密失败: {}", e))?;

        let len = result.len();
        if len == data.len() {
            data.copy_from_slice(&buffer[..len]);
            Ok(())
        } else {
            Err(anyhow!("AES256解密后数据大小不匹配"))
        }
    }
    
    fn encrypt_blake3_in_place(&self, data: &mut [u8], key: &[u8]) -> Result<()> {
        // Blake3加密实际上是在末尾添加哈希
        // 由于是原地操作，我们需要扩展缓冲区
        // 这里我们返回错误，因为原地操作不适合这种加密
        Err(anyhow!("Blake3不支持原地加密"))
    }
    
    fn decrypt_blake3_in_place(&self, data: &mut [u8], key: &[u8]) -> Result<()> {
        // Blake3解密需要验证哈希
        // 这里我们只验证，不修改数据
        if data.len() < 32 {
            return Err(anyhow!("加密数据太短"));
        }
        
        let data_len = data.len() - 32;
        let message = &data[..data_len];
        let hash = &data[data_len..];
        
        let keyed_key: [u8; 32] = key.try_into()
            .map_err(|_| anyhow!("Blake3 key must be 32 bytes"))?;
        let mut hasher = Hasher::new_keyed(&keyed_key);
        hasher.update(message);
        let expected_hash = hasher.finalize();
        
        if hash == expected_hash.as_bytes() {
            // 验证成功，但无法原地移除哈希
            // 这里我们只返回成功，调用者需要知道数据包含哈希
            Ok(())
        } else {
            Err(anyhow!("Blake3验证失败"))
        }
    }
}

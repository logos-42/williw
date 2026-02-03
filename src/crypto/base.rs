//! Solana 基础加密功能模块
//!
//! 提供基础的签名、验证功能，用于与 Solana 区块链和智能合约交互。

use anyhow::{anyhow, Result};
use ed25519_dalek::{SigningKey as SolKeypair,
    Signature as SolRawSignature, Signer as SolSigner};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::sync::Arc;

/// Solana签名结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolSignature {
    pub pubkey: String,
    pub signature: String,
}

/// 加密配置（仅 Solana）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    pub sol_bs58_seed: Option<String>,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            sol_bs58_seed: None,
        }
    }
}

/// Solana 加密套件 - 用于与 Solana 区块链交互
#[derive(Clone)]
pub struct SolanaCryptoSuite {
    sol: Arc<SolIdentity>,
}

impl SolanaCryptoSuite {
    /// 创建新的 Solana 加密套件
    pub fn new(config: CryptoConfig) -> Result<Self> {
        let sol = SolIdentity::new(config.sol_bs58_seed)?;
        Ok(Self {
            sol: Arc::new(sol),
        })
    }

    /// 对数据进行签名
    pub fn sign_bytes(&self, payload: &[u8]) -> Result<SolSignature> {
        let sol_sig = self.sol.sign(payload)?;
        Ok(sol_sig)
    }

    /// 验证签名
    pub fn verify(&self, payload: &[u8], sig: &SolSignature) -> bool {
        self.sol.verify(payload, sig)
    }

    /// 获取Solana地址
    pub fn sol_address(&self) -> String {
        self.sol.pubkey.clone()
    }

    /// 获取密钥对（用于与 Solana 程序库交互）
    pub fn keypair(&self) -> &SolKeypair {
        self.sol.keypair()
    }
}

/// Solana身份（内部使用）
struct SolIdentity {
    keypair: SolKeypair,
    pubkey: String,
}

impl SolIdentity {
    fn new(seed: Option<String>) -> Result<Self> {
        let keypair = if let Some(bs58_seed) = seed {
            let bytes = bs58::decode(bs58_seed).into_vec()?;
            match bytes.len() {
                32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    keypair_from_secret(arr)
                }
                64 => {
                    let mut arr = [0u8; 64];
                    arr.copy_from_slice(&bytes);
                    let key_bytes: [u8; 32] = arr[..32].try_into().map_err(|_| anyhow!("Failed to extract key bytes"))?;
                    SolKeypair::from_bytes(&key_bytes)
                }
                _ => return Err(anyhow!("Solana seed must be 32 or 64 bytes")),
            }
        } else {
            keypair_from_secret(random_bytes())
        };
        let pubkey = bs58::encode(keypair.verifying_key().as_bytes()).into_string();
        Ok(Self { keypair, pubkey })
    }

    fn sign(&self, payload: &[u8]) -> Result<SolSignature> {
        let signature = self.keypair.sign(payload);
        Ok(SolSignature {
            pubkey: self.pubkey.clone(),
            signature: bs58::encode(signature.to_bytes()).into_string(),
        })
    }

    fn verify(&self, payload: &[u8], sig: &SolSignature) -> bool {
        if sig.pubkey != self.pubkey {
            return false;
        }
        if let Ok(bytes) = bs58::decode(&sig.signature).into_vec() {
            if let Ok(signature) = SolRawSignature::try_from(&bytes[..]) {
                return self.keypair.verify(payload, &signature).is_ok();
            }
        }
        false
    }

    /// 获取密钥对引用
    fn keypair(&self) -> &SolKeypair {
        &self.keypair
    }
}

/// 生成随机字节
fn random_bytes() -> [u8; 32] {
    let mut buf = [0u8; 32];
    rand::rng().fill_bytes(&mut buf);
    buf
}

/// 从密钥生成Solana密钥对
fn keypair_from_secret(secret_bytes: [u8; 32]) -> SolKeypair {
    // In ed25519-dalek 2.2.0, from_bytes returns key directly
    SolKeypair::from_bytes(&secret_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_crypto_basic() {
        let config = CryptoConfig::default();
        let suite = SolanaCryptoSuite::new(config).unwrap();

        let payload = b"test payload";
        let signature = suite.sign_bytes(payload).unwrap();

        // 验证签名
        assert!(suite.verify(payload, &signature));

        // 验证错误的payload
        assert!(!suite.verify(b"wrong payload", &signature));

        // 获取地址
        let sol_addr = suite.sol_address();

        // Solana地址是base58编码
        assert!(!sol_addr.is_empty());
    }

    #[test]
    fn test_solana_crypto_with_seed() {
        let sol_seed = "5uHqkH1kq3qjJcK7vqkH1kq3qjJcK7vqkH1kq3qjJcK7vqkH1kq3qjJcK7v"; // base58 encoded 32 bytes

        let config = CryptoConfig {
            sol_bs58_seed: Some(sol_seed.to_string()),
        };

        let suite = SolanaCryptoSuite::new(config).unwrap();

        // 使用相同种子应该生成相同的地址
        let suite2 = SolanaCryptoSuite::new(CryptoConfig {
            sol_bs58_seed: Some(sol_seed.to_string()),
        }).unwrap();

        assert_eq!(suite.sol_address(), suite2.sol_address());
    }

    #[test]
    fn test_random_bytes() {
        let bytes1 = random_bytes();
        let bytes2 = random_bytes();

        assert_eq!(bytes1.len(), 32);
        assert_eq!(bytes2.len(), 32);

        // 随机生成的字节应该不同（极低概率相同）
        assert_ne!(bytes1, bytes2);
    }
}

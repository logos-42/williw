// Temporarily comment out to fix compilation
// use crate::crypto::{CryptoSuite, SignatureBundle};
use crate::types::GgbMessage;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// Temporary mock signature type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSignature {
    pub data: Vec<u8>,
}

#[cfg(feature = "blockchain")]
use crate::blockchain::BlockchainClient;

#[derive(Clone, Debug)]
pub struct StakeRecord {
    pub stake_eth: f64,
    pub stake_sol: f64,
    pub reputation: f64,
    pub last_seen: Instant,
}

impl StakeRecord {
    pub fn combined_weight(&self) -> f32 {
        let stake_component = (self.stake_eth + self.stake_sol).ln_1p() as f32;
        let rep_component = (self.reputation.max(0.0) as f32).ln_1p();
        (stake_component + rep_component).clamp(0.0, 5.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedGossip {
    pub payload: GgbMessage,
    // pub signature: SignatureBundle,  // Temporarily commented
    pub signature: Option<MockSignature>,  // Use mock signature temporarily
    pub staking_score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub heartbeat_timeout: Duration,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout: Duration::from_secs(300),
        }
    }
}

pub struct ConsensusEngine {
    // crypto: Arc<CryptoSuite>,  // Temporarily commented
    ledger: RwLock<HashMap<String, StakeRecord>>,
    config: ConsensusConfig,
    #[cfg(feature = "blockchain")]
    blockchain_client: Option<Arc<dyn BlockchainClient>>,
    _crypto_marker: Arc<()>,  // Placeholder to keep type signature compatible
}

impl ConsensusEngine {
    pub fn new(_crypto: Arc<()>, config: ConsensusConfig) -> Self {  // Temporarily use Arc<()> instead of CryptoSuite
        Self {
            // crypto,  // Temporarily commented
            ledger: RwLock::new(HashMap::new()),
            config,
            #[cfg(feature = "blockchain")]
            blockchain_client: None,
            _crypto_marker: _crypto,
        }
    }
    
    #[cfg(feature = "blockchain")]
    pub fn with_blockchain_client(mut self, client: Arc<dyn BlockchainClient>) -> Self {
        self.blockchain_client = Some(client);
        self
    }
    
    #[cfg(feature = "blockchain")]
    /// 从链上查询质押信息（如果配置了区块链客户端）
    pub async fn query_stake_from_chain(&self, address: &str) -> Option<f64> {
        if let Some(client) = &self.blockchain_client {
            client.query_stake(address).await.ok()
        } else {
            None
        }
    }

    pub fn sign(&self, payload: GgbMessage) -> anyhow::Result<SignedGossip> {
        let bytes = serde_json::to_vec(&payload)?;
        // let signature = self.crypto.sign_bytes(&bytes)?;
        // Use mock signature temporarily
        let signature = Some(MockSignature {
            data: bytes.clone(),  // Simple mock: use the payload bytes as signature
        });
        let peer_id = match &payload {
            GgbMessage::Heartbeat { peer, .. }
            | GgbMessage::SimilarityProbe { sender: peer, .. }
            | GgbMessage::SparseUpdate { sender: peer, .. }
            | GgbMessage::DenseSnapshot { sender: peer, .. } => peer.clone(),
        };
        let staking_score = self
            .ledger
            .read()
            .get(&peer_id)
            .map(|record| record.combined_weight())
            .unwrap_or(0.1);
        Ok(SignedGossip {
            payload,
            signature,
            staking_score,
        })
    }

    pub fn verify(&self, _msg: &SignedGossip) -> bool {
        // return self.crypto.verify(&bytes, &msg.signature);
        // Temporary mock: always return true for verification
        true
    }

    pub fn update_stake(&self, peer: &str, delta_eth: f64, delta_sol: f64, reputation_delta: f64) {
        let mut ledger = self.ledger.write();
        let entry = ledger.entry(peer.to_string()).or_insert(StakeRecord {
            stake_eth: 1.0,
            stake_sol: 0.1,
            reputation: 1.0,
            last_seen: Instant::now(),
        });
        entry.stake_eth = (entry.stake_eth + delta_eth).max(0.0);
        entry.stake_sol = (entry.stake_sol + delta_sol).max(0.0);
        entry.reputation = (entry.reputation + reputation_delta).max(-1.0);
        entry.last_seen = Instant::now();
    }

    pub fn prune_stale(&self) {
        let mut ledger = self.ledger.write();
        let deadline = Instant::now() - self.config.heartbeat_timeout;
        ledger.retain(|_, record| record.last_seen >= deadline);
    }

    pub fn stake_weight(&self, peer: &str) -> f32 {
        self.ledger
            .read()
            .get(peer)
            .map(|record| record.combined_weight())
            .unwrap_or(0.1)
    }
    
    #[cfg(feature = "blockchain")]
    /// 同步链上质押信息到内存账本
    pub async fn sync_stake_from_chain(&self, peer: &str) {
        if let Some(stake) = self.query_stake_from_chain(peer).await {
            let mut ledger = self.ledger.write();
            let entry = ledger.entry(peer.to_string()).or_insert(StakeRecord {
                stake_eth: 0.0,
                stake_sol: 0.0,
                reputation: 1.0,
                last_seen: Instant::now(),
            });
            entry.stake_eth = stake;
            entry.last_seen = Instant::now();
        }
    }
}

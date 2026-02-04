/**
 * P2P 模型分发模块
 * 基于 iroh 实现点对点的模型文件传输
 */

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error, debug};

/// 文件传输消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTransferMessage {
    /// 请求传输文件
    FileRequest {
        file_id: String,
        file_name: String,
        file_size: u64,
        chunk_size: usize,
        file_hash: String,
    },
    /// 响应文件请求
    FileResponse {
        file_id: String,
        accepted: bool,
        reason: Option<String>,
    },
    /// 文件数据块
    FileChunk {
        file_id: String,
        chunk_index: u32,
        data: Vec<u8>,
        chunk_hash: String,
    },
    /// 文件传输完成
    FileComplete {
        file_id: String,
        total_chunks: u32,
        final_hash: String,
    },
    /// 传输进度报告
    ProgressReport {
        file_id: String,
        chunks_received: u32,
        total_chunks: u32,
        percentage: f32,
    },
    /// 传输错误
    TransferError {
        file_id: String,
        error: String,
    },
}

/// 文件传输状态
#[derive(Debug, Clone)]
pub enum TransferStatus {
    Pending,
    Accepted,
    InProgress { chunks_received: u32, total_chunks: u32 },
    Completed,
    Failed(String),
}

/// 文件传输会话
#[derive(Debug, Clone)]
pub struct TransferSession {
    pub file_id: String,
    pub file_name: String,
    pub file_path: PathBuf,
    pub file_size: u64,
    pub chunk_size: usize,
    pub total_chunks: u32,
    pub status: TransferStatus,
    pub chunks_received: HashMap<u32, Vec<u8>>,
    pub file_hash: String,
    pub created_at: std::time::Instant,
}

impl TransferSession {
    pub fn new(file_id: String, file_name: String, file_path: PathBuf, 
               file_size: u64, chunk_size: usize, file_hash: String) -> Self {
        let total_chunks = (file_size as usize + chunk_size - 1) / chunk_size;
        Self {
            file_id,
            file_name,
            file_path,
            file_size,
            chunk_size,
            total_chunks: total_chunks as u32,
            status: TransferStatus::Pending,
            chunks_received: HashMap::new(),
            file_hash,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn add_chunk(&mut self, chunk_index: u32, data: Vec<u8>) -> Result<()> {
        if chunk_index >= self.total_chunks {
            return Err(anyhow!("块索引超出范围: {} >= {}", chunk_index, self.total_chunks));
        }

        self.chunks_received.insert(chunk_index, data);
        
        let received = self.chunks_received.len() as u32;
        self.status = TransferStatus::InProgress {
            chunks_received: received,
            total_chunks: self.total_chunks,
        };

        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.chunks_received.len() == self.total_chunks as usize
    }

    pub fn get_progress(&self) -> f32 {
        if self.total_chunks == 0 {
            return 100.0;
        }
        (self.chunks_received.len() as f32 / self.total_chunks as f32) * 100.0
    }
}

/// P2P 模型分发器
pub struct P2PModelDistributor {
    node_id: String,
    active_transfers: Arc<RwLock<HashMap<String, TransferSession>>>,
    message_tx: mpsc::Sender<(String, FileTransferMessage)>,
    message_rx: mpsc::Receiver<(String, FileTransferMessage)>,
}

impl P2PModelDistributor {
    pub fn new(node_id: String) -> Self {
        let (message_tx, message_rx) = mpsc::channel(1000);
        
        Self {
            node_id,
            active_transfers: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            message_rx,
        }
    }

    /// 发送文件到指定节点
    pub async fn send_file(&mut self, 
                          peer_id: String, 
                          file_path: &Path,
                          chunk_size: Option<usize>) -> Result<String> {
        let file_path = file_path.to_path_buf();
        
        // 检查文件是否存在
        if !file_path.exists() {
            return Err(anyhow!("文件不存在: {}", file_path.display()));
        }

        // 读取文件信息
        let metadata = fs::metadata(&file_path).await?;
        let file_size = metadata.len();
        let file_name = file_path.file_name()
            .ok_or_else(|| anyhow!("无效的文件名"))?
            .to_string_lossy()
            .to_string();

        // 计算文件哈希
        let file_hash = self.calculate_file_hash(&file_path).await?;
        
        // 生成文件传输ID
        let file_id = uuid::Uuid::new_v4().to_string();
        
        // 设置块大小（默认1MB）
        let chunk_size = chunk_size.unwrap_or(1024 * 1024);
        
        info!("开始发送文件: {} (大小: {} bytes, 哈希: {})", 
              file_name, file_size, file_hash);

        // 发送文件请求
        let request = FileTransferMessage::FileRequest {
            file_id: file_id.clone(),
            file_name: file_name.clone(),
            file_size,
            chunk_size,
            file_hash: file_hash.clone(),
        };

        self.send_message(&peer_id, request).await?;

        // 等待响应（简化实现，实际应该有超时）
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 开始发送文件块
        self.send_file_chunks(&peer_id, &file_path, &file_id, chunk_size).await?;

        Ok(file_id)
    }

    /// 发送文件块
    async fn send_file_chunks(&mut self, 
                              peer_id: &str, 
                              file_path: &Path,
                              file_id: &str,
                              chunk_size: usize) -> Result<()> {
        let mut file = fs::File::open(file_path).await?;
        let mut buffer = vec![0u8; chunk_size];
        let mut chunk_index = 0u32;

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let chunk_data = buffer[..bytes_read].to_vec();
            let chunk_hash = self.calculate_chunk_hash(&chunk_data);

            let chunk_message = FileTransferMessage::FileChunk {
                file_id: file_id.to_string(),
                chunk_index,
                data: chunk_data,
                chunk_hash,
            };

            self.send_message(peer_id, chunk_message).await?;
            
            debug!("发送块 {} 到 {}", chunk_index, peer_id);
            chunk_index += 1;

            // 添加小延迟避免过快发送
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // 发送完成消息
        let complete_message = FileTransferMessage::FileComplete {
            file_id: file_id.to_string(),
            total_chunks: chunk_index,
            final_hash: String::new(), // 可以计算整个文件的哈希
        };

        self.send_message(peer_id, complete_message).await?;
        
        info!("文件传输完成: {} ({} 块)", file_id, chunk_index);
        
        Ok(())
    }

    /// 接收文件
    pub async fn receive_file(&mut self, 
                             output_dir: &Path,
                             file_request: FileTransferMessage) -> Result<String> {
        if let FileTransferMessage::FileRequest { 
            file_id, 
            file_name, 
            file_size, 
            chunk_size, 
            file_hash 
        } = file_request {
            
            info!("接收文件请求: {} (大小: {} bytes)", file_name, file_size);

            // 创建传输会话
            let output_path = output_dir.join(&file_name);
            let session = TransferSession::new(
                file_id.clone(),
                file_name,
                output_path,
                file_size,
                chunk_size,
                file_hash,
            );

            {
                let mut transfers = self.active_transfers.write().await;
                transfers.insert(file_id.clone(), session);
            }

            // 发送接受响应
            let _response = FileTransferMessage::FileResponse {
                file_id: file_id.clone(),
                accepted: true,
                reason: None,
            };

            // 这里需要知道发送方ID，简化处理
            // self.send_message(&sender_id, _response).await;

            Ok(file_id)
        } else {
            Err(anyhow!("无效的文件请求消息"))
        }
    }

    /// 处理接收到的文件块
    pub async fn handle_file_chunk(&mut self,
                                   _sender_id: String,
                                   chunk_message: FileTransferMessage) -> Result<()> {
        if let FileTransferMessage::FileChunk {
            file_id,
            chunk_index,
            data,
            chunk_hash,
        } = chunk_message {

            // 验证块哈希
            let calculated_hash = self.calculate_chunk_hash(&data);
            if calculated_hash != chunk_hash {
                error!("块哈希验证失败: 文件 {} 块 {}", file_id, chunk_index);
                return Err(anyhow!("块哈希验证失败"));
            }

            // 添加到传输会话
            let should_assemble = {
                let mut transfers = self.active_transfers.write().await;
                if let Some(session) = transfers.get_mut(&file_id) {
                    session.add_chunk(chunk_index, data)?;
                    
                    let progress = session.get_progress();
                    debug!("文件 {} 进度: {:.1}%", file_id, progress);

                    // 检查是否完成
                    if session.is_complete() {
                        info!("文件 {} 接收完成，开始组装文件", file_id);
                        true
                    } else {
                        false
                    }
                } else {
                    warn!("未找到文件传输会话: {}", file_id);
                    false
                }
            };

            // 在锁外组装文件
            if should_assemble {
                self.assemble_file(&file_id).await?;
            }
        }

        Ok(())
    }

    /// 组装文件
    async fn assemble_file(&mut self, file_id: &str) -> Result<()> {
        let session = {
            let transfers = self.active_transfers.read().await;
            transfers.get(file_id).cloned()
        };

        if let Some(session) = session {
            info!("组装文件: {}", session.file_name);
            
            // 创建输出文件
            let mut output_file = fs::File::create(&session.file_path).await?;
            
            // 按顺序写入所有块
            for chunk_index in 0..session.total_chunks {
                if let Some(chunk_data) = session.chunks_received.get(&chunk_index) {
                    output_file.write_all(chunk_data).await?;
                } else {
                    return Err(anyhow!("缺少块: {}", chunk_index));
                }
            }

            output_file.flush().await?;

            // 验证文件哈希
            let received_hash = self.calculate_file_hash(&session.file_path).await?;
            if received_hash != session.file_hash {
                error!("文件哈希验证失败: 期望 {}, 实际 {}", 
                       session.file_hash, received_hash);
                return Err(anyhow!("文件哈希验证失败"));
            }

            info!("文件组装完成: {} (大小: {} bytes)", 
                  session.file_name, session.file_size);

            // 更新状态
            {
                let mut transfers = self.active_transfers.write().await;
                if let Some(session) = transfers.get_mut(file_id) {
                    session.status = TransferStatus::Completed;
                }
            }
        }

        Ok(())
    }

    /// 发送消息
    async fn send_message(&mut self, peer_id: &str, message: FileTransferMessage) -> Result<()> {
        // 这里应该通过iroh发送消息，目前简化实现
        let _ = (peer_id, message);
        
        // 模拟发送到消息队列
        // self.message_tx.send((peer_id.to_string(), message)).await?;
        
        Ok(())
    }

    /// 计算文件哈希
    async fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        use sha3::{Sha3_256, Digest};
        
        let mut file = fs::File::open(file_path).await?;
        let mut hasher = Sha3_256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// 计算块哈希
    fn calculate_chunk_hash(&self, data: &[u8]) -> String {
        use sha3::{Sha3_256, Digest};
        
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// 获取传输状态
    pub async fn get_transfer_status(&self, file_id: &str) -> Option<TransferStatus> {
        let transfers = self.active_transfers.read().await;
        transfers.get(file_id).map(|session| session.status.clone())
    }

    /// 获取所有活跃传输
    pub async fn get_active_transfers(&self) -> Vec<String> {
        let transfers = self.active_transfers.read().await;
        transfers.keys().cloned().collect()
    }

    /// 清理完成的传输
    pub async fn cleanup_completed_transfers(&self) {
        let mut transfers = self.active_transfers.write().await;
        transfers.retain(|_, session| {
            !matches!(session.status, TransferStatus::Completed)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_transfer_session() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // 创建测试文件
        fs::write(&file_path, b"Hello, World!").await.unwrap();
        
        let session = TransferSession::new(
            "test_id".to_string(),
            "test.txt".to_string(),
            file_path,
            13,
            5,
            "test_hash".to_string(),
        );

        assert_eq!(session.total_chunks, 3);
        assert_eq!(session.get_progress(), 0.0);
        assert!(!session.is_complete());
    }

    #[tokio::test]
    async fn test_p2p_distributor_creation() {
        let distributor = P2PModelDistributor::new("test_node".to_string());
        assert_eq!(distributor.node_id, "test_node");
    }
}

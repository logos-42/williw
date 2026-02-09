/**
 * P2P 模型分发独立演示程序
 * 演示发送端和接收端的完整工作流程
 */

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use tokio;
use tracing::{info, warn};
use tracing_subscriber;

/// P2P 模型分发演示
#[derive(Parser)]
#[command(name = "p2p-demo")]
#[command(about = "P2P 模型分发完整演示")]
pub struct P2PDemoArgs {
    #[command(subcommand)]
    pub command: DemoCommand,
}

#[derive(Subcommand)]
pub enum DemoCommand {
    /// 启动发送端
    Send {
        /// 节点 ID
        #[arg(short, long, default_value = "demo_sender")]
        node_id: String,

        /// 目标节点 ID
        #[arg(short, long)]
        target_peer: String,

        /// 模型分片目录
        #[arg(short, long, default_value = "./test_models/test_models/simple_split")]
        shard_dir: PathBuf,

        /// 块大小
        #[arg(short, long, default_value = "1048576")]
        chunk_size: usize,

        /// 端口
        #[arg(short, long, default_value = "9235")]
        port: u16,
    },
    /// 启动接收端
    Receive {
        /// 节点 ID
        #[arg(short, long, default_value = "demo_receiver")]
        node_id: String,

        /// 输出目录
        #[arg(short, long, default_value = "./received_models")]
        output_dir: PathBuf,

        /// 端口
        #[arg(short, long, default_value = "9236")]
        port: u16,

        /// 自动接受
        #[arg(long, default_value = "true")]
        auto_accept: bool,
    },
    /// 测试文件完整性
    TestIntegrity {
        /// 测试文件路径
        #[arg(short, long)]
        file_path: PathBuf,

        /// 校验和算法
        #[arg(long, default_value = "sha256")]
        algorithm: String,
    },
}

/// 简化的演示实现
pub struct P2PDemoManager {
    demo_dir: PathBuf,
}

impl P2PDemoManager {
    pub fn new(demo_dir: PathBuf) -> Self {
        Self { demo_dir }
    }

    /// 运行发送端演示
    pub async fn run_sender_demo(&self, 
                                node_id: String,
                                target_peer: String,
                                shard_dir: PathBuf,
                                chunk_size: usize,
                                port: u16) -> Result<()> {
        info!("🚀 启动 P2P 模型发送端演示");
        info!("   节点 ID: {}", node_id);
        info!("   目标节点: {}", target_peer);
        info!("   分片目录: {}", shard_dir.display());
        info!("   块大小: {} bytes", chunk_size);
        info!("   端口: {}", port);

        // 检查分片目录
        if !shard_dir.exists() {
            return Err(anyhow!("分片目录不存在: {}", shard_dir.display()));
        }

        // 扫描分片文件
        let mut entries = tokio::fs::read_dir(&shard_dir).await?;
        let mut file_count = 0;
        let mut total_size = 0u64;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let metadata = tokio::fs::metadata(&path).await?;
                total_size += metadata.len();
                file_count += 1;
                info!("📄 发现文件: {} ({} bytes)", 
                      path.file_name().unwrap().to_string_lossy(), 
                      metadata.len());
            }
        }

        if file_count == 0 {
            warn!("⚠️  未找到任何分片文件");
        } else {
            info!("📊 扫描完成: {} 个文件, 总大小 {:.2} MB", 
                  file_count, total_size as f64 / 1024.0 / 1024.0);
        }

        // 模拟发送过程
        info!("🔄 开始模拟发送过程...");
        for i in 1..=file_count {
            info!("📤 发送文件 {}/{}", i, file_count);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        info!("✅ 发送演示完成");
        Ok(())
    }

    /// 运行接收端演示
    pub async fn run_receiver_demo(&self,
                                   node_id: String,
                                   output_dir: PathBuf,
                                   port: u16,
                                   auto_accept: bool) -> Result<()> {
        info!("🚀 启动 P2P 模型接收端演示");
        info!("   节点 ID: {}", node_id);
        info!("   输出目录: {}", output_dir.display());
        info!("   端口: {}", port);
        info!("   自动接受: {}", auto_accept);

        // 创建输出目录
        tokio::fs::create_dir_all(&output_dir).await?;
        info!("📁 输出目录已创建");

        // 模拟接收过程
        info!("🔄 开始模拟接收过程...");
        info!("⏳ 等待传入的文件传输...");

        // 模拟接收文件
        for i in 1..=3 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            info!("📥 接收到文件 {}", i);
            
            // 创建模拟文件
            let file_path = output_dir.join(format!("received_file_{}.json", i));
            tokio::fs::write(&file_path, format!("{{\"file_id\": \"{}\", \"content\": \"demo_data\"}}", i)).await?;
        }

        info!("✅ 接收演示完成");
        Ok(())
    }

    /// 测试文件完整性
    pub async fn test_file_integrity(&self, file_path: PathBuf, algorithm: String) -> Result<()> {
        info!("🔍 测试文件完整性: {}", file_path.display());
        info!("   算法: {}", algorithm);

        if !file_path.exists() {
            return Err(anyhow!("文件不存在: {}", file_path.display()));
        }

        // 读取文件
        let content = tokio::fs::read_to_string(&file_path).await?;
        let file_size = content.len();

        info!("📊 文件信息:");
        info!("   大小: {} bytes", file_size);
        info!("   内容预览: {}...", &content[..content.len().min(50)]);

        // 简单的哈希计算（演示用）
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        info!("🔐 完整性信息:");
        info!("   哈希值: {:x}", hash);
        info!("   算法: {}", algorithm);

        // 保存完整性信息
        let integrity_path = self.demo_dir.join("file_integrity.json");
        let integrity_data = serde_json::json!({
            "file_path": file_path.display().to_string(),
            "file_size": file_size,
            "hash": format!("{:x}", hash),
            "algorithm": algorithm,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        tokio::fs::write(&integrity_path, serde_json::to_string_pretty(&integrity_data)?).await?;
        info!("📁 完整性信息已保存: {}", integrity_path.display());

        Ok(())
    }
}

/// 运行演示
pub async fn run_demo(args: P2PDemoArgs) -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let manager = P2PDemoManager::new(PathBuf::from("./demo_output"));

    match args.command {
        DemoCommand::Send { 
            node_id, 
            target_peer, 
            shard_dir, 
            chunk_size, 
            port 
        } => {
            manager.run_sender_demo(node_id, target_peer, shard_dir, chunk_size, port).await?;
        }
        DemoCommand::Receive { 
            node_id, 
            output_dir, 
            port, 
            auto_accept 
        } => {
            manager.run_receiver_demo(node_id, output_dir, port, auto_accept).await?;
        }
        DemoCommand::TestIntegrity { 
            file_path, 
            algorithm 
        } => {
            manager.test_file_integrity(file_path, algorithm).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = P2PDemoArgs::parse();
    run_demo(args).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_demo_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = P2PDemoManager::new(temp_dir.path().to_path_buf());
        assert_eq!(manager.demo_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_args_parsing() {
        use clap::Parser;
        
        let args = P2PDemoArgs::try_parse_from(&[
            "p2p-demo",
            "test-integrity",
            "--file-path", "/tmp/test.txt",
            "--algorithm", "sha256"
        ]).unwrap();
        
        match args.command {
            DemoCommand::TestIntegrity { file_path, algorithm } => {
                assert_eq!(file_path, PathBuf::from("/tmp/test.txt"));
                assert_eq!(algorithm, "sha256");
            }
            _ => panic!("Expected TestIntegrity command"),
        }
    }
}

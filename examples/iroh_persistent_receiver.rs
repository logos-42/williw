/**
 * 常驻iroh接收端节点
 * 持续运行，接收来自任何发送端的消息
 */

use anyhow::Result;
use clap::Parser;
use iroh::{Endpoint, endpoint::Connection};
use iroh::endpoint_info::EndpointIdExt;
use tracing::{info, error, warn};
use tracing_subscriber;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// 常驻iroh接收端
#[derive(Parser)]
#[command(name = "iroh-persistent-receiver")]
#[command(about = "常驻iroh接收端节点")]
pub struct Args {
    /// 绑定端口
    #[arg(long, default_value = "9234")]
    port: u16,
    
    /// 节点名称
    #[arg(long, default_value = "persistent-receiver")]
    name: String,
    
    /// 日志级别
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// 文件接收目录
    #[arg(long, default_value = "./received_files")]
    receive_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let args = Args::parse();
    
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level));
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    info!("🚀 启动常驻iroh接收端节点");
    info!("📛 节点名称: {}", args.name);
    info!("🔗 绑定端口: {}", args.port);
    info!("📁 文件接收目录: {}", args.receive_dir.display());
    
    // 确保接收目录存在
    if !args.receive_dir.exists() {
        fs::create_dir_all(&args.receive_dir).await?;
        info!("✅ 创建接收目录: {}", args.receive_dir.display());
    }
    
    // 创建统计信息
    let stats = Arc::new(ReceiverStats::new());
    
    // 启动接收端
    start_persistent_receiver(args.port, args.name, args.receive_dir, stats).await
}

/// 接收端统计信息
struct ReceiverStats {
    message_count: AtomicU64,
    file_count: AtomicU64,
    connection_count: AtomicU64,
    last_message_time: RwLock<Option<SystemTime>>,
    connected_nodes: RwLock<HashMap<String, NodeInfo>>,
}

/// 传输类型枚举
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum TransferData {
    #[serde(rename = "message")]
    Message { content: String },
    #[serde(rename = "file")]
    File { 
        filename: String,
        size: u64,
        content: Vec<u8>,
    },
}

impl ReceiverStats {
    fn new() -> Self {
        Self {
            message_count: AtomicU64::new(0),
            file_count: AtomicU64::new(0),
            connection_count: AtomicU64::new(0),
            last_message_time: RwLock::new(None),
            connected_nodes: RwLock::new(HashMap::new()),
        }
    }
    
    async fn increment_message(&self) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
        *self.last_message_time.write().await = Some(SystemTime::now());
    }
    
    async fn increment_file(&self) {
        self.file_count.fetch_add(1, Ordering::Relaxed);
        *self.last_message_time.write().await = Some(SystemTime::now());
    }
    
    async fn increment_connection(&self) {
        self.connection_count.fetch_add(1, Ordering::Relaxed);
    }
    
    async fn add_node(&self, node_id: String) {
        let mut nodes = self.connected_nodes.write().await;
        let now = SystemTime::now();
        
        match nodes.get_mut(&node_id) {
            Some(info) => {
                info.message_count += 1;
                info.last_seen = now;
            }
            None => {
                nodes.insert(node_id.clone(), NodeInfo {
                    node_id: node_id.clone(),
                    first_seen: now,
                    last_seen: now,
                    message_count: 1,
                });
            }
        }
    }
    
    async fn get_stats(&self) -> StatsDisplay {
        let message_count = self.message_count.load(Ordering::Relaxed);
        let connection_count = self.connection_count.load(Ordering::Relaxed);
        let last_message = *self.last_message_time.read().await;
        let connected_nodes = self.connected_nodes.read().await.clone();
        
        StatsDisplay {
            message_count,
            connection_count,
            last_message,
            connected_nodes,
        }
    }
}

/// 节点信息
#[derive(Clone)]
struct NodeInfo {
    node_id: String,
    first_seen: SystemTime,
    last_seen: SystemTime,
    message_count: u64,
}

/// 统计显示
struct StatsDisplay {
    message_count: u64,
    connection_count: u64,
    last_message: Option<SystemTime>,
    connected_nodes: HashMap<String, NodeInfo>,
}

/// 启动常驻接收端
async fn start_persistent_receiver(port: u16, name: String, receive_dir: PathBuf, stats: Arc<ReceiverStats>) -> Result<()> {
    // 创建端点
    let endpoint = Endpoint::builder()
        .bind_addr_v4(format!("127.0.0.1:{}", port).parse::<std::net::SocketAddrV4>()?)
        .alpns(vec![b"robust".to_vec()])
        .bind()
        .await?;
    
    let node_id = endpoint.id().to_z32();
    
    info!("🎉 ===== 常驻iroh接收端启动成功 =====");
    info!("📛 节点名称: {}", name);
    info!("🔑 节点ID: {}", node_id);
    info!("📍 监听端口: {}", port);
    info!("📁 文件接收目录: {}", receive_dir.display());
    info!("📋 发送消息命令:");
    info!("   cargo run --example iroh_robust_local -- send --target {} --port {} --message \"Hello\"", node_id, port);
    info!("📋 发送文件命令:");
    info!("   cargo run --example iroh_robust_local -- send-file --target {} --port {} --file \"path/to/file.txt\"", node_id, port);
    info!("⏹️  按 Ctrl+C 停止");
    info!("========================================");
    
    let receive_dir = Arc::new(receive_dir);
    
    // 启动统计信息显示任务
    let stats_clone = stats.clone();
    let node_id_clone = node_id.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            display_stats(&stats_clone, &node_id_clone).await;
        }
    });
    
    // 主接收循环
    loop {
        info!("👂 等待连接...");
        
        match endpoint.accept().await {
            Some(incoming) => {
                info!("🔗 收到连接请求");
                
                match incoming.accept() {
                    Ok(accepting) => {
                        let stats_clone = stats.clone();
                        let receive_dir_clone = receive_dir.clone();
                        tokio::spawn(async move {
                            match accepting.await {
                                Ok(connection) => {
                                    handle_connection(connection, receive_dir_clone, stats_clone).await;
                                }
                                Err(e) => {
                                    error!("❌ 连接建立失败: {}", e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("❌ 接受连接失败: {}", e);
                    }
                }
            }
            None => {
                warn!("⏹️ 端点关闭");
                break;
            }
        }
    }
    
    Ok(())
}

/// 处理连接
async fn handle_connection(connection: Connection, receive_dir: Arc<PathBuf>, stats: Arc<ReceiverStats>) {
    let remote_node_id = connection.remote_id().to_z32();
    info!("✅ 连接建立成功");
    info!("👤 远程节点: {}", remote_node_id);
    
    // 更新统计信息
    stats.increment_connection().await;
    stats.add_node(remote_node_id.clone()).await;
    
    // 持续接收数据
    loop {
        match receive_data(&connection, &receive_dir, &stats).await {
            Ok(response) => {
                if let Err(e) = send_response(&connection, &response).await {
                    error!("❌ 发送响应失败: {}", e);
                    break;
                }
            }
            Err(e) => {
                warn!("❌ 接收数据失败: {}", e);
                break;
            }
        }
    }
    
    info!("🔚 连接结束: {}", remote_node_id);
}

/// 接收数据（消息或文件）
async fn receive_data(connection: &Connection, receive_dir: &PathBuf, stats: &Arc<ReceiverStats>) -> Result<String> {
    let mut recv_stream = connection.accept_uni().await?;
    use tokio::io::AsyncReadExt;
    let data = recv_stream.read_to_end(100 * 1024 * 1024).await?; // 最大100MB
    
    // 尝试解析为TransferData
    match serde_json::from_slice::<TransferData>(&data) {
        Ok(transfer_data) => {
            match transfer_data {
                TransferData::Message { content } => {
                    info!("📨 收到消息: {}", content);
                    stats.increment_message().await;
                    Ok(format!("✅ 消息已接收: {} bytes", content.len()))
                }
                TransferData::File { filename, size, content } => {
                    info!("📁 收到文件: {} ({} bytes)", filename, size);
                    
                    // 保存文件
                    let file_path = receive_dir.join(&filename);
                    let mut file = fs::File::create(&file_path).await?;
                    file.write_all(&content).await?;
                    file.flush().await?;
                    drop(file);
                    
                    info!("💾 文件已保存: {}", file_path.display());
                    stats.increment_file().await;
                    
                    Ok(format!("✅ 文件已接收并保存: {} ({} bytes)", filename, size))
                }
            }
        }
        Err(_) => {
            // 如果不是JSON格式，当作普通文本消息处理
            let message = String::from_utf8_lossy(&data);
            info!("📨 收到原始消息: {}", message);
            stats.increment_message().await;
            Ok(format!("✅ 消息已接收: {} bytes", data.len()))
        }
    }
}

/// 发送响应
async fn send_response(connection: &Connection, response: &str) -> Result<()> {
    let mut send_stream = connection.open_uni().await?;
    use tokio::io::AsyncWriteExt;
    send_stream.write_all(response.as_bytes()).await?;
    let _ = send_stream.finish();
    Ok(())
}

/// 显示统计信息
async fn display_stats(stats: &ReceiverStats, node_id: &str) {
    let stats_display = stats.get_stats().await;
    
    info!("📊 ===== 节点统计信息 =====");
    info!("🔑 节点ID: {}", node_id);
    info!("📨 总消息数: {}", stats_display.message_count);
    info!("📁 总文件数: {}", stats_display.file_count);
    info!("🔗 总连接数: {}", stats_display.connection_count);
    
    if let Some(last_time) = stats_display.last_message {
        if let Ok(duration) = last_time.duration_since(UNIX_EPOCH) {
            info!("⏰ 最后消息时间: {}", chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_default());
        }
    }
    
    info!("👥 已连接节点数: {}", stats_display.connected_nodes.len());
    
    for (node_id, node_info) in &stats_display.connected_nodes {
        if let Ok(first_duration) = node_info.first_seen.duration_since(UNIX_EPOCH) {
            if let Ok(last_duration) = node_info.last_seen.duration_since(UNIX_EPOCH) {
                info!("  📱 {}: 消息数={}, 首次={}, 最后={}", 
                    node_id, 
                    node_info.message_count,
                    chrono::DateTime::from_timestamp(first_duration.as_secs() as i64, 0).unwrap_or_default().format("%H:%M:%S"),
                    chrono::DateTime::from_timestamp(last_duration.as_secs() as i64, 0).unwrap_or_default().format("%H:%M:%S"));
            }
        }
    }
    
    info!("========================================");
}

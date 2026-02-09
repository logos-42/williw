/**
 * 常驻iroh文件接收端节点
 * 持续运行，接收来自任何发送端的文件和消息
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
use std::path::{Path, PathBuf};
use tokio::fs;
use serde::{Deserialize, Serialize};

/// 常驻iroh文件接收端
#[derive(Parser)]
#[command(name = "iroh-persistent-file-receiver")]
#[command(about = "常驻iroh文件接收端节点")]
pub struct Args {
    /// 绑定端口
    #[arg(long, default_value = "9234")]
    port: u16,
    
    /// 节点名称
    #[arg(long, default_value = "persistent-file-receiver")]
    name: String,
    
    /// 日志级别
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// 文件保存目录
    #[arg(long, default_value = "./received_files")]
    output_dir: String,
    
    /// 最大文件大小 (MB)
    #[arg(long, default_value = "100")]
    max_file_size: usize,
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
    
    info!("🚀 启动常驻iroh文件接收端节点");
    info!("📛 节点名称: {}", args.name);
    info!("🔗 绑定端口: {}", args.port);
    info!("📁 文件保存目录: {}", args.output_dir);
    info!("📏 最大文件大小: {} MB", args.max_file_size);
    
    // 创建输出目录
    let output_path = PathBuf::from(&args.output_dir);
    if !output_path.exists() {
        fs::create_dir_all(&output_path).await?;
        info!("📁 创建文件保存目录: {}", args.output_dir);
    }
    
    // 创建统计信息
    let stats = Arc::new(ReceiverStats::new());
    
    // 启动接收端
    start_persistent_file_receiver(args.port, args.name, args.output_dir, args.max_file_size, stats).await
}

/// 接收端统计信息
struct ReceiverStats {
    message_count: AtomicU64,
    file_count: AtomicU64,
    connection_count: AtomicU64,
    last_message_time: RwLock<Option<SystemTime>>,
    last_file_time: RwLock<Option<SystemTime>>,
    connected_nodes: RwLock<HashMap<String, NodeInfo>>,
    received_files: RwLock<Vec<FileInfo>>,
}

impl ReceiverStats {
    fn new() -> Self {
        Self {
            message_count: AtomicU64::new(0),
            file_count: AtomicU64::new(0),
            connection_count: AtomicU64::new(0),
            last_message_time: RwLock::new(None),
            last_file_time: RwLock::new(None),
            connected_nodes: RwLock::new(HashMap::new()),
            received_files: RwLock::new(Vec::new()),
        }
    }
    
    async fn increment_message(&self) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
        *self.last_message_time.write().await = Some(SystemTime::now());
    }
    
    async fn increment_file(&self) {
        self.file_count.fetch_add(1, Ordering::Relaxed);
        *self.last_file_time.write().await = Some(SystemTime::now());
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
    
    async fn add_file(&self, file_info: FileInfo) {
        let mut files = self.received_files.write().await;
        files.push(file_info);
        
        // 保持最近的100个文件记录
        if files.len() > 100 {
            files.remove(0);
        }
    }
    
    async fn get_stats(&self) -> StatsDisplay {
        let message_count = self.message_count.load(Ordering::Relaxed);
        let file_count = self.file_count.load(Ordering::Relaxed);
        let connection_count = self.connection_count.load(Ordering::Relaxed);
        let last_message = *self.last_message_time.read().await;
        let last_file = *self.last_file_time.read().await;
        let connected_nodes = self.connected_nodes.read().await.clone();
        let received_files = self.received_files.read().await.clone();
        
        StatsDisplay {
            message_count,
            file_count,
            connection_count,
            last_message,
            last_file,
            connected_nodes,
            received_files,
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

/// 文件信息
#[derive(Clone, Serialize, Deserialize)]
struct FileInfo {
    filename: String,
    size: usize,
    sender: String,
    received_at: SystemTime,
    file_path: String,
}

/// 统计显示
struct StatsDisplay {
    message_count: u64,
    file_count: u64,
    connection_count: u64,
    last_message: Option<SystemTime>,
    last_file: Option<SystemTime>,
    connected_nodes: HashMap<String, NodeInfo>,
    received_files: Vec<FileInfo>,
}

/// 文件传输消息
#[derive(Serialize, Deserialize)]
enum TransferMessage {
    Text(String),
    File {
        filename: String,
        size: usize,
        data: Vec<u8>,
    },
    FileInfo {
        filename: String,
        size: usize,
    },
}

/// 启动常驻文件接收端
async fn start_persistent_file_receiver(
    port: u16, 
    name: String, 
    output_dir: String,
    max_file_size: usize,
    stats: Arc<ReceiverStats>
) -> Result<()> {
    // 创建端点
    let endpoint = Endpoint::builder()
        .bind_addr_v4(format!("127.0.0.1:{}", port).parse::<std::net::SocketAddrV4>()?)
        .alpns(vec![b"file-transfer".to_vec()])
        .bind()
        .await?;
    
    let node_id = endpoint.id().to_z32();
    
    info!("🎉 ===== 常驻iroh文件接收端启动成功 =====");
    info!("📛 节点名称: {}", name);
    info!("🔑 节点ID: {}", node_id);
    info!("📍 监听端口: {}", port);
    info!("📁 文件保存目录: {}", output_dir);
    info!("📏 最大文件大小: {} MB", max_file_size);
    info!("📋 发送消息命令:");
    info!("   cargo run --example iroh_file_sender -- send --target {} --port {} --message \"Hello\"", node_id, port);
    info!("📋 发送文件命令:");
    info!("   cargo run --example iroh_file_sender -- send-file --target {} --port {} --file \"/path/to/file\"", node_id, port);
    info!("⏹️  按 Ctrl+C 停止");
    info!("========================================");
    
    // 启动统计信息显示任务
    let stats_clone = stats.clone();
    let node_id_clone = node_id.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            display_file_stats(&stats_clone, &node_id_clone).await;
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
                        let output_dir_clone = output_dir.clone();
                        let max_file_size_clone = max_file_size;
                        tokio::spawn(async move {
                            match accepting.await {
                                Ok(connection) => {
                                    handle_file_connection(connection, stats_clone, output_dir_clone, max_file_size_clone).await;
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

/// 处理文件连接
async fn handle_file_connection(
    connection: Connection, 
    stats: Arc<ReceiverStats>,
    output_dir: String,
    max_file_size: usize
) {
    let remote_node_id = connection.remote_id().to_z32();
    info!("✅ 连接建立成功");
    info!("👤 远程节点: {}", remote_node_id);
    
    // 更新统计信息
    stats.increment_connection().await;
    stats.add_node(remote_node_id.clone()).await;
    
    // 持续接收消息和文件
    loop {
        match receive_transfer_message(&connection).await {
            Ok(message) => {
                match message {
                    TransferMessage::Text(text) => {
                        info!("📨 收到文本消息: {}", text);
                        stats.increment_message().await;
                        stats.add_node(remote_node_id.clone()).await;
                        
                        // 发送确认
                        let response = format!("Text message received by {}: {}", 
                            "persistent-file-receiver", 
                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
                        
                        if let Err(e) = send_response(&connection, &response).await {
                            error!("❌ 发送响应失败: {}", e);
                        }
                    }
                    TransferMessage::File { filename, size, data } => {
                        info!("📁 收到文件: {} ({} bytes)", filename, size);
                        
                        // 检查文件大小
                        if size > max_file_size * 1024 * 1024 {
                            error!("❌ 文件太大: {} bytes (最大: {} MB)", size, max_file_size);
                            continue;
                        }
                        
                        // 保存文件
                        match save_received_file(&filename, &data, &output_dir, &remote_node_id).await {
                            Ok(file_path) => {
                                info!("✅ 文件保存成功: {}", file_path);
                                
                                // 更新统计信息
                                stats.increment_file().await;
                                stats.add_node(remote_node_id.clone()).await;
                                
                                let file_info = FileInfo {
                                    filename: filename.clone(),
                                    size,
                                    sender: remote_node_id.clone(),
                                    received_at: SystemTime::now(),
                                    file_path: file_path.clone(),
                                };
                                stats.add_file(file_info).await;
                                
                                // 发送确认
                                let response = format!("File received and saved to: {} by {}", 
                                    file_path, 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
                                
                                if let Err(e) = send_response(&connection, &response).await {
                                    error!("❌ 发送响应失败: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("❌ 文件保存失败: {}", e);
                            }
                        }
                    }
                    TransferMessage::FileInfo { filename, size } => {
                        info!("📄 收到文件信息: {} ({} bytes)", filename, size);
                        // 这里可以实现文件分块传输的逻辑
                    }
                }
            }
            Err(e) => {
                warn!("❌ 接收消息失败: {}", e);
                break;
            }
        }
    }
    
    info!("🔚 连接结束: {}", remote_node_id);
}

/// 接收传输消息
async fn receive_transfer_message(connection: &Connection) -> Result<TransferMessage> {
    let mut recv_stream = connection.accept_uni().await?;
    use tokio::io::AsyncReadExt;
    let data = recv_stream.read_to_end(1024 * 1024 * 1024).await?; // 最大1GB
    
    // 尝试反序列化
    match serde_json::from_slice::<TransferMessage>(&data) {
        Ok(message) => Ok(message),
        Err(_) => {
            // 如果不是JSON格式，当作纯文本处理
            let text = String::from_utf8(data)?;
            Ok(TransferMessage::Text(text))
        }
    }
}

/// 保存接收到的文件
async fn save_received_file(
    filename: &str, 
    data: &[u8], 
    output_dir: &str,
    sender: &str
) -> Result<String> {
    // 创建发送者目录
    let sender_dir = Path::new(output_dir).join(sender);
    if !sender_dir.exists() {
        fs::create_dir_all(&sender_dir).await?;
    }
    
    // 生成唯一文件名
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let file_stem = Path::new(filename).file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = Path::new(filename).extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    let unique_filename = if extension.is_empty() {
        format!("{}_{}", file_stem, timestamp)
    } else {
        format!("{}_{}.{}", file_stem, timestamp, extension)
    };
    
    let file_path = sender_dir.join(&unique_filename);
    
    // 写入文件
    fs::write(&file_path, data).await?;
    
    Ok(file_path.to_string_lossy().to_string())
}

/// 发送响应
async fn send_response(connection: &Connection, response: &str) -> Result<()> {
    let mut send_stream = connection.open_uni().await?;
    use tokio::io::AsyncWriteExt;
    send_stream.write_all(response.as_bytes()).await?;
    let _ = send_stream.finish();
    Ok(())
}

/// 显示文件统计信息
async fn display_file_stats(stats: &ReceiverStats, node_id: &str) {
    let stats_display = stats.get_stats().await;
    
    info!("📊 ===== 文件接收端统计信息 =====");
    info!("🔑 节点ID: {}", node_id);
    info!("📨 总消息数: {}", stats_display.message_count);
    info!("📁 总文件数: {}", stats_display.file_count);
    info!("🔗 总连接数: {}", stats_display.connection_count);
    
    if let Some(last_time) = stats_display.last_message {
        if let Ok(duration) = last_time.duration_since(UNIX_EPOCH) {
            info!("⏰ 最后消息时间: {}", chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_default());
        }
    }
    
    if let Some(last_time) = stats_display.last_file {
        if let Ok(duration) = last_time.duration_since(UNIX_EPOCH) {
            info!("📁 最后文件时间: {}", chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_default());
        }
    }
    
    info!("👥 已连接节点数: {}", stats_display.connected_nodes.len());
    
    // 显示最近接收的文件
    if !stats_display.received_files.is_empty() {
        info!("📁 最近接收的文件:");
        for (i, file) in stats_display.received_files.iter().rev().take(5).enumerate() {
            if let Ok(received_at) = file.received_at.duration_since(UNIX_EPOCH) {
                info!("  {}. {} ({} bytes) 来自 {} 时间: {}", 
                    i + 1, 
                    file.filename, 
                    file.size,
                    file.sender.chars().take(8).collect::<String>(),
                    chrono::DateTime::from_timestamp(received_at.as_secs() as i64, 0).unwrap_or_default().format("%H:%M:%S"));
            }
        }
    }
    
    info!("========================================");
}

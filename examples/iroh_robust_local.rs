/**
 * 健壮的iroh本地P2P演示
 * 包含详细的错误处理和调试信息
 */

use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh::{Endpoint, endpoint::Connection, EndpointAddr, PublicKey};
use iroh::endpoint_info::EndpointIdExt;
use std::net::{SocketAddr, Ipv4Addr};
use std::time::Duration;
use tracing::{info, error, warn, debug};
use tracing_subscriber;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

/// 健壮P2P演示
#[derive(Parser)]
#[command(name = "iroh-robust-local")]
#[command(about = "健壮的iroh本地P2P演示")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
    
    /// 启用调试模式
    #[arg(long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 启动接收端
    Receive {
        /// 绑定端口
        #[arg(long, default_value = "11206")]
        port: u16,
    },
    /// 发送消息
    Send {
        /// 目标节点ID
        #[arg(long)]
        target: String,
        
        /// 目标端口
        #[arg(long, default_value = "11206")]
        port: u16,
        
        /// 消息内容
        #[arg(long, default_value = "Hello from robust iroh!")]
        message: String,
        
        /// 重试次数
        #[arg(long, default_value = "5")]
        retries: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // 配置日志级别
    let level = if args.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
    
    match args.command {
        Commands::Receive { port } => {
            start_receiver(port).await
        }
        Commands::Send { target, port, message, retries } => {
            send_message(target, port, message, retries).await
        }
    }
}

/// 启动接收端
async fn start_receiver(port: u16) -> Result<()> {
    info!("🚀 启动健壮iroh接收端");
    debug!("绑定端口: {}", port);
    
    // 创建端点配置
    let mut builder = Endpoint::builder();
    builder = builder
        .bind_addr_v4(std::net::SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
        .alpns(vec![b"robust".to_vec()]);
    
    // 尝试启用本地网络发现
    debug!("启用本地网络发现");
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    builder = builder.discovery(mdns);
    
    let endpoint = builder.bind().await?;
    
    let node_id = endpoint.id().to_z32();
    // 获取本地端点信息
    info!("📍 端点已创建");
    
    println!("🎉 ===== 健壮接收端启动成功 =====");
    println!("🔑 节点ID: {}", node_id);
    println!("📍 监听端口: {}", port);
    println!("📋 发送命令:");
    println!("   cargo run --example iroh_robust_local -- send --target {} --port {}", node_id, port);
    println!("⏹️  按 Ctrl+C 停止");
    println!("==================================");
    
    // 连接处理循环
    let mut connection_count = 0;
    while let Some(incoming) = endpoint.accept().await {
        connection_count += 1;
        info!("🔗 收到第{}个连接请求", connection_count);
        
        debug!("收到连接请求");
        
        match incoming.accept() {
            Ok(accepting) => {
                info!("📋 接受连接中...");
                
                match timeout(Duration::from_secs(30), accepting).await {
                    Ok(Ok(connection)) => {
                        info!("✅ 连接建立成功");
                        debug!("连接详情: 远程={}", connection.remote_id().to_z32());
                        
                        // 处理连接
                        match handle_connection(connection).await {
                            Ok(_) => {
                                println!("🎉 连接处理成功！");
                            }
                            Err(e) => {
                                error!("❌ 连接处理失败: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("❌ 连接建立失败: {}", e);
                    }
                    Err(_) => {
                        error!("❌ 连接建立超时");
                    }
                }
            }
            Err(e) => {
                error!("❌ 接受连接失败: {}", e);
            }
        }
    }
    
    Ok(())
}

/// 处理连接
async fn handle_connection(connection: Connection) -> Result<()> {
    info!("🔄 开始处理连接");
    
    // 接收消息
    match timeout(Duration::from_secs(30), receive_message(&connection)).await {
        Ok(Ok(message)) => {
            println!("📨 收到消息: {}", message);
            
            // 发送确认回复
            let reply = format!("消息已收到: {}", message);
            match send_reply(&connection, &reply).await {
                Ok(_) => {
                    info!("📤 确认回复已发送");
                }
                Err(e) => {
                    warn!("⚠️ 发送回复失败: {}", e);
                }
            }
        }
        Ok(Err(e)) => {
            error!("❌ 接收消息失败: {}", e);
            return Err(e);
        }
        Err(_) => {
            error!("❌ 接收消息超时");
            return Err(anyhow::anyhow!("接收消息超时"));
        }
    }
    
    Ok(())
}

/// 接收消息
async fn receive_message(connection: &Connection) -> Result<String> {
    debug!("📥 等待接收消息流...");
    
    let mut recv_stream = connection.accept_uni().await?;
    debug!("📡 收到数据流");
    
    let data = recv_stream.read_to_end(1024 * 1024).await?;
    
    let message = String::from_utf8(data)?;
    debug!("📋 消息长度: {} 字节", message.len());
    
    Ok(message)
}

/// 发送回复
async fn send_reply(connection: &Connection, reply: &str) -> Result<()> {
    debug!("📤 发送回复: {}", reply);
    
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(reply.as_bytes()).await?;
    send_stream.finish()?;
    
    Ok(())
}

/// 发送消息
async fn send_message(target_node: String, target_port: u16, message: String, max_retries: u32) -> Result<()> {
    info!("🚀 启动健壮iroh发送端");
    println!("🎯 目标节点: {}", target_node);
    println!("📍 目标端口: {}", target_port);
    println!("📨 消息: {}", message);
    println!("🔄 最大重试次数: {}", max_retries);
    
    // 创建发送端点
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    let endpoint = Endpoint::builder()
        .bind_addr_v4(std::net::SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .alpns(vec![b"robust".to_vec()])
        .discovery(mdns)
        .bind()
        .await?;
    
    let sender_id = endpoint.id().to_z32();
    debug!("🔑 发送方节点ID: {}", sender_id);
    
    // 解析目标节点
    let public_key = PublicKey::from_z32(&target_node)
        .map_err(|e| anyhow::anyhow!("无效节点ID: {}", e))?;
    
    // 创建端点地址
    let target_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), target_port);
    let endpoint_addr = EndpointAddr::from(public_key)
        .with_ip_addr(target_addr);
    
    debug!("🔗 端点地址配置完成");
    
    // 重试连接
    let mut last_error = None;
    for attempt in 1..=max_retries {
        info!("🔗 连接尝试 {}/{}", attempt, max_retries);
        
        // 每次尝试前等待一段时间
        if attempt > 1 {
            let wait_time = std::cmp::min(attempt * 2, 10);
            info!("⏳ 等待{}秒后重试...", wait_time);
            tokio::time::sleep(Duration::from_secs(wait_time as u64)).await;
        }
        
        match timeout(
            Duration::from_secs(20),
            endpoint.connect(endpoint_addr.clone(), b"robust")
        ).await {
            Ok(Ok(connection)) => {
                println!("✅ 连接成功！");
                debug!("连接详情: 远程={}", connection.remote_id().to_z32());
                
                // 发送消息
                match send_and_receive(&connection, &message).await {
                    Ok(reply) => {
                        println!("🎉 消息发送成功！");
                        println!("📨 收到回复: {}", reply);
                        return Ok(());
                    }
                    Err(e) => {
                        error!("❌ 消息发送失败: {}", e);
                        last_error = Some(e);
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("❌ 连接失败: {}", e);
                last_error = Some(e.into());
            }
            Err(_) => {
                warn!("❌ 连接超时");
                last_error = Some(anyhow::anyhow!("连接超时"));
            }
        }
    }
    
    // 所有重试都失败了
    match last_error {
        Some(e) => Err(anyhow::anyhow!("所有连接尝试都失败了，最后错误: {}", e)),
        None => Err(anyhow::anyhow!("连接失败，原因未知")),
    }
}

/// 发送消息并接收回复
async fn send_and_receive(connection: &Connection, message: &str) -> Result<String> {
    info!("📤 发送消息...");
    
    // 发送消息
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(message.as_bytes()).await?;
    send_stream.finish()?;
    
    debug!("✅ 消息发送完成");
    
    // 接收回复
    info!("👂 等待回复...");
    match timeout(Duration::from_secs(15), receive_message(connection)).await {
        Ok(Ok(reply)) => {
            debug!("📨 回复接收完成");
            Ok(reply)
        }
        Ok(Err(e)) => {
            error!("❌ 接收回复失败: {}", e);
            Err(e)
        }
        Err(_) => {
            error!("❌ 等待回复超时");
            Err(anyhow::anyhow!("等待回复超时"))
        }
    }
}
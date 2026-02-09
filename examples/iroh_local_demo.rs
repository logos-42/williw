/**
 * iroh本地P2P演示 - 专门解决本地连接问题
 * 使用本地发现和直接连接
 */

use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh::{Endpoint, endpoint::Connection, EndpointAddr, PublicKey};
use iroh::endpoint_info::EndpointIdExt;
use std::net::{SocketAddr, Ipv4Addr};
use tracing::{info, error, warn};
use tracing_subscriber;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 本地P2P演示
#[derive(Parser)]
#[command(name = "iroh-local-demo")]
#[command(about = "iroh本地P2P演示")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 启动接收端
    Receive {
        /// 绑定端口
        #[arg(long, default_value = "11204")]
        port: u16,
    },
    /// 发送消息
    Send {
        /// 目标节点ID
        #[arg(long)]
        target: String,
        
        /// 目标地址
        #[arg(long, default_value = "127.0.0.1:11204")]
        addr: String,
        
        /// 消息内容
        #[arg(long)]
        message: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // 设置日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let args = Args::parse();
    
    match args.command {
        Commands::Receive { port } => {
            start_receiver(port).await
        }
        Commands::Send { target, addr, message } => {
            send_message(target, addr, message).await
        }
    }
}

/// 启动接收端
async fn start_receiver(port: u16) -> Result<()> {
    info!("🚀 启动iroh本地接收端");
    
    // 创建绑定地址
    let bind_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    info!("🔗 绑定地址: {}", bind_addr);
    
    // 创建端点配置
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    let endpoint = Endpoint::builder()
        .bind_addr_v4(std::net::SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
        .alpns(vec![b"iroh-local".to_vec()])
        .discovery(mdns)  // 启用本地网络发现
        .bind()
        .await?;
    
    let node_id = endpoint.id().to_z32();
    // 获取本地端点信息
    info!("📍 端点已创建");
    
    info!("🎉 ===== iroh接收端启动成功 =====");
    info!("🔑 节点ID: {}", node_id);
    info!("📍 监听端口: {}", port);
    info!("📋 发送消息命令:");
    info!("   cargo run --example iroh_local_demo -- send --target {} --addr {} --message \"Hello World\"", 
          node_id, bind_addr);
    info!("⏹️  按 Ctrl+C 停止");
    info!("=====================================");
    
    // 监听连接
    loop {
        info!("👂 等待连接...");
        
        match endpoint.accept().await {
            Some(incoming) => {
                info!("🔗 收到连接请求");
                
                // 获取连接信息
                info!("👤 收到连接请求");
                
                match incoming.accept() {
                    Ok(accepting) => {
                        match accepting.await {
                            Ok(connection) => {
                                info!("✅ 连接建立成功");
                                
                                // 处理连接
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(connection).await {
                                        error!("❌ 处理连接失败: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("❌ 连接建立失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ 接受连接失败: {}", e);
                    }
                }
            }
            None => {
                info!("⏹️ 端点关闭");
                break;
            }
        }
    }
    
    Ok(())
}

/// 处理连接
async fn handle_connection(connection: Connection) -> Result<()> {
    info!("🔄 开始处理连接");
    
    // 接收消息
    match receive_message(&connection).await {
        Ok(msg) => {
            info!("📨 收到消息: {}", msg);
            
            // 发送回复
            match send_reply(&connection, "消息已收到！").await {
                Ok(_) => {
                    info!("📤 回复发送成功");
                }
                Err(e) => {
                    warn!("⚠️ 发送回复失败: {}", e);
                }
            }
            
            info!("🎉 iroh本地P2P传输成功！");
        }
        Err(e) => {
            error!("❌ 接收消息失败: {}", e);
        }
    }
    
    Ok(())
}

/// 接收消息
async fn receive_message(connection: &Connection) -> Result<String> {
    info!("📥 等待接收消息...");
    
    // 接收单向流
    let mut recv_stream = connection.accept_uni().await?;
    info!("📡 收到数据流");
    
    // 读取数据
    let data = recv_stream.read_to_end(1024 * 1024).await?;
    
    // 转换为字符串
    let message = String::from_utf8(data)?;
    info!("📋 消息内容: {} 字节", message.len());
    
    Ok(message)
}

/// 发送回复
async fn send_reply(connection: &Connection, reply: &str) -> Result<()> {
    info!("📤 发送回复...");
    
    // 打开单向流
    let mut send_stream = connection.open_uni().await?;
    
    // 发送数据
    send_stream.write_all(reply.as_bytes()).await?;
    send_stream.finish()?;
    
    info!("✅ 回复发送完成");
    Ok(())
}

/// 发送消息
async fn send_message(target_node: String, target_addr: String, message: String) -> Result<()> {
    info!("🚀 启动iroh本地发送端");
    info!("🎯 目标节点: {}", target_node);
    info!("📍 目标地址: {}", target_addr);
    info!("📨 消息: {}", message);
    
    // 解析目标地址
    let addr: SocketAddr = target_addr.parse()
        .map_err(|e| anyhow::anyhow!("无效的目标地址: {}", e))?;
    
    // 创建发送端点
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    let endpoint = Endpoint::builder()
        .bind_addr_v4(std::net::SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .alpns(vec![b"iroh-local".to_vec()])
        .discovery(mdns)  // 启用本地网络发现
        .bind()
        .await?;
    
    let sender_id = endpoint.id().to_z32();
    info!("🔑 发送方节点ID: {}", sender_id);
    
    // 解析目标节点ID
    let public_key = PublicKey::from_z32(&target_node)
        .map_err(|e| anyhow::anyhow!("无效的目标节点ID: {}", e))?;
    
    // 创建端点地址，包含直接地址信息
    let mut endpoint_addr = EndpointAddr::from(public_key);
    endpoint_addr = endpoint_addr.with_ip_addr(addr);
    
    info!("🔗 尝试连接到目标节点...");
    info!("💡 使用直接地址连接: {}", addr);
    
    // 等待端点初始化
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 尝试连接
    let connection = match tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        endpoint.connect(endpoint_addr, b"iroh-local")
    ).await {
        Ok(Ok(conn)) => {
            info!("✅ 连接成功！");
            conn
        }
        Ok(Err(e)) => {
            error!("❌ 连接失败: {}", e);
            return Err(anyhow::anyhow!("连接失败: {}", e));
        }
        Err(_) => {
            error!("❌ 连接超时");
            return Err(anyhow::anyhow!("连接超时"));
        }
    };
    
    info!("📍 连接详情:");
    info!("  - 远程节点: {}", connection.remote_id().to_z32());
    info!("  - 本地地址: {:?}", connection.local_ip());
    info!("  - 远程地址: {:?}", connection.remote_address());
    
    // 发送消息
    info!("📤 开始发送消息...");
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(message.as_bytes()).await?;
    send_stream.finish()?;
    
    info!("✅ 消息发送成功！");
    
    // 等待回复
    info!("👂 等待回复...");
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        receive_message(&connection)
    ).await {
        Ok(Ok(reply)) => {
            info!("📨 收到回复: {}", reply);
        }
        Ok(Err(e)) => {
            warn!("⚠️ 接收回复失败: {}", e);
        }
        Err(_) => {
            warn!("⚠️ 等待回复超时");
        }
    }
    
    info!("🎉 iroh本地P2P传输完成！");
    
    Ok(())
}
/**
 * iroh跨网络P2P演示
 * 支持不同电脑之间的P2P通信
 */

use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh::{Endpoint, endpoint::Connection, EndpointAddr, PublicKey};
use iroh::endpoint_info::EndpointIdExt;
use std::net::{SocketAddr, Ipv4Addr, IpAddr};
use tracing::{info, error, warn};
use tracing_subscriber;
// Removed unused imports AsyncReadExt and AsyncWriteExt

/// 跨网络P2P演示
#[derive(Parser)]
#[command(name = "iroh-network-demo")]
#[command(about = "iroh跨网络P2P演示")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 启动接收端
    Receive {
        /// 绑定端口
        #[arg(long, default_value = "11207")]
        port: u16,
        
        /// 绑定IP地址 (0.0.0.0 表示所有接口)
        #[arg(long, default_value = "0.0.0.0")]
        bind_ip: String,
    },
    /// 发送消息
    Send {
        /// 目标节点ID
        #[arg(long)]
        target: String,
        
        /// 目标IP地址
        #[arg(long)]
        target_ip: String,
        
        /// 目标端口
        #[arg(long, default_value = "11207")]
        target_port: u16,
        
        /// 消息内容
        #[arg(long, default_value = "Hello from remote computer!")]
        message: String,
    },
    /// 显示本机网络信息
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
    
    let args = Args::parse();
    
    match args.command {
        Commands::Receive { port, bind_ip } => {
            start_receiver(port, bind_ip).await
        }
        Commands::Send { target, target_ip, target_port, message } => {
            send_message(target, target_ip, target_port, message).await
        }
        Commands::Info => {
            show_network_info().await
        }
    }
}

/// 显示网络信息
async fn show_network_info() -> Result<()> {
    println!("🌐 本机网络信息");
    println!("================");
    
    // 获取本机IP地址
    match get_local_ip().await {
        Ok(ip) => {
            println!("📍 本机IP地址: {}", ip);
            println!("🔧 建议配置:");
            println!("   接收端: cargo run --example iroh_network_demo -- receive --bind-ip 0.0.0.0 --port 11207");
            println!("   发送端: cargo run --example iroh_network_demo -- send --target <节点ID> --target-ip {} --target-port 11207", ip);
        }
        Err(e) => {
            println!("❌ 无法获取本机IP: {}", e);
        }
    }
    
    println!("");
    println!("🔥 防火墙配置提醒:");
    println!("   - Windows: 允许端口11207通过Windows防火墙");
    println!("   - 路由器: 如需跨网段通信，请配置端口转发");
    println!("   - 企业网络: 请联系网络管理员开放端口");
    
    Ok(())
}

/// 获取本机IP地址
async fn get_local_ip() -> Result<IpAddr> {
    use std::net::UdpSocket;
    
    // 通过连接到外部地址来获取本机IP
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    let local_addr = socket.local_addr()?;
    Ok(local_addr.ip())
}

/// 启动接收端
async fn start_receiver(port: u16, bind_ip: String) -> Result<()> {
    info!("🚀 启动iroh跨网络接收端");
    
    // 解析绑定IP
    let bind_addr: IpAddr = bind_ip.parse()
        .map_err(|e| anyhow::anyhow!("无效的绑定IP地址: {}", e))?;
    
    println!("🔗 绑定地址: {}:{}", bind_addr, port);
    
    // 创建端点配置 - 绑定到指定IP和端口
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    let endpoint = Endpoint::builder()
        .bind_addr_v4(std::net::SocketAddrV4::new(
            match bind_addr {
                IpAddr::V4(ipv4) => ipv4,
                IpAddr::V6(_) => return Err(anyhow::anyhow!("暂不支持IPv6")),
            },
            port
        ))
        .alpns(vec![b"iroh-network".to_vec()])
        .discovery(mdns)
        .bind()
        .await?;
    
    let node_id = endpoint.id().to_z32();
    
    // 显示本机IP信息
    match get_local_ip().await {
        Ok(local_ip) => {
            println!("🎉 ===== iroh跨网络接收端启动成功 =====");
            println!("🔑 节点ID: {}", node_id);
            println!("📍 本机IP: {}", local_ip);
            println!("📍 监听端口: {}", port);
            println!("🌐 绑定接口: {}", bind_addr);
            println!("");
            println!("📋 远程发送命令:");
            println!("   cargo run --example iroh_network_demo -- send \\");
            println!("     --target {} \\", node_id);
            println!("     --target-ip {} \\", local_ip);
            println!("     --target-port {} \\", port);
            println!("     --message \"Hello from remote!\"");
            println!("");
            println!("🔥 请确保防火墙允许端口{}通过", port);
            println!("⏹️  按 Ctrl+C 停止");
            println!("==========================================");
        }
        Err(e) => {
            warn!("无法获取本机IP: {}", e);
            println!("🎉 ===== iroh跨网络接收端启动成功 =====");
            println!("🔑 节点ID: {}", node_id);
            println!("📍 监听端口: {}", port);
            println!("🌐 绑定接口: {}", bind_addr);
            println!("⏹️  按 Ctrl+C 停止");
            println!("==========================================");
        }
    }
    
    // 监听连接
    let mut connection_count = 0;
    while let Some(incoming) = endpoint.accept().await {
        connection_count += 1;
        info!("🔗 收到第{}个连接请求", connection_count);
        
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
    
    Ok(())
}

/// 处理连接
async fn handle_connection(connection: Connection) -> Result<()> {
    info!("🔄 开始处理跨网络连接");
    
    // 获取连接信息 - 使用可用的API方法
    let remote_id = connection.remote_id().to_z32();
    info!("📍 远程节点ID: {}", remote_id);
    
    // 接收消息
    match receive_message(&connection).await {
        Ok(msg) => {
            println!("📨 收到跨网络消息: {}", msg);
            println!("📍 来自节点: {}", remote_id);
            
            // 发送回复
            let reply = format!("跨网络消息已收到: {}", msg);
            match send_reply(&connection, &reply).await {
                Ok(_) => {
                    info!("📤 跨网络回复发送成功");
                }
                Err(e) => {
                    warn!("⚠️ 发送回复失败: {}", e);
                }
            }
            
            println!("🎉 跨网络P2P通信成功！");
        }
        Err(e) => {
            error!("❌ 接收消息失败: {}", e);
        }
    }
    
    Ok(())
}

/// 接收消息
async fn receive_message(connection: &Connection) -> Result<String> {
    let mut recv_stream = connection.accept_uni().await?;
    let data = recv_stream.read_to_end(1024 * 1024).await?;
    let message = String::from_utf8(data)?;
    Ok(message)
}

/// 发送回复
async fn send_reply(connection: &Connection, reply: &str) -> Result<()> {
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(reply.as_bytes()).await?;
    send_stream.finish()?;
    Ok(())
}

/// 发送消息
async fn send_message(target_node: String, target_ip: String, target_port: u16, message: String) -> Result<()> {
    info!("🚀 启动iroh跨网络发送端");
    println!("🎯 目标节点: {}", target_node);
    println!("📍 目标IP: {}", target_ip);
    println!("📍 目标端口: {}", target_port);
    println!("📨 消息: {}", message);
    
    // 解析目标IP
    let target_addr: IpAddr = target_ip.parse()
        .map_err(|e| anyhow::anyhow!("无效的目标IP地址: {}", e))?;
    
    // 创建发送端点 - 绑定到所有接口
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    let endpoint = Endpoint::builder()
        .bind_addr_v4(std::net::SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        .alpns(vec![b"iroh-network".to_vec()])
        .discovery(mdns)
        .bind()
        .await?;
    
    let sender_id = endpoint.id().to_z32();
    info!("🔑 发送方节点ID: {}", sender_id);
    
    // 解析目标节点ID
    let public_key = PublicKey::from_z32(&target_node)
        .map_err(|e| anyhow::anyhow!("无效的目标节点ID: {}", e))?;
    
    // 创建端点地址，指定目标IP和端口
    let socket_addr = SocketAddr::new(target_addr, target_port);
    let endpoint_addr = EndpointAddr::from(public_key)
        .with_ip_addr(socket_addr);
    
    info!("🔗 尝试跨网络连接...");
    println!("🌐 连接目标: {}", socket_addr);
    
    // 尝试连接
    let connection = match tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        endpoint.connect(endpoint_addr, b"iroh-network")
    ).await {
        Ok(Ok(conn)) => {
            println!("✅ 跨网络连接成功！");
            conn
        }
        Ok(Err(e)) => {
            error!("❌ 跨网络连接失败: {}", e);
            println!("🔧 故障排除建议:");
            println!("   1. 检查目标IP地址是否正确");
            println!("   2. 确认目标端口{}已开放", target_port);
            println!("   3. 检查防火墙设置");
            println!("   4. 确认目标机器的接收端正在运行");
            return Err(anyhow::anyhow!("跨网络连接失败: {}", e));
        }
        Err(_) => {
            error!("❌ 跨网络连接超时");
            println!("🔧 连接超时，请检查网络连接和防火墙设置");
            return Err(anyhow::anyhow!("跨网络连接超时"));
        }
    };
    
    println!("📍 连接详情:");
    println!("  - 远程节点: {}", connection.remote_id().to_z32());
    // Note: local_ip() and remote_address() methods are not available in iroh 0.95
    // Connection established successfully, details available through other means
    
    // 发送消息
    info!("📤 发送跨网络消息...");
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(message.as_bytes()).await?;
    send_stream.finish()?;
    
    println!("✅ 跨网络消息发送成功！");
    
    // 等待回复
    info!("👂 等待跨网络回复...");
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(15),
        receive_message(&connection)
    ).await {
        Ok(Ok(reply)) => {
            println!("📨 收到跨网络回复: {}", reply);
        }
        Ok(Err(e)) => {
            warn!("⚠️ 接收回复失败: {}", e);
        }
        Err(_) => {
            warn!("⚠️ 等待回复超时");
        }
    }
    
    println!("🎉 跨网络P2P通信完成！");
    
    Ok(())
}
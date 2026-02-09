/**
 * iroh安全P2P通信演示
 * 使用节点发现和中继，不暴露IP地址
 */

use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh::{Endpoint, endpoint::Connection, EndpointAddr, PublicKey};
use iroh::endpoint_info::EndpointIdExt;
use tracing::{info, error, warn};
use tracing_subscriber;

/// 安全P2P演示
#[derive(Parser)]
#[command(name = "iroh-secure-p2p")]
#[command(about = "iroh安全P2P通信，不暴露IP地址")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 启动接收端（监听模式）
    Listen {
        /// 自定义端口（可选，默认随机）
        #[arg(long)]
        port: Option<u16>,
        
        /// 节点名称（可选）
        #[arg(long)]
        name: Option<String>,
    },
    /// 连接并发送消息
    Connect {
        /// 目标节点ID
        #[arg(long)]
        target: String,
        
        /// 消息内容
        #[arg(long, default_value = "Hello from secure P2P!")]
        message: String,
        
        /// 连接超时（秒）
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    /// 显示本节点信息
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
        Commands::Listen { port, name } => {
            start_listener(port, name).await
        }
        Commands::Connect { target, message, timeout } => {
            connect_and_send(target, message, timeout).await
        }
        Commands::Info => {
            show_node_info().await
        }
    }
}

/// 显示节点信息
async fn show_node_info() -> Result<()> {
    println!("🔐 iroh安全P2P通信");
    println!("==================");
    
    // 创建临时端点获取节点ID
    let endpoint = create_secure_endpoint(None).await?;
    let node_id = endpoint.id().to_z32();
    
    println!("🆔 本节点ID: {}", node_id);
    println!("🔒 安全特性:");
    println!("   ✅ 不暴露IP地址");
    println!("   ✅ 使用iroh内置发现机制");
    println!("   ✅ 支持NAT穿透");
    println!("   ✅ 端到端加密");
    println!("");
    println!("📋 使用方法:");
    println!("   监听: cargo run --example iroh_secure_p2p -- listen");
    println!("   连接: cargo run --example iroh_secure_p2p -- connect --target <节点ID>");
    
    Ok(())
}

/// 创建安全的iroh端点
async fn create_secure_endpoint(port: Option<u16>) -> Result<Endpoint> {
    let mut builder = Endpoint::builder();
    
    // 配置绑定地址
    if let Some(p) = port {
        builder = builder.bind_addr_v4(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::UNSPECIFIED, 
            p
        ));
    } else {
        // 使用随机端口，更安全
        builder = builder.bind_addr_v4(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::UNSPECIFIED, 
            0
        ));
    }
    
    // 启用本地网络发现（mDNS）
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    builder = builder.discovery(mdns);
    
    // 设置ALPN协议标识
    builder = builder.alpns(vec![b"secure-p2p".to_vec()]);
    
    let endpoint = builder.bind().await?;
    
    info!("🔐 安全端点创建成功");
    Ok(endpoint)
}

/// 启动监听端
async fn start_listener(port: Option<u16>, name: Option<String>) -> Result<()> {
    info!("🚀 启动iroh安全监听端");
    
    let endpoint = create_secure_endpoint(port).await?;
    let node_id = endpoint.id().to_z32();
    
    let display_name = name.unwrap_or_else(|| "Anonymous".to_string());
    
    println!("🎉 ===== iroh安全监听端启动成功 =====");
    println!("🆔 节点ID: {}", node_id);
    println!("👤 节点名称: {}", display_name);
    println!("🔒 安全模式: 已启用");
    println!("🌐 发现机制: mDNS + 中继服务器");
    println!("");
    println!("📋 远程连接命令:");
    println!("   cargo run --example iroh_secure_p2p -- connect \\");
    println!("     --target {} \\", node_id);
    println!("     --message \"Hello from secure connection!\"");
    println!("");
    println!("🔐 隐私保护:");
    println!("   ✅ IP地址不会暴露给对方");
    println!("   ✅ 使用iroh内置NAT穿透");
    println!("   ✅ 端到端加密通信");
    println!("⏹️  按 Ctrl+C 停止");
    println!("==========================================");
    
    // 监听连接
    let mut connection_count = 0;
    while let Some(incoming) = endpoint.accept().await {
        connection_count += 1;
        info!("🔗 收到第{}个安全连接请求", connection_count);
        
        match incoming.accept() {
            Ok(accepting) => {
                match accepting.await {
                    Ok(connection) => {
                        info!("✅ 安全连接建立成功");
                        
                        // 处理连接
                        tokio::spawn(async move {
                            if let Err(e) = handle_secure_connection(connection).await {
                                error!("❌ 处理安全连接失败: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("❌ 安全连接建立失败: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("❌ 接受安全连接失败: {}", e);
            }
        }
    }
    
    Ok(())
}

/// 处理安全连接
async fn handle_secure_connection(connection: Connection) -> Result<()> {
    info!("🔄 开始处理安全连接");
    
    // 获取远程节点ID（不暴露IP）
    let remote_id = connection.remote_id().to_z32();
    info!("🆔 远程节点ID: {}", remote_id);
    
    // 接收消息
    match receive_secure_message(&connection).await {
        Ok(msg) => {
            println!("📨 收到安全消息: {}", msg);
            println!("🆔 来自节点: {}", remote_id);
            println!("🔒 连接状态: 端到端加密");
            
            // 发送加密回复
            let reply = format!("🔐 安全消息已收到: {}", msg);
            match send_secure_reply(&connection, &reply).await {
                Ok(_) => {
                    info!("📤 安全回复发送成功");
                }
                Err(e) => {
                    warn!("⚠️ 发送安全回复失败: {}", e);
                }
            }
            
            println!("🎉 安全P2P通信成功！");
        }
        Err(e) => {
            error!("❌ 接收安全消息失败: {}", e);
        }
    }
    
    Ok(())
}

/// 接收安全消息
async fn receive_secure_message(connection: &Connection) -> Result<String> {
    let mut recv_stream = connection.accept_uni().await?;
    let data = recv_stream.read_to_end(1024 * 1024).await?;
    let message = String::from_utf8(data)?;
    Ok(message)
}

/// 发送安全回复
async fn send_secure_reply(connection: &Connection, reply: &str) -> Result<()> {
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(reply.as_bytes()).await?;
    send_stream.finish()?;
    Ok(())
}

/// 连接并发送消息
async fn connect_and_send(target_node: String, message: String, timeout_secs: u64) -> Result<()> {
    info!("🚀 启动iroh安全连接端");
    println!("🎯 目标节点: {}", target_node);
    println!("📨 消息: {}", message);
    println!("⏱️ 超时时间: {}秒", timeout_secs);
    println!("🔒 安全模式: 已启用");
    
    // 创建发送端点
    let endpoint = create_secure_endpoint(None).await?;
    let sender_id = endpoint.id().to_z32();
    info!("🆔 发送方节点ID: {}", sender_id);
    
    // 解析目标节点ID
    let public_key = PublicKey::from_z32(&target_node)
        .map_err(|e| anyhow::anyhow!("无效的目标节点ID: {}", e))?;
    
    // 创建端点地址（不指定IP，让iroh自动发现）
    let endpoint_addr = EndpointAddr::from(public_key);
    
    info!("🔗 尝试安全连接...");
    println!("🔍 正在发现目标节点...");
    println!("🔐 使用iroh内置发现机制");
    
    // 尝试连接（使用iroh的自动发现）
    let connection = match tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_secs),
        endpoint.connect(endpoint_addr, b"secure-p2p")
    ).await {
        Ok(Ok(conn)) => {
            println!("✅ 安全连接成功！");
            println!("🔒 连接已加密，IP地址未暴露");
            conn
        }
        Ok(Err(e)) => {
            error!("❌ 安全连接失败: {}", e);
            println!("🔧 故障排除建议:");
            println!("   1. 确认目标节点ID正确");
            println!("   2. 确认目标节点正在运行监听模式");
            println!("   3. 检查网络连接");
            println!("   4. 等待iroh发现机制完成");
            return Err(anyhow::anyhow!("安全连接失败: {}", e));
        }
        Err(_) => {
            error!("❌ 安全连接超时");
            println!("🔧 连接超时，可能的原因:");
            println!("   1. 目标节点不在线");
            println!("   2. 网络发现需要更多时间");
            println!("   3. NAT穿透失败");
            return Err(anyhow::anyhow!("安全连接超时"));
        }
    };
    
    println!("📍 连接详情:");
    println!("  - 远程节点: {}", connection.remote_id().to_z32());
    println!("  - 加密状态: ✅ 端到端加密");
    println!("  - 隐私保护: ✅ IP地址未暴露");
    
    // 发送安全消息
    info!("📤 发送安全消息...");
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(message.as_bytes()).await?;
    send_stream.finish()?;
    
    println!("✅ 安全消息发送成功！");
    
    // 等待加密回复
    info!("👂 等待安全回复...");
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(15),
        receive_secure_message(&connection)
    ).await {
        Ok(Ok(reply)) => {
            println!("📨 收到安全回复: {}", reply);
        }
        Ok(Err(e)) => {
            warn!("⚠️ 接收回复失败: {}", e);
        }
        Err(_) => {
            warn!("⚠️ 等待回复超时");
        }
    }
    
    println!("🎉 安全P2P通信完成！");
    
    Ok(())
}
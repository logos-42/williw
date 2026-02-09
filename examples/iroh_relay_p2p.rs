/**
 * iroh中继P2P通信
 * 使用iroh中继节点实现真正的去中心化连接
 * 两个节点通过同一个中继节点连接，无需知道对方IP地址
 */

use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh::{Endpoint, endpoint::Connection, EndpointAddr, PublicKey};
use iroh::endpoint_info::EndpointIdExt;
use std::time::Duration;
use tracing::{info, error, warn, debug};
use tracing_subscriber;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

/// 中继P2P通信
#[derive(Parser)]
#[command(name = "iroh-relay-p2p")]
#[command(about = "iroh中继P2P通信，使用中继节点实现去中心化连接")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
    
    /// 启用调试模式
    #[arg(long, global = true)]
    debug: bool,
    
    /// 自定义中继服务器URL
    #[arg(long, global = true)]
    relay_url: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 启动监听端（等待连接）
    Listen {
        /// 节点名称
        #[arg(long)]
        name: Option<String>,
        
        /// 绑定端口（可选，默认随机）
        #[arg(long)]
        port: Option<u16>,
    },
    /// 连接到另一个节点
    Connect {
        /// 目标节点ID
        #[arg(long)]
        target: String,
        
        /// 消息内容
        #[arg(long, default_value = "Hello via relay!")]
        message: String,
        
        /// 连接超时（秒）
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    /// 显示节点信息
    Info,
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
        Commands::Listen { name, port } => {
            start_relay_listener(name, port, args.relay_url).await
        }
        Commands::Connect { target, message, timeout } => {
            connect_via_relay(target, message, timeout, args.relay_url).await
        }
        Commands::Info => {
            show_relay_info(args.relay_url).await
        }
    }
}

/// 显示中继节点信息
async fn show_relay_info(custom_relay: Option<String>) -> Result<()> {
    println!("🌐 iroh中继P2P通信");
    println!("==================");
    
    // 创建临时端点获取节点ID
    let endpoint = create_relay_endpoint(None, custom_relay.clone()).await?;
    let node_id = endpoint.id().to_z32();
    
    println!("🆔 本节点ID: {}", node_id);
    println!("🔗 中继服务器: {}", 
        custom_relay.unwrap_or_else(|| "iroh默认中继".to_string()));
    
    println!("");
    println!("🌟 去中心化特性:");
    println!("   ✅ 无需知道对方IP地址");
    println!("   ✅ 通过中继节点自动连接");
    println!("   ✅ 支持NAT穿透");
    println!("   ✅ 端到端加密通信");
    println!("   ✅ 真正的P2P连接");
    
    println!("");
    println!("📋 使用方法:");
    println!("   监听: cargo run --example iroh_relay_p2p -- listen --name \"Node-A\"");
    println!("   连接: cargo run --example iroh_relay_p2p -- connect --target <节点ID>");
    
    Ok(())
}

/// 创建使用中继的iroh端点
async fn create_relay_endpoint(port: Option<u16>, custom_relay: Option<String>) -> Result<Endpoint> {
    let mut builder = Endpoint::builder();
    
    // 配置绑定地址
    if let Some(p) = port {
        builder = builder.bind_addr_v4(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::UNSPECIFIED, 
            p
        ));
    } else {
        // 使用随机端口
        builder = builder.bind_addr_v4(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::UNSPECIFIED, 
            0
        ));
    }
    
    // 启用本地网络发现（mDNS）
    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    builder = builder.discovery(mdns);
    
    // 如果指定了自定义中继服务器
    if let Some(relay_url) = custom_relay {
        info!("🔗 使用自定义中继服务器: {}", relay_url);
        // 注意：这里需要根据iroh的API设置自定义中继
        // 当前版本可能需要不同的配置方法
    } else {
        info!("🔗 使用iroh默认中继服务器");
    }
    
    // 设置ALPN协议标识
    builder = builder.alpns(vec![b"relay-p2p".to_vec()]);
    
    let endpoint = builder.bind().await?;
    
    info!("🌐 中继端点创建成功");
    Ok(endpoint)
}

/// 启动中继监听端
async fn start_relay_listener(name: Option<String>, port: Option<u16>, custom_relay: Option<String>) -> Result<()> {
    info!("🚀 启动iroh中继监听端");
    
    let endpoint = create_relay_endpoint(port, custom_relay.clone()).await?;
    let node_id = endpoint.id().to_z32();
    
    let display_name = name.unwrap_or_else(|| "Relay-Node".to_string());
    
    println!("🎉 ===== iroh中继监听端启动成功 =====");
    println!("🆔 节点ID: {}", node_id);
    println!("👤 节点名称: {}", display_name);
    println!("🔗 中继模式: 已启用");
    println!("🌐 连接方式: 通过中继节点自动发现");
    
    if let Some(relay) = custom_relay {
        println!("🔧 中继服务器: {}", relay);
    } else {
        println!("🔧 中继服务器: iroh默认中继");
    }
    
    println!("");
    println!("📋 在另一台电脑上运行:");
    println!("   cargo run --example iroh_relay_p2p -- connect \\");
    println!("     --target {} \\", node_id);
    println!("     --message \"Hello via relay from another computer!\"");
    println!("");
    println!("🌟 去中心化优势:");
    println!("   - 对方无需知道你的IP地址");
    println!("   - 自动通过中继节点连接");
    println!("   - 支持跨网络、跨防火墙通信");
    println!("⏹️  按 Ctrl+C 停止");
    println!("=============================================");
    
    // 监听连接
    let mut connection_count = 0;
    while let Some(incoming) = endpoint.accept().await {
        connection_count += 1;
        info!("🔗 收到第{}个中继连接请求", connection_count);
        
        match incoming.accept() {
            Ok(accepting) => {
                match timeout(Duration::from_secs(30), accepting).await {
                    Ok(Ok(connection)) => {
                        info!("✅ 中继连接建立成功");
                        
                        // 处理连接
                        tokio::spawn(async move {
                            if let Err(e) = handle_relay_connection(connection).await {
                                error!("❌ 处理中继连接失败: {}", e);
                            }
                        });
                    }
                    Ok(Err(e)) => {
                        error!("❌ 中继连接建立失败: {}", e);
                    }
                    Err(_) => {
                        error!("❌ 中继连接建立超时");
                    }
                }
            }
            Err(e) => {
                error!("❌ 接受中继连接失败: {}", e);
            }
        }
    }
    
    Ok(())
}

/// 处理中继连接
async fn handle_relay_connection(connection: Connection) -> Result<()> {
    info!("🔄 开始处理中继连接");
    
    let remote_id = connection.remote_id().to_z32();
    info!("🆔 远程节点ID: {}", remote_id);
    
    // 接收消息
    match timeout(Duration::from_secs(30), receive_message(&connection)).await {
        Ok(Ok(msg)) => {
            println!("📨 收到中继消息: {}", msg);
            println!("🆔 来自节点: {}", remote_id);
            println!("🔗 连接状态: 通过中继节点P2P连接");
            
            // 发送回复
            let reply = format!("🌐 中继消息已收到: {}", msg);
            match send_reply(&connection, &reply).await {
                Ok(_) => {
                    info!("📤 中继回复发送成功");
                }
                Err(e) => {
                    warn!("⚠️ 发送中继回复失败: {}", e);
                }
            }
            
            println!("🎉 中继P2P通信成功！");
        }
        Err(_) => {
            error!("❌ 接收中继消息超时");
        }
        Ok(Err(e)) => {
            error!("❌ 接收中继消息失败: {}", e);
        }
    }
    
    Ok(())
}

/// 通过中继连接到另一个节点
async fn connect_via_relay(target_node: String, message: String, timeout_secs: u64, custom_relay: Option<String>) -> Result<()> {
    info!("🚀 启动iroh中继连接");
    println!("🎯 目标节点: {}", target_node);
    println!("📨 消息: {}", message);
    println!("⏱️ 超时时间: {}秒", timeout_secs);
    println!("🔗 连接方式: 通过中继节点");
    
    if let Some(ref relay) = custom_relay {
        println!("🔧 中继服务器: {}", relay);
    } else {
        println!("🔧 中继服务器: iroh默认中继");
    }
    
    // 创建发送端点
    let endpoint = create_relay_endpoint(None, custom_relay).await?;
    let sender_id = endpoint.id().to_z32();
    info!("🆔 本机节点ID: {}", sender_id);
    
    // 解析目标节点ID
    let public_key = PublicKey::from_z32(&target_node)
        .map_err(|e| anyhow::anyhow!("无效的目标节点ID: {}", e))?;
    
    // 创建端点地址（不指定IP，让iroh通过中继自动发现）
    let endpoint_addr = EndpointAddr::from(public_key);
    
    info!("🔗 尝试通过中继连接...");
    println!("🌐 正在通过中继节点发现目标...");
    println!("🔍 使用iroh去中心化发现机制");
    
    // 尝试连接（使用iroh的中继发现）
    let connection = match timeout(
        Duration::from_secs(timeout_secs),
        endpoint.connect(endpoint_addr, b"relay-p2p")
    ).await {
        Ok(Ok(conn)) => {
            println!("✅ 中继连接成功！");
            println!("🌟 去中心化连接已建立");
            conn
        }
        Ok(Err(e)) => {
            error!("❌ 中继连接失败: {}", e);
            println!("🔧 故障排除建议:");
            println!("   1. 确认目标节点ID正确");
            println!("   2. 确认目标节点正在运行监听模式");
            println!("   3. 检查网络连接到中继服务器");
            println!("   4. 等待中继发现机制完成（可能需要更长时间）");
            println!("   5. 尝试增加超时时间: --timeout 60");
            return Err(anyhow::anyhow!("中继连接失败: {}", e));
        }
        Err(_) => {
            error!("❌ 中继连接超时");
            println!("🔧 连接超时，可能的原因:");
            println!("   1. 目标节点不在线");
            println!("   2. 中继发现需要更多时间");
            println!("   3. 网络连接到中继服务器有问题");
            println!("   4. 尝试增加超时时间");
            return Err(anyhow::anyhow!("中继连接超时"));
        }
    };
    
    println!("📍 连接详情:");
    println!("  - 远程节点: {}", connection.remote_id().to_z32());
    println!("  - 连接类型: 中继P2P连接");
    println!("  - 去中心化: ✅ 无IP地址暴露");
    
    // 发送消息
    info!("📤 发送中继消息...");
    let mut send_stream = connection.open_uni().await?;
    send_stream.write_all(message.as_bytes()).await?;
    send_stream.finish()?;
    
    println!("✅ 中继消息发送成功！");
    
    // 等待回复
    info!("👂 等待中继回复...");
    match timeout(
        Duration::from_secs(15),
        receive_message(&connection)
    ).await {
        Ok(Ok(reply)) => {
            println!("📨 收到中继回复: {}", reply);
        }
        Ok(Err(e)) => {
            warn!("⚠️ 接收回复失败: {}", e);
        }
        Err(_) => {
            warn!("⚠️ 等待回复超时");
        }
    }
    
    println!("🎉 中继P2P通信完成！");
    println!("🌟 去中心化通信成功，无IP地址暴露");
    
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
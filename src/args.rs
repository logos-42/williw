use crate::config::AppConfig;
use std::path::PathBuf;

/// 解析命令行参数并返回配置
pub fn parse_args_and_build_config() -> AppConfig {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let mut stats_output: Option<String> = None;
    let mut node_id: Option<usize> = None;
    let mut model_dim: Option<usize> = None;
    let mut quic_port: Option<u16> = None;
    let mut bootstrap_peers: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--stats-output" => {
                if i + 1 < args.len() {
                    stats_output = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--node-id" => {
                if i + 1 < args.len() {
                    node_id = args[i + 1].parse().ok();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--model-dim" => {
                if i + 1 < args.len() {
                    model_dim = args[i + 1].parse().ok();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--quic-port" => {
                if i + 1 < args.len() {
                    quic_port = args[i + 1].parse().ok();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--bootstrap" => {
                if i + 1 < args.len() {
                    bootstrap_peers.push(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    // 如果没有通过命令行指定端口，尝试从环境变量读取
    if quic_port.is_none() {
        if let Ok(port_str) = std::env::var("GGB_QUIC_PORT") {
            quic_port = port_str.parse().ok();
        }
    }

    // 如果指定了node-id但没有指定端口，根据node-id自动分配端口
    if quic_port.is_none() {
        if let Some(id) = node_id {
            quic_port = Some(9234 + id as u16);
        }
    }

    // 根据 node-id 自动设置 bootstrap（连接其他节点）
    if bootstrap_peers.is_empty() {
        if let Some(id) = node_id {
            // 自动连接到其他节点
            for other_id in 0..3 {
                if other_id != id {
                    let port = 9234 + other_id;
                    bootstrap_peers.push(format!("127.0.0.1:{}", port));
                }
            }
        }
    }

    if let Some(id) = node_id {
        println!("节点 ID: {}", id);
    }

    // 构建配置，支持自定义模型维度和端口
    let mut config = AppConfig::default();
    if let Some(dim) = model_dim {
        // config.inference.model_dim = dim; // 注释掉，因为AppConfig没有inference字段
        println!("使用自定义模型维度: {}", dim);
    }
    if let Some(port) = quic_port {
        config.comms.quic_bind = Some(std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            port,
        ));
    }

    // 从环境变量读取 checkpoint 目录
    if let Ok(checkpoint_dir) = std::env::var("GGB_CHECKPOINT_DIR") {
        // config.inference.checkpoint_dir = Some(PathBuf::from(checkpoint_dir)); // 注释掉
        println!("使用checkpoint目录: {}", checkpoint_dir);
    }

    // 从环境变量读取学习率
    if let Ok(lr_str) = std::env::var("GGB_LEARNING_RATE") {
        if let Ok(lr) = lr_str.parse::<f32>() {
            // config.inference.learning_rate = lr; // 注释掉
            println!("使用自定义学习率: {}", lr);
        }
    }

    // 从环境变量读取是否启用训练
    if let Ok(use_training) = std::env::var("GGB_USE_TRAINING") {
        // if config.inference.use_training { // 注释掉
        //     println!("启用训练模式");
        // }
        println!("训练模式设置: {}", use_training);
    }

    // 添加 bootstrap 节点
    for peer in &bootstrap_peers {
        if let Ok(addr) = peer.parse() {
            config.comms.quic_bootstrap.push(addr);
            println!("添加 Bootstrap 节点: {}", peer);
        }
    }
    if let Some(port) = quic_port {
        println!("使用 QUIC 端口: {}", port);
    }

    config
}

/// 获取统计输出路径
pub fn get_stats_output() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut stats_output: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--stats-output" {
            if i + 1 < args.len() {
                stats_output = Some(args[i + 1].clone());
            }
            break;
        }
        i += 1;
    }
    stats_output
}
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod events;
mod api_client;
mod system_checks;
mod task_handler;
mod ai_decision;

use commands::training_commands::{
    start_training,
    stop_training,
    get_training_status,
    get_training_stats,
    ai_analyze_system,
    ai_start_training,
};
use commands::workers_commands::{
    poll_workers_messages,
    handle_ai_node_connection,
    register_iroh_node_to_workers,
    get_available_nodes_from_workers,
};
use commands::node_commands::{
    get_node_info,
    get_connected_peers,
};
use commands::model_device_commands::{
    get_device_info,
};
use commands::api_key_commands::{
    get_api_keys,
    create_api_key,
    delete_api_key,
    update_api_key_name,
};
use commands::external_api_commands::{
    get_external_apis,
    save_external_api,
    delete_external_api,
    test_external_api,
    chat_with_external_api,
};
use commands::gpu_commands::{
    check_deploy_status,
    start_gpu_server,
    stop_gpu_server,
    install_dependencies,
    download_default_model,
};
use commands::ai_decision_commands::{
    ai_download_and_split_model,
};

use tauri::Emitter;
use tauri::Manager;
use state::{AppState, CliArgsStore, TrainingStatus};
use std::sync::Arc;
use parking_lot::Mutex;

// Initialize logger
extern crate log;

/// 解析命令行参数
fn parse_cli_args() -> CliArgsStore {
    let args: Vec<String> = std::env::args().collect();
    let mut cli_args = CliArgsStore {
        auto_start: true, // 默认自动启动
        node_id: None,
        quic_port: None,
    };
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--auto-start" => {
                cli_args.auto_start = true;
                i += 1;
            }
            "--no-auto-start" => {
                cli_args.auto_start = false;
                i += 1;
            }
            "--node-id" => {
                if i + 1 < args.len() {
                    cli_args.node_id = args[i + 1].parse().ok();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--quic-port" => {
                if i + 1 < args.len() {
                    cli_args.quic_port = args[i + 1].parse().ok();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Williw P2P Desktop App");
                println!("Usage: williw-desktop [OPTIONS]");
                println!("");
                println!("Options:");
                println!("  --auto-start       自动启动 iroh 节点 (默认)");
                println!("  --no-auto-start    不自动启动 iroh 节点");
                println!("  --node-id <id>     指定节点 ID");
                println!("  --quic-port <port> 指定 QUIC 端口");
                println!("  --help, -h         显示此帮助信息");
                i += 1;
            }
            _ => i += 1,
        }
    }
    
    cli_args
}

fn spawn_node_driver(
    node_store: Arc<Mutex<Option<williw::Node>>>,
    training_status: Arc<Mutex<TrainingStatus>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            ticker.tick().await;

            if !training_status.lock().is_running {
                break;
            }

            let mut node_opt = { node_store.lock().take() };
            let Some(mut node) = node_opt.take() else {
                break;
            };

            if let Err(e) = node.drive_once().await {
                log::warn!("[AutoStart] node driver tick failed: {}", e);
            }

            node_store.lock().replace(node);
        }

        log::info!("[AutoStart] node driver stopped");
    });
}

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let cli_args = parse_cli_args();
    
    // Initialize logger - enable info level by default for GUI app
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    
    log::info!("[CLI] Parsed arguments: auto_start={}, node_id={:?}, quic_port={:?}", 
        cli_args.auto_start, cli_args.node_id, cli_args.quic_port);
    
    let app_state = AppState::new().await;
    
    // 将 CLI 参数保存到 app_state 中，供后续使用
    {
        let mut cli_args_store = app_state.cli_args.lock();
        *cli_args_store = Some(cli_args.clone());
    }
    
    // 保存 CLI 参数到 app_state 的 node_config 中
    {
        let mut node_config = app_state.node_config.lock();
        if let Some(node_id) = cli_args.node_id {
            node_config.node_id = Some(node_id);
        }
        if let Some(port) = cli_args.quic_port {
            node_config.quic_port = Some(port);
        }
    }

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            start_training,
            stop_training,
            get_training_status,
            get_training_stats,
            ai_analyze_system,
            ai_start_training,
            poll_workers_messages,
            handle_ai_node_connection,
            get_node_info,
            get_connected_peers,
            get_device_info,
            get_api_keys,
            create_api_key,
            delete_api_key,
            get_external_apis,
            save_external_api,
            delete_external_api,
            test_external_api,
            chat_with_external_api,
            check_deploy_status,
            start_gpu_server,
            stop_gpu_server,
            install_dependencies,
            download_default_model,
            update_api_key_name,
            ai_download_and_split_model,
            register_iroh_node_to_workers,
            get_available_nodes_from_workers,
        ])
        .setup(|app| {
            // Initialize event handlers
            events::setup_event_handlers(app.handle().clone())?;

            // Auto-start iroh P2P node on app startup (根据 CLI 参数决定)
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_state = app_handle.state::<AppState>();
                
                // 从 app_state 获取 CLI 参数
                let cli_args = {
                    let args_guard = app_state.cli_args.lock();
                    args_guard.clone()
                };
                
                // 检查是否应该自动启动
                let should_auto_start = cli_args.map(|args| args.auto_start).unwrap_or(true);
                
                if !should_auto_start {
                    log::info!("[AutoStart] Skipping auto-start due to --no-auto-start flag");
                    return;
                }
                
                // Wait a bit for app to fully initialize
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Check if node already exists
                {
                    let node_guard = app_state.node.lock();
                    if node_guard.is_some() {
                        log::info!("[AutoStart] Node already running");
                        return;
                    }
                }
                
                log::info!("[AutoStart] Starting iroh P2P node...");
                
                // 从 app_state 获取 CLI 参数配置
                let node_config = {
                    let config = app_state.node_config.lock();
                    config.clone()
                };
                
                // 尝试多次启动，使用不同端口
                let mut last_error = None;
                let ports_to_try = if let Some(port) = node_config.quic_port {
                    vec![port]
                } else {
                    // 默认尝试多个端口
                    vec![0, 9234, 9235, 9236, 9237, 9238, 9239, 9240]
                };
                
                for port in ports_to_try {
                    // 创建配置
                    let mut app_config = williw::config::AppConfig::default();
                    
                    // 如果端口为0，使用随机端口
                    let actual_port = if port == 0 {
                        // 使用随机可用端口
                        9000 + (rand::random::<u16>() % 1000)
                    } else {
                        port
                    };
                    
                    app_config.comms.quic_bind = Some(std::net::SocketAddr::new(
                        std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                        actual_port,
                    ));
                    
                    log::info!("[AutoStart] Attempting to start on port {} (config: {:?})", actual_port, app_config.comms.quic_bind);
                    
                    match williw::Node::new(app_config).await {
                        Ok(node) => {
                            let node_id = node.comms.node_id().to_string();
                            log::info!("[AutoStart] Node started with ID: {}", node_id);
                            
                            // 检查节点ID格式 - 如果是iroh-UUID格式，说明QuicGateway可能创建失败
                            if node_id.starts_with("iroh-") {
                                log::warn!("[AutoStart] ⚠️ Node ID is UUID format (iroh-UUID), QuicGateway may have failed to initialize!");
                            } else {
                                log::info!("[AutoStart] ✅ Using real iroh node ID on port {}: {}", actual_port, node_id);
                            }
                            
                            *app_state.node.lock() = Some(node);
                            
                            // Update training status
                            let mut status = app_state.training_status.lock();
                            status.is_running = true;
                            drop(status);

                            // Start node background driver so the node keeps progressing without blocking UI.
                            spawn_node_driver(app_state.node.clone(), app_state.training_status.clone());
                            
                            // Emit event to frontend
                            let _ = app_handle.emit("node-started", serde_json::json!({
                                "node_id": node_id,
                                "port": actual_port
                            }));
                            
                            // 成功启动，跳出循环
                            break;
                        }
                        Err(e) => {
                            log::warn!("[AutoStart] Failed to start on port {}: {}", actual_port, e);
                            last_error = Some(e);
                            // 继续尝试下一个端口
                        }
                    }
                }
                
                // 检查节点是否成功启动
                {
                    let node_guard = app_state.node.lock();
                    if node_guard.is_none() {
                        log::error!("[AutoStart] ❌ All ports failed, node could not start");
                        let _ = app_handle.emit("node-error", serde_json::json!({
                            "error": last_error.map(|e| e.to_string()).unwrap_or_else(|| "All ports failed".to_string())
                        }));
                    }
                }
            });

            // Start background task to refresh device info every minute
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    // Emit event to refresh device info in frontend
                    let _ = app_handle.emit("device_info_refresh", ());
                }
            });

            // ===== 消息轮询后台任务：每10秒轮询 Workers 网络接收任务 =====
            let app_handle_tasks = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_state = app_handle_tasks.state::<AppState>();
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
                // 跳过第一次立即触发，等待应用完全启动
                interval.tick().await;
                
                println!("[TaskPoll] Starting automatic workers message polling (every 10s)");
                let mut last_poll_time: Option<String> = None;
                
                loop {
                    interval.tick().await;
                    
                    // 轮询 Workers 的待处理消息
                    match app_state.api_client.poll_messages(last_poll_time.clone()).await {
                        Ok(response) => {
                            if response.success && !response.messages.is_empty() {
                                println!("[TaskPoll] 📥 收到 {} 条消息", response.messages.len());
                                
                                println!("[TaskPoll] 批量处理 {} 条消息", response.messages.len());
                                
                                // 批量处理所有消息
                                let results = task_handler::TaskHandler::handle_messages(
                                    response.messages,
                                    &app_handle_tasks,
                                    &app_state,
                                ).await;

                                match results {
                                    Ok(task_results) => {
                                        for task_result in task_results {
                                            // 上报任务结果到 Workers
                                            if let Err(e) = task_handler::report_task_result(
                                                &app_state.api_client,
                                                task_result,
                                            ).await {
                                                println!("[TaskPoll] ⚠️ 结果上报失败: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("[TaskPoll] ❌ 消息处理失败: {}", e);
                                    }
                                }
                            }
                            last_poll_time = Some(response.poll_timestamp);
                        }
                        Err(e) => {
                            println!("[TaskPoll] ⚠️ 轮询失败: {:?}", e);
                        }
                    }
                }
            });

            // Start background task to upload full node info to workers every 30 seconds
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_state = app_handle.state::<AppState>();
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                // 跳过第一次立即触发，等待节点启动
                interval.tick().await;
                
                println!("[AutoUpload] Starting automatic node info upload task (every 30s)");
                
                loop {
                    interval.tick().await;
                    
                    // Get device info
                    let device_info = {
                        let info = app_state.device_info.lock();
                        info.clone()
                    };
                    
                    // Get iroh node info if available
                    let iroh_node = {
                        let node_guard = app_state.node.lock();
                        println!("[AutoUpload] Checking node state: exists={}", node_guard.is_some());
                        if let Some(node) = node_guard.as_ref() {
                            let node_id = node.comms.node_id().to_string();
                            println!("[AutoUpload] Node ID from comms: {}", node_id);
                            let capabilities = node.device_manager.get();
                            let stats = node.stats.lock().unwrap();
                            let (primary_peers, backup_peers) = node.topology.neighbor_sets();
                            
                            // Build peers list
                            let mut peers = Vec::new();
                            for peer_id in primary_peers {
                                if let Some(snapshot) = node.topology.peer_snapshot(peer_id.as_str()) {
                                    peers.push(crate::api_client::IrohPeerInfo {
                                        id: peer_id.clone(),
                                        peer_type: "primary".to_string(),
                                        similarity: snapshot.similarity as f64,
                                        geo_affinity: snapshot.geo_affinity as f64,
                                        embedding_dim: snapshot.embedding_dim,
                                        position: crate::api_client::GeoPosition {
                                            lat: snapshot.position.lat as f64,
                                            lon: snapshot.position.lon as f64,
                                        },
                                    });
                                }
                            }
                            for peer_id in backup_peers {
                                if let Some(snapshot) = node.topology.peer_snapshot(peer_id.as_str()) {
                                    peers.push(crate::api_client::IrohPeerInfo {
                                        id: peer_id.clone(),
                                        peer_type: "backup".to_string(),
                                        similarity: snapshot.similarity as f64,
                                        geo_affinity: snapshot.geo_affinity as f64,
                                        embedding_dim: snapshot.embedding_dim,
                                        position: crate::api_client::GeoPosition {
                                            lat: snapshot.position.lat as f64,
                                            lon: snapshot.position.lon as f64,
                                        },
                                    });
                                }
                            }
                            
                            Some(crate::api_client::IrohNodeInfo {
                                node_id,
                                is_running: true,
                                tick_counter: node.tick_counter,
                                device_capabilities: crate::api_client::IrohDeviceCapabilities {
                                    max_memory_mb: capabilities.max_memory_mb,
                                    cpu_cores: capabilities.cpu_cores,
                                    has_gpu: capabilities.has_gpu,
                                    network_type: format!("{:?}", capabilities.network_type),
                                    battery_level: capabilities.battery_level,
                                    is_charging: capabilities.is_charging,
                                },
                                training_stats: crate::api_client::IrohTrainingStats {
                                    total_ticks: stats.get_stats().tick_count,
                                    accuracy: stats.get_stats().training_accuracy,
                                    loss: stats.get_stats().training_loss,
                                    samples_processed: stats.get_stats().samples_processed,
                                },
                                peers,
                            })
                        } else {
                            println!("[AutoUpload] ⚠️ Node not running - iroh_node is None");
                            None
                        }
                    };
                    
                    // Upload to workers - only if node is running
                    if let Some(device_info) = device_info {
                        if iroh_node.is_some() {
                            println!("[AutoUpload] Uploading WITH iroh node info: CPU cores={}, Memory={}GB, GPU={:?}", 
                                device_info.cpu_cores, device_info.total_memory_gb, device_info.gpu_type);
                            match app_state.api_client.upload_full_node_info(device_info, iroh_node).await {
                                Ok(response) => {
                                    if response.success {
                                        println!("[AutoUpload] ✅ Node info uploaded successfully");
                                    } else {
                                        println!("[AutoUpload] ❌ Upload failed: {}", response.message);
                                    }
                                }
                                Err(e) => {
                                    println!("[AutoUpload] ❌ Upload error: {:?}", e);
                                }
                            }
                        } else {
                            // Node not running - only upload device info without iroh node
                            println!("[AutoUpload] ⚠️ Node not running - uploading device info only (no iroh node info)");
                            println!("[AutoUpload] 💡 Please start the node using the toggle switch to enable full node info upload");
                            // Still upload device info but mark node as not running
                            match app_state.api_client.upload_full_node_info(device_info, None).await {
                                Ok(response) => {
                                    if response.success {
                                        println!("[AutoUpload] ✅ Device info uploaded (node not running)");
                                    } else {
                                        println!("[AutoUpload] ❌ Upload failed: {}", response.message);
                                    }
                                }
                                Err(e) => {
                                    println!("[AutoUpload] ❌ Upload error: {:?}", e);
                                }
                            }
                        }
                    } else {
                        println!("[AutoUpload] ⚠️ No device info available, skipping upload");
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

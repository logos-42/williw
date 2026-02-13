// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod events;
mod api_client;
mod system_checks;

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
};
use commands::node_commands::{
    get_node_info,
    get_connected_peers,
};
use commands::model_device_commands::{
    get_device_info,
};

use tauri::Emitter;
use tauri::Manager;
use state::AppState;

// Initialize logger
extern crate log;

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::init();
    
    let app_state = AppState::new().await;

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
        ])
        .setup(|app| {
            // Initialize event handlers
            events::setup_event_handlers(app.handle().clone())?;

            // Auto-start iroh P2P node on app startup
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_state = app_handle.state::<AppState>();
                
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
                
                // Create default config
                let app_config = williw::config::AppConfig::default();
                
                match williw::Node::new(app_config).await {
                    Ok(node) => {
                        let node_id = node.comms.node_id().to_string();
                        log::info!("[AutoStart] Node started with ID: {}", node_id);
                        *app_state.node.lock() = Some(node);
                        
                        // Update training status
                        let mut status = app_state.training_status.lock();
                        status.is_running = true;
                        
                        // Emit event to frontend
                        let _ = app_handle.emit("node-started", serde_json::json!({
                            "node_id": node_id
                        }));
                    }
                    Err(e) => {
                        log::error!("[AutoStart] Failed to start node: {}", e);
                        let _ = app_handle.emit("node-error", serde_json::json!({
                            "error": e.to_string()
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
                            None
                        }
                    };
                    
                    // Upload to workers
                    if let Some(device_info) = device_info {
                        println!("[AutoUpload] Uploading node info: CPU cores={}, Memory={}GB, GPU={:?}", 
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
                        println!("[AutoUpload] ⚠️ No device info available, skipping upload");
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

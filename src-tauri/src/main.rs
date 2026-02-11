// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod events;
mod api_client;

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
            commands::start_training,
            commands::stop_training,
            commands::get_training_status,
            commands::select_model,
            commands::get_available_models,
            commands::get_device_info,
            commands::get_training_stats,
            commands::update_settings,
            commands::get_settings,
            commands::get_api_keys,
            commands::create_api_key,
            commands::delete_api_key,
            commands::update_api_key_name,
            commands::get_node_info,
            commands::get_connected_peers,
            commands::upload_device_info_to_workers,
            commands::upload_model_selection_to_workers,
            commands::upload_training_data_to_workers,
            commands::test_workers_connection,
            commands::request_inference_from_workers,
            commands::reassign_node_from_workers,
            commands::check_node_health_from_workers,
            commands::upload_full_node_info_to_workers,
            commands::start_gpu_server,
            commands::check_gpu_server_status,
            commands::install_gpu_dependencies,
            commands::get_workflow_status,
            commands::start_document_driven_workflow,
            commands::run_ai_setup,
            commands::check_setup_status,
            commands::start_gpu_inference_server,
            commands::test_external_api,
            commands::save_external_api,
            commands::get_external_apis,
            commands::delete_external_api,
            commands::toggle_external_api,
            commands::chat_with_external_api,
        ])
        .setup(|app| {
            // Initialize event handlers
            events::setup_event_handlers(app.handle().clone())?;

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
                
                log::info!("[AutoUpload] Starting automatic node info upload task (every 30s)");
                
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
                        if let Some(node) = node_guard.as_ref() {
                            let node_id = node.comms.node_id().to_string();
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
                        match app_state.api_client.upload_full_node_info(device_info, iroh_node).await {
                            Ok(response) => {
                                if response.success {
                                    log::info!("[AutoUpload] Node info uploaded successfully");
                                } else {
                                    log::warn!("[AutoUpload] Upload failed: {}", response.message);
                                }
                            }
                            Err(e) => {
                                log::error!("[AutoUpload] Upload error: {}", e);
                            }
                        }
                    } else {
                        log::warn!("[AutoUpload] No device info available, skipping upload");
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

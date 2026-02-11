use crate::state::AppState;
use tauri::State;
use serde_json;

/// Get node information
#[tauri::command]
pub fn get_node_info(
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let node_guard = state.node.lock();

    if let Some(node) = node_guard.as_ref() {
        let node_id = node.comms.node_id().to_string();
        let capabilities = node.device_manager.get();
        let stats = node.stats.lock().unwrap();

        Ok(serde_json::json!({
            "id": node_id,
            "is_running": true,
            "tick_counter": node.tick_counter,
            "device_capabilities": {
                "max_memory_mb": capabilities.max_memory_mb,
                "cpu_cores": capabilities.cpu_cores,
                "has_gpu": capabilities.has_gpu,
                "network_type": capabilities.network_type,
                "battery_level": capabilities.battery_level,
                "is_charging": capabilities.is_charging
            },
            "training_stats": {
                "total_ticks": stats.get_stats().tick_count,
                "accuracy": stats.get_stats().training_accuracy,
                "loss": stats.get_stats().training_loss,
                "samples_processed": stats.get_stats().samples_processed
            }
        }))
    } else {
        Ok(serde_json::json!({
            "id": null,
            "is_running": false,
            "message": "Node is not running"
        }))
    }
}

/// Get connected peers information
#[tauri::command]
pub fn get_connected_peers(
    state: State<'_, AppState>
) -> Result<Vec<serde_json::Value>, String> {
    let node_guard = state.node.lock();

    if let Some(node) = node_guard.as_ref() {
        let (primary_peers, backup_peers) = node.topology.neighbor_sets();

        let mut peers = Vec::new();

        for peer_id in primary_peers {
            if let Some(snapshot) = node.topology.peer_snapshot(&peer_id) {
                peers.push(serde_json::json!({
                    "id": peer_id,
                    "type": "primary",
                    "similarity": snapshot.similarity,
                    "geo_affinity": snapshot.geo_affinity,
                    "embedding_dim": snapshot.embedding_dim,
                    "position": {
                        "lat": snapshot.position.lat,
                        "lon": snapshot.position.lon
                    }
                }));
            }
        }

        for peer_id in backup_peers {
            if let Some(snapshot) = node.topology.peer_snapshot(&peer_id) {
                peers.push(serde_json::json!({
                    "id": peer_id,
                    "type": "backup",
                    "similarity": snapshot.similarity,
                    "geo_affinity": snapshot.geo_affinity,
                    "embedding_dim": snapshot.embedding_dim,
                    "position": {
                        "lat": snapshot.position.lat,
                        "lon": snapshot.position.lon
                    }
                }));
            }
        }

        Ok(peers)
    } else {
        Ok(vec![])
    }
}
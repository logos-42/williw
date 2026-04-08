use crate::state::AppState;
use tauri::State;
use serde_json;
use std::process::Command;

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

/// 获取本节点的去中心化算力状态（用于分布式推理面板展示）
/// 返回：P2P节点状态、硬件能力、可承担的层数估算
#[tauri::command]
pub async fn get_distributed_node_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // 1. P2P 节点状态
    let p2p_status = {
        let node_guard = state.node.lock();
        if let Some(node) = node_guard.as_ref() {
            let node_id = node.comms.node_id().to_string();
            let (primary, backup) = node.topology.neighbor_sets();
            let peer_count = primary.len() + backup.len();
            serde_json::json!({
                "running": true,
                "node_id": &node_id[..node_id.len().min(16)], // 截短显示
                "node_id_full": node_id,
                "peer_count": peer_count,
                "tick_counter": node.tick_counter,
            })
        } else {
            serde_json::json!({
                "running": false,
                "node_id": null,
                "peer_count": 0,
            })
        }
    };

    // 2. 本地硬件能力
    let ram_gb: u64 = Command::new("sh")
        .arg("-c")
        .arg("sysctl -n hw.memsize 2>/dev/null || free -b 2>/dev/null | awk '/Mem:/{print $2}' || echo 0")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
        .map(|b| b / (1024 * 1024 * 1024))
        .unwrap_or(8);

    let cpu_cores: u32 = Command::new("sh")
        .arg("-c")
        .arg("sysctl -n hw.logicalcpu 2>/dev/null || nproc 2>/dev/null || echo 4")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or(4);

    let arch = std::env::consts::ARCH;
    let is_apple_silicon = arch == "aarch64";

    // 3. 估算本节点可承担的最大层数
    // 粗算：1B 参数 ≈ 2GB RAM（fp16），1GB RAM ≈ 可承担约 2 层（7B 28层÷14GB）
    let usable_ram_gb = ram_gb.saturating_sub(4); // 预留 4GB 给系统
    let max_layers_estimate = (usable_ram_gb * 2).min(80) as u32; // 最多 80 层（72B）

    // 4. 推荐本节点承担的任务
    let recommended_role = if ram_gb >= 16 {
        "inference_primary"   // 可独立推理 1.5B-7B 模型
    } else if ram_gb >= 8 {
        "inference_shard"     // 可承担部分模型层（分布式）
    } else {
        "coordinator"         // 仅协调，不承担推理
    };

    Ok(serde_json::json!({
        "p2p": p2p_status,
        "hardware": {
            "ram_gb": ram_gb,
            "cpu_cores": cpu_cores,
            "arch": arch,
            "is_apple_silicon": is_apple_silicon,
            "max_layers_estimate": max_layers_estimate,
        },
        "compute": {
            "recommended_role": recommended_role,
            "can_participate_distributed": ram_gb >= 4,
            "usable_ram_gb": usable_ram_gb,
        },
    }))
}

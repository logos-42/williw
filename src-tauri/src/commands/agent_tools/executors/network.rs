/// Network tools executor
///
/// Provides network diagnosis and process management capabilities.

use std::process::Command;
use serde_json;

/// Perform network diagnosis including ping, DNS lookup, and port checking.
pub async fn network_diagnosis(target: &str, operation: &str, port: u16) -> serde_json::Value {
    log::info!("[Agent] 网络诊断：{} operation={}", target, operation);

    let mut results = serde_json::json!({});

    if operation == "ping" || operation == "all" {
        let ping_output = Command::new("sh")
            .arg("-c")
            .arg(format!("ping -c 3 '{}' 2>&1 | tail -1", target))
            .output();

        if let Ok(o) = ping_output {
            results["ping"] = serde_json::json!({
                "success": o.status.success(),
                "output": String::from_utf8_lossy(&o.stdout).trim()
            });
        }
    }

    if operation == "dns" || operation == "all" {
        let dns_output = Command::new("sh")
            .arg("-c")
            .arg(format!("nslookup '{}' 2>&1", target))
            .output();

        if let Ok(o) = dns_output {
            results["dns"] = serde_json::json!({
                "success": o.status.success(),
                "output": String::from_utf8_lossy(&o.stdout).trim()
            });
        }
    }

    if operation == "port" || operation == "all" {
        let port_output = Command::new("sh")
            .arg("-c")
            .arg(format!("nc -zv -w3 '{}' {} 2>&1", target, port))
            .output();

        if let Ok(o) = port_output {
            results["port"] = serde_json::json!({
                "success": o.status.success(),
                "port": port,
                "output": String::from_utf8_lossy(&o.stdout).trim()
            });
        }
    }

    serde_json::json!({
        "success": true,
        "target": target,
        "operation": operation,
        "results": results
    })
}

/// Terminate a running process by name.
pub async fn kill_process(process_name: &str, force: bool) -> serde_json::Value {
    log::info!("[Agent] 终止进程：name={}, force={}", process_name, force);

    let signal = if force { "-9" } else { "" };
    let command = if cfg!(target_os = "windows") {
        if force {
            format!("taskkill /F /IM {}", process_name)
        } else {
            format!("taskkill /IM {}", process_name)
        }
    } else {
        if force {
            format!("pkill -9 {}", process_name)
        } else {
            format!("pkill {}", process_name)
        }
    };

    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output();

    match output {
        Ok(o) => {
            let success = o.status.success();
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();

            serde_json::json!({
                "success": success,
                "process_name": process_name,
                "force": force,
                "stdout": stdout,
                "stderr": stderr,
                "message": if success {
                    format!("进程 {} 已终止", process_name)
                } else {
                    format!("终止失败：{}", stderr)
                }
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "process_name": process_name,
            "error": format!("执行错误：{}", e)
        })
    }
}

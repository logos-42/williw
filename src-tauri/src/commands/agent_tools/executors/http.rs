/// HTTP tools executor
///
/// Provides HTTP endpoint checking and condition waiting capabilities.

use std::process::Command;
use std::fs;
use serde_json;

/// Check if an HTTP endpoint is reachable (whether the service is running).
pub async fn check_http_endpoint(url: &str) -> serde_json::Value {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    match client.get(url).send().await {
        Ok(resp) => serde_json::json!({
            "reachable": true,
            "status_code": resp.status().as_u16(),
            "ok": resp.status().is_success()
        }),
        Err(e) => serde_json::json!({
            "reachable": false,
            "error": format!("{}", e)
        })
    }
}

/// Poll HTTP endpoint, command, or file until expected pattern matches.
pub async fn wait_for_condition(
    target: &str,
    target_type: &str,
    expected: &str,
    max_attempts: u32,
    interval_secs: u64,
    _timeout_secs: u64,
) -> serde_json::Value {
    log::info!("[Agent] 等待条件：target={}, type={}, expected={}", target, target_type, expected);

    let interval = tokio::time::Duration::from_secs(interval_secs);
    let mut matched = false;
    let mut attempts = 0;

    for attempt in 0..max_attempts {
        attempts = attempt + 1;

        let check_result = match target_type {
            "http" => {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .unwrap_or_default();

                match client.get(target).send().await {
                    Ok(resp) => {
                        let body = resp.text().await.unwrap_or_default();
                        serde_json::json!({ "matched": body.contains(expected), "content": body })
                    }
                    Err(_) => serde_json::json!({ "matched": false })
                }
            }
            "command" => {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(target)
                    .output()
                    .ok();

                match output {
                    Some(o) => {
                        let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                        let combined = format!("{}\n{}", stdout, stderr);
                        serde_json::json!({ "matched": combined.contains(expected), "content": combined })
                    }
                    None => serde_json::json!({ "matched": false })
                }
            }
            "file" => {
                match fs::read_to_string(target) {
                    Ok(content) => serde_json::json!({ "matched": content.contains(expected), "content": content }),
                    Err(_) => serde_json::json!({ "matched": false })
                }
            }
            _ => serde_json::json!({ "matched": false, "error": "未知目标类型" })
        };

        if check_result.get("matched").and_then(|v| v.as_bool()).unwrap_or(false) {
            matched = true;
            break;
        }

        if attempt < max_attempts - 1 {
            tokio::time::sleep(interval).await;
        }
    }

    serde_json::json!({
        "success": matched,
        "matched": matched,
        "attempts": attempts,
        "max_attempts": max_attempts,
        "message": if matched {
            format!("条件在第 {} 次尝试后匹配", attempts)
        } else {
            format!("在 {} 次尝试后仍未匹配", max_attempts)
        }
    })
}

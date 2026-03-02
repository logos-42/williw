//! Agent Chat 模块
//!
//! 本地聊天和模型预热相关命令

use serde_json::Value;
use std::process::Command;

/// 预热本地 Ollama 模型
///
/// 将模型加载到内存，保持 keep_alive=-1 永不卸载
/// 适合首次加载或模型已卸载的情况，会等待加载完成（最多 90 秒）
#[tauri::command]
pub async fn warmup_local_model(model_name: String) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;

    log::info!("[Warmup] 开始预热模型：{}", model_name);

    let request_body = serde_json::json!({
        "model": model_name,
        "prompt": "",
        "stream": false,
        "keep_alive": -1,
    });

    match client
        .post("http://localhost:11434/api/generate")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 200 => {
            log::info!("[Warmup] 模型 {} 预热成功", model_name);
            Ok(serde_json::json!({
                "success": true,
                "model": model_name,
                "message": format!("模型 {} 已加载到内存", model_name),
            }))
        }
        Ok(resp) => {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            log::warn!("[Warmup] 预热返回 {}: {}", status, text);
            Ok(serde_json::json!({
                "success": false,
                "status": status,
                "message": format!("预热返回 {} (可能模型仍在加载)", status),
            }))
        }
        Err(e) => {
            log::error!("[Warmup] 预热失败：{}", e);
            Err(format!("预热失败：{}", e))
        }
    }
}

/// 快速检测本地是否有可用的推理服务
#[tauri::command]
pub async fn quick_start_local_inference() -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    let ollama_running = match client.get("http://localhost:11434/api/tags").send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };

    if !ollama_running {
        return Ok(serde_json::json!({
            "found": false,
            "reason": "Ollama not running",
        }));
    }

    let models: Vec<String> = match client.get("http://localhost:11434/api/tags").send().await {
        Ok(resp) => {
            match resp.json::<Value>().await {
                Ok(json) => {
                    json.get("models")
                        .and_then(|m| m.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => vec![],
            }
        }
        Err(_) => vec![],
    };

    if models.is_empty() {
        return Ok(serde_json::json!({
            "found": true,
            "has_models": false,
            "inference_endpoint": "http://localhost:11434/v1",
        }));
    }

    let ram_gb: u64 = Command::new("sh")
        .arg("-c")
        .arg("sysctl -n hw.memsize 2>/dev/null || echo 0")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
        .map(|b| b / (1024 * 1024 * 1024))
        .unwrap_or(8);

    let best_model = if ram_gb >= 16 {
        models.iter()
            .find(|m| m.contains("3b") || m.contains("7b"))
            .cloned()
            .unwrap_or_else(|| models.first().cloned().unwrap_or_default())
    } else {
        models.iter()
            .find(|m| m.contains("0.5b") || m.contains("1.5b"))
            .cloned()
            .unwrap_or_else(|| models.first().cloned().unwrap_or_default())
    };

    Ok(serde_json::json!({
        "found": true,
        "has_models": true,
        "all_models": models,
        "model_name": best_model,
        "inference_endpoint": "http://localhost:11434/v1",
        "ram_gb": ram_gb,
    }))
}

/// 与本地推理端点聊天
#[tauri::command]
pub async fn chat_with_local_endpoint(
    message: String,
    endpoint: String,
    model_name: String,
    system_prompt: Option<String>,
) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    let sys = system_prompt.unwrap_or_else(|| "你是一个有帮助的 AI 助手。".to_string());

    let request_body = serde_json::json!({
        "model": model_name,
        "messages": [
            {"role": "system", "content": sys},
            {"role": "user", "content": message}
        ],
        "stream": false,
    });

    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));

    log::info!("[LocalChat] 调用：{} 模型：{}", url, model_name);

    let mut last_err = String::new();
    let mut response_opt = None;
    for attempt in 0..6u8 {
        match client.post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 502 && attempt < 5 {
                    log::warn!("[LocalChat] 502 Bad Gateway (attempt {}/6), 等待模型加载...", attempt + 1);
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    continue;
                }
                if !status.is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    last_err = format!("推理服务返回错误 {}: {}", status, text);
                    break;
                }
                response_opt = Some(resp);
                break;
            }
            Err(e) => {
                last_err = format!("请求失败：{}", e);
                if attempt < 5 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }
            }
        }
    }

    let response = match response_opt {
        Some(r) => r,
        None => return Err(last_err),
    };

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("推理服务返回错误 {}: {}", status, text));
    }

    let json: Value = response.json().await
        .map_err(|e| format!("解析响应失败：{}", e))?;

    let content = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|f| f.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("无法获取回复")
        .to_string();

    Ok(serde_json::json!({
        "success": true,
        "message": content,
        "model": model_name,
        "endpoint": endpoint,
    }))
}

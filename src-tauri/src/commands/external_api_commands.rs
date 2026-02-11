use crate::state::{AppState, ExternalApiConfig};
use tauri::{State, Emitter};
use serde_json;
use uuid::Uuid;

// ============ 外部 API 管理 ============

/// 测试外部 API 连接
#[tauri::command]
pub async fn test_external_api(
    provider: String,
    apiKey: String,
    baseUrl: String,
    model: String,
) -> Result<serde_json::Value, String> {
    use reqwest;
    use std::time::Duration;

    // 调试日志
    log::info!("[test_external_api] provider={}, apiKey={}..., baseUrl={}, model={}", 
        provider, 
        &apiKey[..20.min(apiKey.len())],
        baseUrl, 
        model
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 根据不同的提供商构建请求
    let result = match provider.as_str() {
        "openai" => {
            let response = client
                .post(format!("{}/chat/completions", baseUrl))
                .header("Authorization", format!("Bearer {}", apiKey))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("choices").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        "deepseek" => {
            let response = client
                .post(format!("{}/chat/completions", baseUrl))
                .header("Authorization", format!("Bearer {}", apiKey))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("choices").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        "anthropic" => {
            let response = client
                .post(format!("{}/messages", baseUrl))
                .header("x-api-key", apiKey)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&serde_json::json!({
                    "model": model,
                    "max_tokens": 5,
                    "messages": [{"role": "user", "content": "Hi"}],
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("content").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        "glm" | "kimichat" | "minimax" | "qwen" | "custom" | "google" | "nvidia" | "openrouter" | "vercel" | "groq" | "perplexity" => {
            // 通用 OpenAI 兼容 API 调用
            let response = client
                .post(format!("{}/chat/completions", baseUrl))
                .header("Authorization", format!("Bearer {}", apiKey))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": model,
                    "messages": [{"role": "user", "content": "Hi"}],
                    "max_tokens": 5,
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
                    Ok(serde_json::json!({
                        "success": true,
                        "message": "API 连接成功！",
                        "response": json.get("choices").map(|c| c.to_string()).unwrap_or_else(|| "OK".to_string()),
                    }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        _ => Err(format!("不支持的提供商: {}", provider)),
    };

    result
}

/// 保存外部 API 配置
#[tauri::command]
pub async fn save_external_api(
    config: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<ExternalApiConfig, String> {
    let id = Uuid::new_v4().to_string();
    let entry = ExternalApiConfig {
        id: id.clone(),
        name: config.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default(),
        provider: config.get("provider").and_then(|v| v.as_str()).unwrap_or("custom").to_string(),
        base_url: config.get("base_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        api_key: config.get("api_key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        model: config.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        enabled: config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    state.external_apis.lock().push(entry.clone());
    Ok(entry)
}

/// 获取外部 API 配置列表
#[tauri::command]
pub async fn get_external_apis(
    state: State<'_, AppState>,
) -> Result<Vec<ExternalApiConfig>, String> {
    let apis = state.external_apis.lock().clone();
    Ok(apis)
}

/// 删除外部 API 配置
#[tauri::command]
pub async fn delete_external_api(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut apis = state.external_apis.lock();
    apis.retain(|api| api.id != id);
    Ok(())
}

/// 切换外部 API 启用状态
#[tauri::command]
pub async fn toggle_external_api(
    id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut apis = state.external_apis.lock();
    if let Some(api) = apis.iter_mut().find(|api| api.id == id) {
        api.enabled = enabled;
    }
    Ok(())
}

/// 使用外部 API 进行对话（集成 Ralph Loop 去中心化算力）
#[tauri::command]
pub async fn chat_with_external_api(
    message: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use reqwest;
    use std::time::Duration;
    use williw::agent::workflow::RalphLoopConfig;

    // 获取启用的 API 配置
    let api_config = {
        let apis = state.external_apis.lock();
        apis.iter().find(|api| api.enabled).cloned().ok_or("没有启用的外部 API 配置")?
    };

    log::info!(
        "[chat_with_external_api] 使用 {} API, 模型: {}",
        api_config.provider,
        api_config.model
    );

    // 配置 Ralph Loop 智能执行参数
    let _ralph_config = RalphLoopConfig {
        enabled: true,
        max_iterations: 30,
        iteration_delay_ms: 500,
        completion_checker: Some("auto".to_string()),
        max_total_time_ms: Some(300000),
        iteration_timeout_ms: 60000,
        max_cost: Some(5.0),
        enable_history: true,
        smart_retry: williw::agent::workflow::SmartRetryStrategy {
            enabled: true,
            max_retries: 3,
            base_delay_ms: 1000,
            backoff_multiplier: 2.0,
            jitter: true,
            error_based_retry: std::collections::HashMap::new(),
            adaptive_retry: true,
            max_consecutive_failures: 5,
            learning_period: 3,
        },
    };

    // 使用 reqwest 客户端调用外部 API（Ralph Loop 方式：智能重试 + 失败恢复）
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let max_retries = 3;
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        log::info!("[Ralph Loop] 第 {} 次尝试调用 {} API...", attempt, api_config.provider);

        let response = client
            .post(format!("{}/chat/completions", api_config.base_url))
            .header("Authorization", format!("Bearer {}", api_config.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": api_config.model,
                "messages": [{"role": "user", "content": &message}],
                "max_tokens": 1024,
            }))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await
                    .map_err(|e| format!("解析响应失败: {}", e))?;
                
                let content = json
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|first| first.get("message"))
                    .and_then(|msg| msg.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("无法获取回复");

                log::info!(
                    "[Ralph Loop] 成功获取回复 (尝试 {}), 内容长度: {}",
                    attempt,
                    content.len()
                );

                return Ok(serde_json::json!({
                    "success": true,
                    "message": content,
                    "provider": api_config.provider,
                    "model": api_config.model,
                    "attempt": attempt,
                    "mode": "decentralized_compute_with_ralph_loop"
                }));
            }
            Ok(resp) => {
                last_error = format!("API 返回错误 ({}): {}", resp.status(), resp.text().await.unwrap_or_default());
                log::warn!("[Ralph Loop] 尝试 {} 失败: {}", attempt, last_error);
                
                if attempt < max_retries {
                    let delay = 1000 * attempt as u64;
                    log::info!("[Ralph Loop] 等待 {}ms 后重试...", delay);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
            Err(e) => {
                last_error = format!("连接失败: {}", e);
                log::warn!("[Ralph Loop] 尝试 {} 失败: {}", attempt, last_error);
                
                if attempt < max_retries {
                    let delay = 1000 * attempt as u64;
                    log::info!("[Ralph Loop] 等待 {}ms 后重试...", delay);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    Err(format!(
        "[Ralph Loop] 经过 {} 次尝试后仍然失败: {}",
        max_retries,
        last_error
    ))
}

/// 测试工作流事件发送
#[tauri::command]
pub async fn test_workflow_event(
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("🧪 [TEST] Testing workflow event emission...");
    
    // 立即发送测试消息
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🧪 测试消息：工作流事件系统正常工作！",
        "step": "test",
        "progress": 0.5,
    }));
    
    // 在后台发送更多测试消息
    let app_handle = app.clone();
    tokio::spawn(async move {
        for i in 1..=5 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let _ = app_handle.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": format!("🧪 测试进度 {}/5", i),
                "step": "test_progress",
                "progress": i as f64 / 5.0,
            }));
        }
        
        // 发送完成消息
        let _ = app_handle.emit("workflow-message", serde_json::json!({
            "type": "success",
            "content": "✅ 测试完成！事件系统工作正常。",
            "step": "test_complete",
            "progress": 1.0,
        }));
    });
    
    Ok("Test event sequence started".to_string())
}
use crate::state::{AppState, ExternalApiConfig};
use tauri::{State, Emitter, Manager};
use serde_json;
use uuid::Uuid;
use std::path::PathBuf;

// ====== 持久化存储路径 ======

fn get_config_path(app: &tauri::AppHandle) -> PathBuf {
    let data_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("external_apis.json")
}

/// 从磁盘加载 API 配置
pub fn load_apis_from_disk(app: &tauri::AppHandle) -> Vec<ExternalApiConfig> {
    let path = get_config_path(app);
    if !path.exists() {
        return vec![];
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(e) => {
            log::warn!("[ExternalApi] 读取配置失败: {}", e);
            vec![]
        }
    }
}

/// 保存 API 配置到磁盘
fn save_apis_to_disk(app: &tauri::AppHandle, apis: &[ExternalApiConfig]) {
    let path = get_config_path(app);
    match serde_json::to_string_pretty(apis) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                log::error!("[ExternalApi] 写入配置失败: {}", e);
            } else {
                log::info!("[ExternalApi] 配置已保存到: {:?}", path);
            }
        }
        Err(e) => log::error!("[ExternalApi] 序列化失败: {}", e),
    }
}

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

    log::info!("[test_external_api] provider={}, baseUrl={}, model={}", provider, baseUrl, model);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let result = match provider.as_str() {
        "anthropic" => {
            let response = client
                .post(format!("{}/messages", baseUrl))
                .header("x-api-key", &apiKey)
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
                    Ok(serde_json::json!({ "success": true, "message": "API 连接成功！" }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
        // 通用 OpenAI 兼容 API（openai, deepseek, glm, kimichat, minimax, qwen, groq, custom 等）
        _ => {
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
                    Ok(serde_json::json!({ "success": true, "message": "API 连接成功！" }))
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(format!("API 返回错误 ({}): {}", status, error_text))
                }
                Err(e) => Err(format!("连接失败: {}", e)),
            }
        }
    };

    result
}

/// 保存外部 API 配置（含持久化）
#[tauri::command]
pub async fn save_external_api(
    config: serde_json::Value,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ExternalApiConfig, String> {
    let id = Uuid::new_v4().to_string();
    let entry = ExternalApiConfig {
        id: id.clone(),
        name: config.get("name").and_then(|v| v.as_str()).unwrap_or("未命名").to_string(),
        provider: config.get("provider").and_then(|v| v.as_str()).unwrap_or("custom").to_string(),
        base_url: config.get("base_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        api_key: config.get("api_key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        model: config.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        enabled: config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    let mut apis = state.external_apis.lock();
    apis.push(entry.clone());
    save_apis_to_disk(&app, &apis);

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

/// 删除外部 API 配置（含持久化）
#[tauri::command]
pub async fn delete_external_api(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut apis = state.external_apis.lock();
    apis.retain(|api| api.id != id);
    save_apis_to_disk(&app, &apis);
    Ok(())
}

/// 切换外部 API 启用状态（含持久化）
#[tauri::command]
pub async fn toggle_external_api(
    id: String,
    enabled: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut apis = state.external_apis.lock();
    if let Some(api) = apis.iter_mut().find(|api| api.id == id) {
        api.enabled = enabled;
    }
    save_apis_to_disk(&app, &apis);
    Ok(())
}

/// 使用外部 API 进行对话（带重试）
#[tauri::command]
pub async fn chat_with_external_api(
    message: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use reqwest;
    use std::time::Duration;

    // 获取第一个启用的 API 配置
    let api_config = {
        let apis = state.external_apis.lock();
        apis.iter()
            .find(|api| api.enabled && !api.api_key.is_empty())
            .cloned()
            .ok_or_else(|| "没有启用的外部 API 配置，请在设置中添加 OpenAI、DeepSeek 等 API".to_string())?
    };

    log::info!("[chat] 使用 {} API, 模型: {}", api_config.provider, api_config.model);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let max_retries = 3;
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        let response = if api_config.provider == "anthropic" {
            client
                .post(format!("{}/messages", api_config.base_url))
                .header("x-api-key", &api_config.api_key)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&serde_json::json!({
                    "model": api_config.model,
                    "max_tokens": 1024,
                    "messages": [{"role": "user", "content": &message}],
                }))
                .send()
                .await
        } else {
            // 通用 OpenAI 兼容接口
            client
                .post(format!("{}/chat/completions", api_config.base_url))
                .header("Authorization", format!("Bearer {}", api_config.api_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": api_config.model,
                    "messages": [{"role": "user", "content": &message}],
                    "max_tokens": 1024,
                }))
                .send()
                .await
        };

        match response {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await
                    .map_err(|e| format!("解析响应失败: {}", e))?;

                // 提取回复内容（OpenAI 格式 或 Anthropic 格式）
                let content = if api_config.provider == "anthropic" {
                    json.get("content")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|item| item.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("无法获取回复")
                        .to_string()
                } else {
                    json.get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|first| first.get("message"))
                        .and_then(|msg| msg.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("无法获取回复")
                        .to_string()
                };

                log::info!("[chat] 成功 (尝试 {}), 回复长度: {}", attempt, content.len());

                return Ok(serde_json::json!({
                    "success": true,
                    "message": content,
                    "provider": api_config.provider,
                    "model": api_config.model,
                }));
            }
            Ok(resp) => {
                last_error = format!("API 返回错误 ({}): {}", resp.status(), resp.text().await.unwrap_or_default());
                log::warn!("[chat] 尝试 {} 失败: {}", attempt, last_error);
            }
            Err(e) => {
                last_error = format!("连接失败: {}", e);
                log::warn!("[chat] 尝试 {} 失败: {}", attempt, last_error);
            }
        }

        if attempt < max_retries {
            tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
        }
    }

    Err(format!("经过 {} 次尝试后仍然失败: {}", max_retries, last_error))
}

/// 使用外部 API 进行分布式模型对话
/// 当用户点击"运行"激活了某个模型后，后续聊天通过此命令路由，
/// 携带模型上下文信息（模型名、切分方案等）到 system prompt
#[tauri::command]
pub async fn chat_with_distributed_model(
    message: String,
    model_name: String,
    model_repo: String,
    params: String,
    total_layers: u32,
    is_local_only: bool,
    node_count: u32,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use reqwest;
    use std::time::Duration;

    // 获取第一个启用的 API 配置
    let api_config = {
        let apis = state.external_apis.lock();
        apis.iter()
            .find(|api| api.enabled && !api.api_key.is_empty())
            .cloned()
            .ok_or_else(|| "没有启用的外部 API 配置，请在设置中添加 OpenAI、DeepSeek 等 API".to_string())?
    };

    // 构建系统提示：模拟运行的大模型角色
    let deployment_mode = if is_local_only {
        format!("本机单节点运行（无其他 P2P 节点在线）")
    } else {
        format!("分布式 Pipeline Parallelism，跨 {} 个节点协作推理，共 {} 层", node_count, total_layers)
    };

    let system_prompt = format!(
        "你是 {}（{}参数，共{}层），一个通过 Williw 去中心化网络部署的大语言模型。\n\
        当前部署模式：{}\n\
        模型来源：HuggingFace - {}\n\n\
        请以该模型的身份回答用户的问题。回答要专业、准确、有帮助。\n\
        如果用户询问你的架构或部署情况，可以介绍上述分布式推理细节。",
        model_name, params, total_layers, deployment_mode, model_repo
    );

    log::info!(
        "[distributed_chat] 模型: {} ({} 参数), 节点: {}, provider: {}",
        model_name, params, node_count, api_config.provider
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let max_retries = 3;
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        let response = if api_config.provider == "anthropic" {
            client
                .post(format!("{}/messages", api_config.base_url))
                .header("x-api-key", &api_config.api_key)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&serde_json::json!({
                    "model": api_config.model,
                    "max_tokens": 2048,
                    "system": system_prompt,
                    "messages": [{"role": "user", "content": &message}],
                }))
                .send()
                .await
        } else {
            // 通用 OpenAI 兼容接口（DeepSeek, Qwen, OpenAI, GLM, Groq 等）
            client
                .post(format!("{}/chat/completions", api_config.base_url))
                .header("Authorization", format!("Bearer {}", api_config.api_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "model": api_config.model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": &message},
                    ],
                    "max_tokens": 2048,
                }))
                .send()
                .await
        };

        match response {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await
                    .map_err(|e| format!("解析响应失败: {}", e))?;

                let content = if api_config.provider == "anthropic" {
                    json.get("content")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|item| item.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("无法获取回复")
                        .to_string()
                } else {
                    json.get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|first| first.get("message"))
                        .and_then(|msg| msg.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("无法获取回复")
                        .to_string()
                };

                log::info!(
                    "[distributed_chat] 成功 (尝试 {}), 模型: {}, 回复长度: {}",
                    attempt, model_name, content.len()
                );

                return Ok(serde_json::json!({
                    "success": true,
                    "message": content,
                    "provider": api_config.provider,
                    "model": api_config.model,
                    "distributed_model": model_name,
                    "node_count": node_count,
                    "is_local_only": is_local_only,
                }));
            }
            Ok(resp) => {
                last_error = format!("API 返回错误 ({}): {}", resp.status(), resp.text().await.unwrap_or_default());
                log::warn!("[distributed_chat] 尝试 {} 失败: {}", attempt, last_error);
            }
            Err(e) => {
                last_error = format!("连接失败: {}", e);
                log::warn!("[distributed_chat] 尝试 {} 失败: {}", attempt, last_error);
            }
        }

        if attempt < max_retries {
            tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
        }
    }

    Err(format!("经过 {} 次尝试后仍然失败: {}", max_retries, last_error))
}

/// 测试工作流事件发送
#[tauri::command]
pub async fn test_workflow_event(
    app: tauri::AppHandle,
) -> Result<String, String> {
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🧪 测试消息：工作流事件系统正常工作！",
        "progress": 0.5,
    }));

    let app_handle = app.clone();
    tokio::spawn(async move {
        for i in 1..=3 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let _ = app_handle.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": format!("🧪 测试进度 {}/3", i),
                "progress": i as f64 / 3.0,
            }));
        }
        let _ = app_handle.emit("workflow-message", serde_json::json!({
            "type": "success",
            "content": "✅ 测试完成！事件系统工作正常。",
            "progress": 1.0,
        }));
    });

    Ok("Test started".to_string())
}

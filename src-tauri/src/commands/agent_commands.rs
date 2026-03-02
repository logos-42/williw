/// AI Agent 自动配置本地推理环境
///
/// 当用户点击「运行」时，Williw 启动一个 AI 代理循环：
/// - AI（外部 LLM）通过 function calling 决定调用哪些工具
/// - Williw 执行工具并返回结果
/// - 循环直到 AI 配置完成本地推理环境
/// - 每一步都通过 workflow-message 事件显示给用户
///
/// ## 模块化设计（人月神话原则）
/// 本文件仅作为 Tauri 命令入口点，具体功能已拆分到：
/// - [`agent_tools::definitions`]: 工具定义
/// - [`agent_tools::executors`]: 工具执行器
/// - [`agent::setup`]: 设置流程辅助函数

use crate::state::AppState;
use crate::commands::agent_tools::definitions;
use crate::commands::agent_tools::executors::{system, shell, http, filesystem, network, model};
use crate::commands::agent::setup::{self, parse_llm_response};
use crate::commands::agent::chat::quick_start_local_inference;
use tauri::{State, Emitter};
use serde_json;

/// AI Agent 自动配置本地推理环境
///
/// 使用 function calling 让 LLM 自主决定：
/// 1. 检查系统（check_system）
/// 2. 安装/配置工具（run_shell_command）
/// 3. 验证服务（check_http_endpoint）
/// 4. 宣布完成（finish_setup）
#[tauri::command]
pub async fn run_ai_agent_setup(
    user_model_hint: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    use reqwest;

    // ====== 优化：优先检查本地 Ollama 是否已有可用模型 ======
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🔍 检查本地 Ollama 状态...",
    }));

    // 快速检测本地 Ollama
    let local_check = quick_start_local_inference().await;
    if let Ok(check_result) = local_check {
        if check_result.get("found").and_then(|v| v.as_bool()).unwrap_or(false)
            && check_result.get("has_models").and_then(|v| v.as_bool()).unwrap_or(false) {
            let models = check_result.get("all_models")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let best_model = check_result.get("model_name")
                .and_then(|v| v.as_str())
                .unwrap_or("qwen2.5:1.5b");
            let endpoint = check_result.get("inference_endpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("http://localhost:11434/v1");

            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "success",
                "content": format!("✅ 发现本地已有可用的 Ollama 推理服务！\n\n已安装模型：{}\n自动选择：{}\n端点：{}\n\n直接使用现有模型，无需重新配置 👇", models.join(", "), best_model, endpoint),
            }));

            return Ok(serde_json::json!({
                "success": true,
                "inference_endpoint": endpoint,
                "model_name": best_model,
                "summary": format!("使用本地已有模型：{}（共 {} 个模型）", best_model, models.len()),
                "using_local_model": true,
                "local_existing": true,
            }));
        }
    }

    // ====== 本地没有可用模型，继续 AI 代理流程 ======
    let api_config = {
        let apis = state.external_apis.lock();
        apis.iter()
            .find(|api| api.enabled && !api.api_key.is_empty())
            .cloned()
            .ok_or_else(|| "需要先配置外部 API（如 DeepSeek、OpenAI）才能运行 AI 代理".to_string())?
    };

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🤖 AI 代理启动\n\n本地未检测到可用模型，将自动配置...\n\n用户请求运行：{}", user_model_hint),
    }));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    // 收集初始系统信息
    let system_info = system::check_system();

    let system_prompt = format!(
        "你是 Williw 的 AI 部署代理。你的任务是在用户的机器上配置一个可以运行的本地 AI 推理服务。\n\
        \n\
        用户希望运行：{}\n\
        当前系统信息:\n{}\n\
        \n\
        你需要：\n\
        1. 分析系统信息，了解机器能力\n\
        2. 选择最适合该硬件的模型（要保证能跑起来）\n\
        3. 安装必要的软件（推荐使用 Ollama，因为它最简单可靠）\n\
        4. 拉取并启动模型\n\
        5. 验证服务正常运行后调用 finish_setup\n\
        \n\
        重要原则：\n\
        - 优先使用 Ollama（最简单，支持 macOS Metal/GPU，API 兼容 OpenAI）\n\
        - macOS 上 Ollama 安装：curl -fsSL https://ollama.com/install.sh | sh\n\
        - 或者：brew install ollama\n\
        - 根据 RAM 选择模型：4-8GB 选 qwen2.5:0.5b，8-16GB 选 qwen2.5:1.5b，16GB+ 选 qwen2.5:3b 或 llama3.2:3b\n\
        - 如果 check_system 结果中有 ollama_bin_path 字段，说明 Ollama 已安装但不在 PATH 中；\n\
          此时所有 ollama 命令请使用该完整路径，例如 '<ollama_bin_path> pull qwen2.5:1.5b'\n\
          或者使用 'PATH=<ollama_dir>:$PATH ollama pull ...' 的方式\n\
        - 如果 ollama_models 字段已有模型，必须立即使用现有模型，直接调用 finish_setup！
          不要重新拉取任何模型！不要执行任何安装命令！\n\
        - 如果机器确实不适合运行任何模型，调用 report_failure\n\
        - 每次只调用一个工具，等待结果后再决定下一步\n\
        - 用中文解释你的每个决定",
        user_model_hint,
        serde_json::to_string_pretty(&system_info).unwrap_or_default()
    );

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("📊 系统信息收集完成\n\nOS: {} | RAM: {}GB | CPU: {} 核\nGPU: {}\n已安装：{}",
            system_info.get("os").and_then(|v| v.as_str()).unwrap_or("unknown"),
            system_info.get("ram_gb").and_then(|v| v.as_u64()).unwrap_or(0),
            system_info.get("cpu_cores").and_then(|v| v.as_u64()).unwrap_or(0),
            system_info.get("apple_gpu").and_then(|v| v.as_str()).unwrap_or(
                system_info.get("gpu").and_then(|v| v.as_str()).unwrap_or("未检测到专用 GPU")
            ),
            {
                let cmds = system_info.get("commands").and_then(|v| v.as_object());
                cmds.map(|m| m.iter().filter(|(_, v)| v.as_bool().unwrap_or(false)).map(|(k, _)| k.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default()
            }
        ),
    }));

    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("请开始配置本地推理环境。\n\n【关键】如果系统已有 ollama 模型，直接使用现有模型并调用 finish_setup，不要重新安装或下载任何模型！\n\n用户期望：{}", user_model_hint)
        })
    ];

    let tools = definitions::get_tool_definitions();
    let max_turns = 20;

    // Agent 循环
    for turn in 0..max_turns {
        log::info!("[Agent] 第 {} 轮对话", turn + 1);

        let request_body = if api_config.provider == "anthropic" {
            serde_json::json!({
                "model": api_config.model,
                "max_tokens": 2048,
                "system": messages[0].get("content").and_then(|v| v.as_str()).unwrap_or(""),
                "tools": tools,
                "messages": &messages[1..],
            })
        } else {
            serde_json::json!({
                "model": api_config.model,
                "messages": messages,
                "tools": tools,
                "tool_choice": "auto",
                "max_tokens": 2048,
            })
        };

        let api_url = if api_config.provider == "anthropic" {
            format!("{}/messages", api_config.base_url)
        } else {
            format!("{}/chat/completions", api_config.base_url)
        };

        let response = if api_config.provider == "anthropic" {
            client.post(&api_url)
                .header("x-api-key", &api_config.api_key)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .header("anthropic-beta", "tools-2024-04-04")
                .json(&request_body)
                .send()
                .await
        } else {
            client.post(&api_url)
                .header("Authorization", format!("Bearer {}", api_config.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
        };

        let resp_json: serde_json::Value = match response {
            Ok(r) if r.status().is_success() => {
                r.json().await.map_err(|e| format!("解析响应失败：{}", e))?
            }
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                return Err(format!("API 错误 {}: {}", status, text));
            }
            Err(e) => return Err(format!("API 请求失败：{}", e)),
        };

        let (text_content, tool_calls) = parse_llm_response(&resp_json, &api_config.provider);

        if let Some(text) = &text_content {
            if !text.trim().is_empty() {
                let _ = app.emit("workflow-message", serde_json::json!({
                    "type": "info",
                    "content": format!("💭 AI 思考：{}", text),
                }));
            }
        }

        if tool_calls.is_empty() {
            let final_msg = text_content.unwrap_or_else(|| "配置过程结束".to_string());
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "warning",
                "content": format!("⚠️ AI 未调用 finish_setup 就结束了:\n{}", final_msg),
            }));
            return Err(format!("AI 代理未完成配置：{}", final_msg));
        }

        if api_config.provider == "anthropic" {
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": resp_json.get("content").cloned().unwrap_or(serde_json::json!([]))
            }));
        } else {
            let assistant_msg = resp_json
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|first| first.get("message"))
                .cloned()
                .unwrap_or(serde_json::json!({"role": "assistant"}));
            messages.push(assistant_msg);
        }

        let mut tool_results: Vec<serde_json::Value> = vec![];
        let mut setup_result: Option<serde_json::Value> = None;

        for tool_call in &tool_calls {
            let tool_name = tool_call.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = tool_call.get("arguments")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            let tool_id = tool_call.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

            log::info!("[Agent] 工具调用：{} {:?}", tool_name, tool_args);

            let tool_result = match tool_name {
                "check_system" => {
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": "🔍 AI 正在检查系统信息...",
                    }));
                    system::check_system()
                }
                "run_shell_command" => {
                    let command = tool_args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    let timeout = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(30);
                    let background = tool_args.get("background").and_then(|v| v.as_bool()).unwrap_or(false);
                    shell::run_shell_command(command, timeout, background, &app).await
                }
                "check_http_endpoint" => {
                    let url = tool_args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("🌐 检查服务：{}", url),
                    }));
                    http::check_http_endpoint(url).await
                }
                "finish_setup" => {
                    let endpoint = tool_args.get("inference_endpoint").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let model_name = tool_args.get("model_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let summary = tool_args.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "success",
                        "content": format!("✅ 本地推理环境就绪！\n\n{}\n\n模型：{}\n端点：{}", summary, model_name, endpoint),
                    }));

                    setup_result = Some(serde_json::json!({
                        "success": true,
                        "inference_endpoint": endpoint,
                        "model_name": model_name,
                        "summary": summary,
                        "using_local_model": false,
                    }));

                    serde_json::json!({"status": "setup_complete"})
                }
                "report_failure" => {
                    let reason = tool_args.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let suggestion = tool_args.get("suggestion").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "error",
                        "content": format!("❌ 配置失败\n\n原因：{}\n\n建议：{}", reason, suggestion),
                    }));

                    return Err(format!("配置失败：{} 建议：{}", reason, suggestion));
                }
                "download_model" => {
                    let source = tool_args.get("source").and_then(|v| v.as_str()).unwrap_or("ollama");
                    let mdl = tool_args.get("model").and_then(|v| v.as_str()).unwrap_or("");
                    let cache_dir = tool_args.get("cache_dir").and_then(|v| v.as_str());
                    let timeout = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(300);

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📥 下载模型：{} ({})", mdl, source),
                    }));

                    model::download_model(source, mdl, cache_dir, timeout, &app).await
                }
                "start_inference_server" => {
                    let server_type = tool_args.get("server_type").and_then(|v| v.as_str()).unwrap_or("ollama");
                    let mdl = tool_args.get("model").and_then(|v| v.as_str()).unwrap_or("");
                    let port = tool_args.get("port").and_then(|v| v.as_u64()).unwrap_or(11434) as u16;
                    let gpu_layers = tool_args.get("gpu_layers").and_then(|v| v.as_i64()).map(|v| v as i32);
                    let background = tool_args.get("background").and_then(|v| v.as_bool()).unwrap_or(true);

                    model::start_inference_server(server_type, mdl, port, gpu_layers, background, &app).await
                }
                "wait_for_condition" => {
                    let target = tool_args.get("target").and_then(|v| v.as_str()).unwrap_or("");
                    let target_type = tool_args.get("target_type").and_then(|v| v.as_str()).unwrap_or("http");
                    let expected = tool_args.get("expected").and_then(|v| v.as_str()).unwrap_or("");
                    let max_attempts = tool_args.get("max_attempts").and_then(|v| v.as_u64()).unwrap_or(30) as u32;
                    let interval_secs = tool_args.get("interval_seconds").and_then(|v| v.as_u64()).unwrap_or(2);
                    let timeout_secs = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(60);

                    http::wait_for_condition(target, target_type, expected, max_attempts, interval_secs, timeout_secs).await
                }
                "kill_process" => {
                    let process_name = tool_args.get("process_name").and_then(|v| v.as_str()).unwrap_or("");
                    let force = tool_args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("🔪 终止进程：{}", process_name),
                    }));

                    network::kill_process(process_name, force).await
                }
                "write_file" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let content = tool_args.get("content").and_then(|v| v.as_str()).unwrap_or("");

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📝 写文件：{}", path),
                    }));

                    filesystem::write_file(path, content).await
                }
                "read_file" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📄 读文件：{}", path),
                    }));

                    filesystem::read_file(path).await
                }
                "file_exists" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    filesystem::file_exists(path)
                }
                "list_directory" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let include_hidden = tool_args.get("include_hidden").and_then(|v| v.as_bool()).unwrap_or(false);

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📂 列出目录：{}", path),
                    }));

                    filesystem::list_directory(path, include_hidden).await
                }
                "run_command_with_retry" => {
                    let command = tool_args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    let max_retries = tool_args.get("max_retries").and_then(|v| v.as_u64()).unwrap_or(3) as u32;
                    let retry_interval = tool_args.get("retry_interval_seconds").and_then(|v| v.as_u64()).unwrap_or(5);
                    let timeout = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(30);

                    shell::run_command_with_retry(command, max_retries, retry_interval, timeout, &app).await
                }
                "get_ollama_models" => {
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": "📦 获取 Ollama 模型列表...",
                    }));

                    model::get_ollama_models()
                }
                _ => {
                    serde_json::json!({"error": format!("未知工具：{}", tool_name)})
                }
            };

            tool_results.push(serde_json::json!({
                "tool_call_id": tool_id,
                "name": tool_name,
                "result": tool_result,
            }));

            if let Some(result) = setup_result {
                return Ok(result);
            }
        }

        if api_config.provider == "anthropic" {
            let tool_result_blocks: Vec<serde_json::Value> = tool_results.iter().map(|r| {
                serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": r.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "content": serde_json::to_string(r.get("result").unwrap_or(&serde_json::json!({}))).unwrap_or_default()
                })
            }).collect();
            messages.push(serde_json::json!({
                "role": "user",
                "content": tool_result_blocks
            }));
        } else {
            for r in &tool_results {
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": r.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "content": serde_json::to_string(r.get("result").unwrap_or(&serde_json::json!({}))).unwrap_or_default()
                }));
            }
        }
    }

    Err(format!("AI 代理在 {} 轮对话后仍未完成配置", max_turns))
}

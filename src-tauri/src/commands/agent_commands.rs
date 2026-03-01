/// AI Agent Tool Use 模块
/// 
/// 当用户点击「运行」时，Williw 启动一个 AI 代理循环：
/// - AI（外部 LLM）通过 function calling 决定调用哪些工具
/// - Williw 执行工具并返回结果
/// - 循环直到 AI 配置完成本地推理环境
/// - 每一步都通过 workflow-message 事件显示给用户

use crate::state::AppState;
use tauri::{State, Emitter};
use serde_json;
use std::process::Command;

// ====== 工具定义（给 LLM 的 JSON schema）======

fn get_tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "check_system",
                "description": "检查当前机器的硬件信息和已安装的软件。返回 OS、RAM、CPU、GPU、以及关键命令是否存在（ollama, python3, pip, brew, curl）。",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_shell_command",
                "description": "在用户机器上执行 shell 命令。用于安装软件、启动服务、下载模型等操作。注意：命令会真实执行，请谨慎选择。对于长时间运行的命令（如 ollama pull），会等待最多 300 秒。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "要执行的命令，例如 'ollama pull qwen2.5:0.5b'"
                        },
                        "timeout_seconds": {
                            "type": "integer",
                            "description": "超时时间（秒），默认 30，下载操作可设为 300",
                            "default": 30
                        },
                        "background": {
                            "type": "boolean",
                            "description": "是否后台运行（不等待结束），用于启动服务",
                            "default": false
                        }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "check_http_endpoint",
                "description": "检查某个 HTTP 端点是否可达（服务是否已启动）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "要检查的 URL，例如 'http://localhost:11434'"
                        }
                    },
                    "required": ["url"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "finish_setup",
                "description": "当本地推理环境已就绪时调用此工具，告知 Williw 推理端点和模型名称。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "inference_endpoint": {
                            "type": "string",
                            "description": "推理服务的 base URL，例如 'http://localhost:11434/v1'"
                        },
                        "model_name": {
                            "type": "string",
                            "description": "已加载的模型名称，例如 'qwen2.5:1.5b'"
                        },
                        "summary": {
                            "type": "string",
                            "description": "用中文向用户解释配置了什么、为什么选这个模型"
                        }
                    },
                    "required": ["inference_endpoint", "model_name", "summary"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "report_failure",
                "description": "当无法完成配置时调用，说明原因和建议。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "reason": {
                            "type": "string",
                            "description": "失败原因（中文）"
                        },
                        "suggestion": {
                            "type": "string",
                            "description": "给用户的建议（中文）"
                        }
                    },
                    "required": ["reason", "suggestion"]
                }
            }
        }
    ])
}

// ====== 工具执行器 ======

fn tool_check_system() -> serde_json::Value {
    let mut result = serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "commands": {}
    });

    // 检查内存
    let mem_output = Command::new("sh")
        .arg("-c")
        .arg("sysctl -n hw.memsize 2>/dev/null || free -b 2>/dev/null | awk '/Mem:/{print $2}' || echo 0")
        .output();
    if let Ok(output) = mem_output {
        let mem_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let mem_bytes: u64 = mem_str.parse().unwrap_or(0);
        let mem_gb = mem_bytes / (1024 * 1024 * 1024);
        result["ram_gb"] = serde_json::json!(mem_gb);
    }

    // 检查 CPU 核心数
    let cpu_output = Command::new("sh")
        .arg("-c")
        .arg("nproc 2>/dev/null || sysctl -n hw.logicalcpu 2>/dev/null || echo 4")
        .output();
    if let Ok(output) = cpu_output {
        let cpu_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let cpu_count: u32 = cpu_str.parse().unwrap_or(4);
        result["cpu_cores"] = serde_json::json!(cpu_count);
    }

    // 检查 GPU
    let nvidia_output = Command::new("sh")
        .arg("-c")
        .arg("nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>/dev/null | head -1")
        .output();
    if let Ok(output) = nvidia_output {
        let gpu_info = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !gpu_info.is_empty() {
            result["gpu"] = serde_json::json!(gpu_info);
            result["has_nvidia_gpu"] = serde_json::json!(true);
        }
    }

    // 检查 Apple Silicon
    let apple_gpu_output = Command::new("sh")
        .arg("-c")
        .arg("system_profiler SPDisplaysDataType 2>/dev/null | grep 'Chipset Model' | head -1")
        .output();
    if let Ok(output) = apple_gpu_output {
        let apple_gpu = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !apple_gpu.is_empty() {
            result["apple_gpu"] = serde_json::json!(apple_gpu);
        }
    }

    // macOS Ollama 的常见安装路径（安装到 /Applications 但不在 PATH 中）
    let ollama_extra_paths = vec![
        "/Applications/Ollama.app/Contents/Resources/ollama",
        "/usr/local/bin/ollama",
        "/opt/homebrew/bin/ollama",
    ];

    // 检查关键命令是否存在（同时检查 PATH 和已知固定路径）
    let commands_to_check = vec!["ollama", "python3", "python", "pip3", "brew", "curl", "docker"];
    let mut commands_obj = serde_json::json!({});
    for cmd in commands_to_check {
        let in_path = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {} 2>/dev/null", cmd))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // 对 ollama 额外检查已知固定路径
        let exists = if cmd == "ollama" && !in_path {
            ollama_extra_paths.iter().any(|p| std::path::Path::new(p).exists())
        } else {
            in_path
        };
        commands_obj[cmd] = serde_json::json!(exists);
    }
    result["commands"] = commands_obj;

    // 找出 ollama 的实际路径（供 AI 使用全路径调用）
    let ollama_bin = {
        let in_path = Command::new("sh")
            .arg("-c")
            .arg("command -v ollama 2>/dev/null")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        if let Some(path) = in_path.filter(|p| !p.is_empty()) {
            path
        } else {
            ollama_extra_paths.iter()
                .find(|p| std::path::Path::new(p).exists())
                .map(|p| p.to_string())
                .unwrap_or_default()
        }
    };
    if !ollama_bin.is_empty() {
        result["ollama_bin_path"] = serde_json::json!(ollama_bin.clone());
    }

    // 检查 ollama 已有的模型（使用实际路径）
    let ollama_cmd = if ollama_bin.is_empty() { "ollama".to_string() } else { ollama_bin };
    let ollama_models = Command::new("sh")
        .arg("-c")
        .arg(format!("{} list 2>/dev/null || echo 'ollama not found'", ollama_cmd))
        .output();
    if let Ok(output) = ollama_models {
        let models_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        result["ollama_models"] = serde_json::json!(models_str);
    }

    result
}

async fn tool_run_shell_command(
    command: &str,
    timeout_secs: u64,
    background: bool,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    // 显示即将执行的命令
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("🔧 执行命令: `{}`", command),
    }));

    log::info!("[Agent] 执行命令: {}", command);

    // 构建增强的 PATH，包含 Ollama 的已知安装目录
    let ollama_dir = "/Applications/Ollama.app/Contents/Resources";
    let current_path = std::env::var("PATH").unwrap_or_default();
    let enhanced_path = if current_path.contains(ollama_dir) {
        current_path.clone()
    } else {
        format!("{}:{}", ollama_dir, current_path)
    };

    if background {
        // 后台运行，不等待
        let result = Command::new("sh")
            .env("PATH", &enhanced_path)
            .arg("-c")
            .arg(format!("{} &", command))
            .spawn();
        
        match result {
            Ok(_) => {
                // 等一下让服务启动
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                serde_json::json!({
                    "success": true,
                    "stdout": format!("命令已在后台启动: {}", command),
                    "stderr": ""
                })
            }
            Err(e) => serde_json::json!({
                "success": false,
                "error": format!("启动失败: {}", e)
            })
        }
    } else {
        // 前台运行，等待完成
        let timeout = tokio::time::Duration::from_secs(timeout_secs);
        let command_owned = command.to_string();
        let enhanced_path_owned = enhanced_path.clone();
        
        let result = tokio::time::timeout(timeout, async move {
            tokio::task::spawn_blocking(move || {
                Command::new("sh")
                    .env("PATH", &enhanced_path_owned)
                    .arg("-c")
                    .arg(&command_owned)
                    .output()
            }).await
        }).await;

        match result {
            Ok(Ok(Ok(output))) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let success = output.status.success();
                
                // 截断太长的输出
                let stdout_trimmed = if stdout.len() > 2000 {
                    format!("{}...(truncated)", &stdout[..2000])
                } else {
                    stdout
                };
                
                serde_json::json!({
                    "success": success,
                    "exit_code": output.status.code(),
                    "stdout": stdout_trimmed,
                    "stderr": &stderr[..std::cmp::min(stderr.len(), 500)]
                })
            }
            Ok(Ok(Err(e))) => serde_json::json!({
                "success": false,
                "error": format!("执行错误: {}", e)
            }),
            Ok(Err(e)) => serde_json::json!({
                "success": false,
                "error": format!("任务错误: {}", e)
            }),
            Err(_) => serde_json::json!({
                "success": false,
                "error": format!("命令超时（{}秒）", timeout_secs)
            })
        }
    }
}

async fn tool_check_http_endpoint(url: &str) -> serde_json::Value {
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

// ====== 主 Agent 命令 ======

/// AI Agent 自动配置本地推理环境
/// 
/// 使用 function calling 让 LLM 自主决定：
/// 1. 检查系统（check_system）
/// 2. 安装/配置工具（run_shell_command）
/// 3. 验证服务（check_http_endpoint）
/// 4. 宣布完成（finish_setup）
#[tauri::command]
pub async fn run_ai_agent_setup(
    user_model_hint: String, // 用户选择的模型（提示用，AI 可能选不同的）
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    use reqwest;

    // ====== 优化：优先检查本地 Ollama 是否已有可用模型 ======
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": "🔍 检查本地 Ollama 状态...",
    }));

    // 快速检测本地 Ollama（复用 quick_start_local_inference 逻辑）
    let local_check = quick_start_local_inference().await;
    if let Ok(check_result) = local_check {
        if check_result.get("found").and_then(|v| v.as_bool()).unwrap_or(false)
            && check_result.get("has_models").and_then(|v| v.as_bool()).unwrap_or(false) {
            // 本地已有运行中的 Ollama 和模型，直接使用
            let models = check_result.get("all_models")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().ok()).collect::<Vec<_>>())
                .unwrap_or_default();
            let best_model = check_result.get("model_name")
                .and_then(|v| v.as_str())
                .unwrap_or("qwen2.5:1.5b");
            let endpoint = check_result.get("inference_endpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("http://localhost:11434/v1");

            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "success",
                "content": format!("✅ 发现本地已有可用的 Ollama 推理服务！\n\n已安装模型: {}\n自动选择: {}\n端点: {}\n\n直接使用现有模型，无需重新配置 👇", models.join(", "), best_model, endpoint),
            }));

            return Ok(serde_json::json!({
                "success": true,
                "inference_endpoint": endpoint,
                "model_name": best_model,
                "summary": format!("使用本地已有模型: {}（共 {} 个模型）", best_model, models.len()),
                "using_local_model": true,
                "local_existing": true,
            }));
        }
    }

    // ====== 本地没有可用模型，继续 AI 代理流程 ======
    
    // 获取外部 API 配置
    let api_config = {
        let apis = state.external_apis.lock();
        apis.iter()
            .find(|api| api.enabled && !api.api_key.is_empty())
            .cloned()
            .ok_or_else(|| "需要先配置外部 API（如 DeepSeek、OpenAI）才能运行 AI 代理".to_string())?
    };

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🤖 AI 代理启动\n\n本地未检测到可用模型，将自动配置...\n\n用户请求运行: {}", user_model_hint),
    }));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 收集初始系统信息
    let system_info = tool_check_system();

    let system_prompt = format!(
        "你是 Williw 的 AI 部署代理。你的任务是在用户的机器上配置一个可以运行的本地 AI 推理服务。\n\
        \n\
        用户希望运行: {}\n\
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
        - macOS 上 Ollama 安装: curl -fsSL https://ollama.com/install.sh | sh\n\
        - 或者: brew install ollama\n\
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
        "content": format!("📊 系统信息收集完成\n\nOS: {} | RAM: {}GB | CPU: {} 核\nGPU: {}\n已安装: {}",
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

    // 构建初始消息
    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("请开始配置本地推理环境。\n\n【关键】如果系统已有ollama模型，直接使用现有模型并调用 finish_setup，不要重新安装或下载任何模型！\n\n用户期望: {}", user_model_hint)
        })
    ];

    let tools = get_tool_definitions();
    let max_turns = 20;

    // Agent 循环
    for turn in 0..max_turns {
        log::info!("[Agent] 第 {} 轮对话", turn + 1);

        // 调用 LLM
        let request_body = if api_config.provider == "anthropic" {
            serde_json::json!({
                "model": api_config.model,
                "max_tokens": 2048,
                "system": messages[0].get("content").and_then(|v| v.as_str()).unwrap_or(""),
                "tools": tools,
                "messages": &messages[1..],
            })
        } else {
            // OpenAI 兼容格式（DeepSeek, Qwen, OpenAI, GLM 等）
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
                r.json().await.map_err(|e| format!("解析响应失败: {}", e))?
            }
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                return Err(format!("API 错误 {}: {}", status, text));
            }
            Err(e) => return Err(format!("API 请求失败: {}", e)),
        };

        // 解析 LLM 响应（处理 OpenAI 和 Anthropic 格式）
        let (text_content, tool_calls) = parse_llm_response(&resp_json, &api_config.provider);

        // 如果有文本，显示给用户
        if let Some(text) = &text_content {
            if !text.trim().is_empty() {
                let _ = app.emit("workflow-message", serde_json::json!({
                    "type": "info",
                    "content": format!("💭 AI 思考: {}", text),
                }));
            }
        }

        // 如果没有工具调用，且有文本内容，说明 AI 结束了但没调用 finish_setup
        if tool_calls.is_empty() {
            let final_msg = text_content.unwrap_or_else(|| "配置过程结束".to_string());
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "warning",
                "content": format!("⚠️ AI 未调用 finish_setup 就结束了:\n{}", final_msg),
            }));
            return Err(format!("AI 代理未完成配置: {}", final_msg));
        }

        // 将 AI 响应添加到消息历史
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

        // 执行工具调用
        let mut tool_results: Vec<serde_json::Value> = vec![];
        let mut setup_result: Option<serde_json::Value> = None;

        for tool_call in &tool_calls {
            let tool_name = tool_call.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = tool_call.get("arguments")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            let tool_id = tool_call.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

            log::info!("[Agent] 工具调用: {} {:?}", tool_name, tool_args);

            let tool_result = match tool_name {
                "check_system" => {
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": "🔍 AI 正在检查系统信息...",
                    }));
                    tool_check_system()
                }
                "run_shell_command" => {
                    let command = tool_args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    let timeout = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(30);
                    let background = tool_args.get("background").and_then(|v| v.as_bool()).unwrap_or(false);
                    tool_run_shell_command(command, timeout, background, &app).await
                }
                "check_http_endpoint" => {
                    let url = tool_args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("🌐 检查服务: {}", url),
                    }));
                    tool_check_http_endpoint(url).await
                }
                "finish_setup" => {
                    let endpoint = tool_args.get("inference_endpoint").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let model = tool_args.get("model_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let summary = tool_args.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "success",
                        "content": format!("✅ 本地推理环境就绪！\n\n{}\n\n模型: {}\n端点: {}", summary, model, endpoint),
                    }));

                    setup_result = Some(serde_json::json!({
                        "success": true,
                        "inference_endpoint": endpoint,
                        "model_name": model,
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
                        "content": format!("❌ 配置失败\n\n原因: {}\n\n建议: {}", reason, suggestion),
                    }));

                    return Err(format!("配置失败: {} 建议: {}", reason, suggestion));
                }
                _ => {
                    serde_json::json!({"error": format!("未知工具: {}", tool_name)})
                }
            };

            tool_results.push(serde_json::json!({
                "tool_call_id": tool_id,
                "name": tool_name,
                "result": tool_result,
            }));

            // 如果已经完成设置，返回
            if let Some(result) = setup_result {
                return Ok(result);
            }
        }

        // 将工具结果返回给 LLM
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
            // OpenAI 格式：每个工具结果作为独立 tool 消息
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

/// 解析 LLM 响应，提取文本和工具调用
fn parse_llm_response(resp: &serde_json::Value, provider: &str) -> (Option<String>, Vec<serde_json::Value>) {
    let mut text_parts: Vec<String> = vec![];
    let mut tool_calls: Vec<serde_json::Value> = vec![];

    if provider == "anthropic" {
        if let Some(content) = resp.get("content").and_then(|v| v.as_array()) {
            for block in content {
                match block.get("type").and_then(|v| v.as_str()) {
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                            text_parts.push(text.to_string());
                        }
                    }
                    Some("tool_use") => {
                        tool_calls.push(serde_json::json!({
                            "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                            "name": block.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                            "arguments": block.get("input").cloned().unwrap_or(serde_json::json!({}))
                        }));
                    }
                    _ => {}
                }
            }
        }
    } else {
        // OpenAI 格式
        if let Some(choices) = resp.get("choices").and_then(|v| v.as_array()) {
            if let Some(first) = choices.first() {
                let default_msg = serde_json::json!({});
                let msg = first.get("message").unwrap_or(&default_msg);

                if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        text_parts.push(content.to_string());
                    }
                }

                if let Some(calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                    for call in calls {
                        let args_str = call
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}");
                        let args: serde_json::Value = serde_json::from_str(args_str)
                            .unwrap_or(serde_json::json!({}));

                        tool_calls.push(serde_json::json!({
                            "id": call.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                            "name": call.get("function").and_then(|f| f.get("name")).and_then(|v| v.as_str()).unwrap_or(""),
                            "arguments": args
                        }));
                    }
                }
            }
        }
    }

    let text = if text_parts.is_empty() { None } else { Some(text_parts.join("\n")) };
    (text, tool_calls)
}

// ====== 快速启动：不需要 AI 代理，直接检测本地已有服务 ======

/// 找到 ollama 二进制文件路径
fn find_ollama_bin() -> Option<String> {
    let extra_paths = [
        "/Applications/Ollama.app/Contents/Resources/ollama",
        "/usr/local/bin/ollama",
        "/opt/homebrew/bin/ollama",
    ];
    // 先检查 PATH
    let in_path = Command::new("sh")
        .arg("-c")
        .arg("command -v ollama 2>/dev/null")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(p) = in_path {
        return Some(p);
    }
    extra_paths.iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|p| p.to_string())
}

/// 根据本机 RAM 从已有模型中选最合适的
fn select_best_model(models: &[String]) -> String {
    // 获取内存
    let ram_gb: u64 = Command::new("sh")
        .arg("-c")
        .arg("sysctl -n hw.memsize 2>/dev/null || free -b 2>/dev/null | awk '/Mem:/{print $2}' || echo 0")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
        .map(|b| b / (1024 * 1024 * 1024))
        .unwrap_or(8);

    // 优先顺序（越往前越好）
    let preference = if ram_gb >= 16 {
        vec!["qwen2.5:3b", "llama3.2:3b", "qwen2.5:1.5b", "qwen2.5:0.5b"]
    } else {
        vec!["qwen2.5:1.5b", "qwen2.5:0.5b", "qwen2.5:3b", "llama3.2:3b"]
    };

    for pref in preference {
        if let Some(m) = models.iter().find(|m| m.starts_with(pref)) {
            return m.clone();
        }
    }
    // 如果没有匹配偏好，返回第一个
    models.first().cloned().unwrap_or_else(|| "qwen2.5:1.5b".to_string())
}

/// 预热本地 Ollama 模型（将模型加载到内存，保持 keep_alive=-1 永不卸载）
/// 适合首次加载或模型已卸载的情况，会等待加载完成（最多 90 秒）
#[tauri::command]
pub async fn warmup_local_model(model_name: String) -> Result<serde_json::Value, String> {
    use reqwest;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90)) // 给足 90 秒等待 CPU 加载
        .build()
        .map_err(|e| e.to_string())?;

    log::info!("[Warmup] 开始预热模型: {}", model_name);

    // 调用 /api/generate 空 prompt，keep_alive=-1 让模型永久保持在内存
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
            // 即使不是 200 也返回 success，让用户继续尝试
            Ok(serde_json::json!({
                "success": false,
                "status": status,
                "message": format!("预热返回 {} (可能模型仍在加载)", status),
            }))
        }
        Err(e) => {
            log::warn!("[Warmup] 预热请求失败: {}", e);
            Err(format!("预热失败: {}", e))
        }
    }
}

/// 快速检测本地是否有可用的推理服务（无需 AI 代理）
/// 适合 Ollama 已安装并运行的情况，1-2 秒内返回
#[tauri::command]
pub async fn quick_start_local_inference() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    // 检查 Ollama 是否在运行
    let ollama_running = client.get("http://localhost:11434")
        .send()
        .await
        .map(|_| true)
        .unwrap_or(false);

    if !ollama_running {
        return Ok(serde_json::json!({
            "found": false,
            "message": "本地未检测到运行中的 Ollama 推理服务（localhost:11434）"
        }));
    }

    // 获取已安装的模型列表
    let ollama_bin = find_ollama_bin().unwrap_or_else(|| "ollama".to_string());
    let ollama_dir = "/Applications/Ollama.app/Contents/Resources";
    let current_path = std::env::var("PATH").unwrap_or_default();
    let enhanced_path = format!("{}:{}", ollama_dir, current_path);

    let models_output = Command::new("sh")
        .env("PATH", &enhanced_path)
        .arg("-c")
        .arg(format!("{} list 2>/dev/null", ollama_bin))
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // 解析模型列表（跳过表头行）
    let models: Vec<String> = models_output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.first().map(|s| s.to_string())
        })
        .filter(|s| !s.is_empty())
        .collect();

    if models.is_empty() {
        return Ok(serde_json::json!({
            "found": true,
            "has_models": false,
            "message": "Ollama 运行中，但没有已安装的模型。请先运行: ollama pull qwen2.5:1.5b"
        }));
    }

    let best_model = select_best_model(&models);

    Ok(serde_json::json!({
        "found": true,
        "has_models": true,
        "inference_endpoint": "http://localhost:11434/v1",
        "model_name": best_model,
        "all_models": models,
        "summary": format!(
            "检测到本地 Ollama 服务，已安装 {} 个模型，自动选择 {} 进行推理",
            models.len(), best_model
        )
    }))
}

/// 通过本地推理端点进行对话（Ollama / 任何 OpenAI 兼容服务）
#[tauri::command]
pub async fn chat_with_local_endpoint(
    message: String,
    endpoint: String,      // e.g. "http://localhost:11434/v1"
    model_name: String,    // e.g. "qwen2.5:1.5b"
    system_prompt: Option<String>,
) -> Result<serde_json::Value, String> {
    use reqwest;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

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

    log::info!("[LocalChat] 调用: {} 模型: {}", url, model_name);

    // 最多重试 6 次（CPU 模式冷启动可能需要 30-60 秒）
    // 每次间隔 10s，总等待最多 60s
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
                    // 502 = Ollama 模型正在加载（CPU 冷启动可能需要 30-60 秒）
                    log::warn!("[LocalChat] 502 Bad Gateway (attempt {}/6), 等待模型加载... ({} 秒后重试)", 
                        attempt + 1, 10);
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
                last_err = format!("请求失败: {}", e);
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

    let json: serde_json::Value = response.json().await
        .map_err(|e| format!("解析响应失败: {}", e))?;

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

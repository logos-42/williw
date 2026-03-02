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
use std::fs;
use tokio::fs as async_fs;
use std::path::Path;

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
        },
        {
            "type": "function",
            "function": {
                "name": "download_model",
                "description": "Download AI models from Ollama or HuggingFace. Supports both sources.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "enum": ["ollama", "huggingface"],
                            "description": "Model source"
                        },
                        "model": {
                            "type": "string",
                            "description": "Model name (e.g., qwen2.5:0.5b or meta-llama/Llama-3.2-1B)"
                        },
                        "cache_dir": {
                            "type": "string",
                            "description": "Optional cache directory"
                        },
                        "timeout_seconds": {
                            "type": "integer",
                            "default": 300
                        }
                    },
                    "required": ["source", "model"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "start_inference_server",
                "description": "Start a local inference server. Supports Ollama, llama.cpp server.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "server_type": {
                            "type": "string",
                            "enum": ["ollama", "llama.cpp"]
                        },
                        "model": {
                            "type": "string",
                            "description": "Model name or path"
                        },
                        "port": {
                            "type": "integer",
                            "default": 11434
                        },
                        "gpu_layers": {
                            "type": "integer",
                            "description": "GPU layers for llama.cpp (-1 for all)"
                        },
                        "background": {
                            "type": "boolean",
                            "default": true
                        }
                    },
                    "required": ["server_type", "model"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "wait_for_condition",
                "description": "Poll HTTP endpoint, command, or file until expected pattern matches.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "URL, command, or file path"
                        },
                        "target_type": {
                            "type": "string",
                            "enum": ["http", "command", "file"]
                        },
                        "expected": {
                            "type": "string",
                            "description": "Expected pattern (string or regex)"
                        },
                        "max_attempts": {
                            "type": "integer",
                            "default": 30
                        },
                        "interval_seconds": {
                            "type": "integer",
                            "default": 2
                        },
                        "timeout_seconds": {
                            "type": "integer",
                            "default": 60
                        }
                    },
                    "required": ["target", "target_type", "expected"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "kill_process",
                "description": "Terminate a running process by name.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "process_name": {
                            "type": "string",
                            "description": "Process name to kill"
                        },
                        "force": {
                            "type": "boolean",
                            "default": false
                        }
                    },
                    "required": ["process_name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Creates parent directories if needed.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read content from a file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "file_exists",
                "description": "Check if a file or directory exists.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to check"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_directory",
                "description": "List files and directories in a path.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path"
                        },
                        "include_hidden": {
                            "type": "boolean",
                            "default": false
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_command_with_retry",
                "description": "Execute a shell command with automatic retry on failure.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Command to execute"
                        },
                        "max_retries": {
                            "type": "integer",
                            "default": 3
                        },
                        "retry_interval_seconds": {
                            "type": "integer",
                            "default": 5
                        },
                        "timeout_seconds": {
                            "type": "integer",
                            "default": 30
                        }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_ollama_models",
                "description": "Get list of installed Ollama models and their status.",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
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

async fn tool_download_model(
    source: &str,
    model: &str,
    cache_dir: Option<&str>,
    timeout_secs: u64,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    log::info!("[Agent] 下载模型: source={}, model={}", source, model);
    
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("📥 开始下载模型: {} (来源: {})", model, source),
    }));

    match source {
        "ollama" => {
            let ollama_bin = find_ollama_bin().unwrap_or_else(|| "ollama".to_string());
            let command = format!("{} pull {}", ollama_bin, model);
            
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": format!("🔧 执行命令: {}", command),
            }));

            let timeout = tokio::time::Duration::from_secs(timeout_secs);
            let command_owned = command.clone();
            
            let result = tokio::time::timeout(timeout, async move {
                tokio::task::spawn_blocking(move || {
                    Command::new("sh")
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
                    
                    if success {
                        let _ = app.emit("workflow-message", serde_json::json!({
                            "type": "success",
                            "content": format!("✅ 模型下载成功: {}", model),
                        }));
                    }
                    
                    serde_json::json!({
                        "success": success,
                        "source": source,
                        "model": model,
                        "stdout": stdout,
                        "stderr": stderr,
                        "message": if success { format!("模型 {} 下载成功", model) } else { format!("下载失败: {}", stderr) }
                    })
                }
                Ok(Ok(Err(e))) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("执行错误: {}", e)
                }),
                Ok(Err(e)) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("任务错误: {}", e)
                }),
                Err(_) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("下载超时（{}秒）", timeout_secs)
                })
            }
        }
        "huggingface" => {
            let cache_arg = cache_dir.map(|d| format!("--cache-dir {}", d)).unwrap_or_default();
            let command = format!("python3 -c \"from huggingface_hub import snapshot_download; snapshot_download('{}' {})\"", model, cache_arg);
            
            let timeout = tokio::time::Duration::from_secs(timeout_secs);
            let command_owned = command.clone();
            
            let result = tokio::time::timeout(timeout, async move {
                tokio::task::spawn_blocking(move || {
                    Command::new("sh")
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
                    
                    let model_path = if success {
                        cache_dir.unwrap_or("~/.cache/huggingface").to_string()
                    } else {
                        String::new()
                    };
                    
                    serde_json::json!({
                        "success": success,
                        "source": source,
                        "model": model,
                        "model_path": model_path,
                        "stdout": stdout,
                        "stderr": stderr,
                        "message": if success { format!("模型 {} 下载成功", model) } else { format!("下载失败: {}", stderr) }
                    })
                }
                Ok(Ok(Err(e))) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("执行错误: {}", e)
                }),
                Ok(Err(e)) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("任务错误: {}", e)
                }),
                Err(_) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("下载超时（{}秒）", timeout_secs)
                })
            }
        }
        _ => serde_json::json!({
            "success": false,
            "error": format!("不支持的模型来源: {}", source)
        })
    }
}

async fn tool_start_inference_server(
    server_type: &str,
    model: &str,
    port: u16,
    gpu_layers: Option<i32>,
    background: bool,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    log::info!("[Agent] 启动推理服务器: type={}, model={}, port={}", server_type, model, port);

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("🚀 启动推理服务器: {} (模型: {}, 端口: {})", server_type, model, port),
    }));

    match server_type {
        "ollama" => {
            let command = if background {
                format!("ollama serve &")
            } else {
                "ollama serve".to_string()
            };

            let result = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .spawn();

            match result {
                Ok(child) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    let endpoint = format!("http://localhost:{}/v1", port);
                    serde_json::json!({
                        "success": true,
                        "server_type": server_type,
                        "model": model,
                        "endpoint": endpoint,
                        "pid": child.id(),
                        "message": format!("Ollama 服务已启动 (PID: {})", child.id())
                    })
                }
                Err(e) => serde_json::json!({
                    "success": false,
                    "error": format!("启动失败: {}", e)
                })
            }
        }
        "llama.cpp" => {
            let gpu_layers_arg = gpu_layers.map(|l| format!("--gpu-layers {}", l)).unwrap_or_default();
            let command = if background {
                format!("llama-server --model {} --port {} {} &", model, port, gpu_layers_arg)
            } else {
                format!("llama-server --model {} --port {} {}", model, port, gpu_layers_arg)
            };

            let result = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .spawn();

            match result {
                Ok(child) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    let endpoint = format!("http://localhost:{}/v1", port);
                    serde_json::json!({
                        "success": true,
                        "server_type": server_type,
                        "model": model,
                        "endpoint": endpoint,
                        "pid": child.id(),
                        "message": format!("llama.cpp 服务已启动 (PID: {})", child.id())
                    })
                }
                Err(e) => serde_json::json!({
                    "success": false,
                    "error": format!("启动失败: {}", e)
                })
            }
        }
        _ => serde_json::json!({
            "success": false,
            "error": format!("不支持的服务器类型: {}", server_type)
        })
    }
}

async fn tool_wait_for_condition(
    target: &str,
    target_type: &str,
    expected: &str,
    max_attempts: u32,
    interval_secs: u64,
    _timeout_secs: u64,
) -> serde_json::Value {
    log::info!("[Agent] 等待条件: target={}, type={}, expected={}", target, target_type, expected);

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

async fn tool_kill_process(process_name: &str, force: bool) -> serde_json::Value {
    log::info!("[Agent] 终止进程: name={}, force={}", process_name, force);

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
                    format!("终止失败: {}", stderr) 
                }
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "process_name": process_name,
            "error": format!("执行错误: {}", e)
        })
    }
}

async fn tool_write_file(path: &str, content: &str) -> serde_json::Value {
    log::info!("[Agent] 写文件: {}", path);

    let path_obj = Path::new(path);
    
    if let Some(parent) = path_obj.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return serde_json::json!({
                    "success": false,
                    "path": path,
                    "error": format!("创建父目录失败: {}", e)
                });
            }
        }
    }

    match fs::write(path, content) {
        Ok(_) => {
            let bytes_written = content.len();
            serde_json::json!({
                "success": true,
                "path": path,
                "bytes_written": bytes_written,
                "message": format!("成功写入 {} 字节", bytes_written)
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "path": path,
            "error": format!("写入失败: {}", e)
        })
    }
}

async fn tool_read_file(path: &str) -> serde_json::Value {
    log::info!("[Agent] 读文件: {}", path);

    match fs::read_to_string(path) {
        Ok(content) => {
            let size = content.len();
            serde_json::json!({
                "success": true,
                "path": path,
                "content": content,
                "size": size,
                "message": format!("成功读取 {} 字节", size)
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "path": path,
            "error": format!("读取失败: {}", e)
        })
    }
}

fn tool_file_exists(path: &str) -> serde_json::Value {
    log::info!("[Agent] 检查文件是否存在: {}", path);

    let path_obj = Path::new(path);
    let exists = path_obj.exists();
    let is_file = path_obj.is_file();
    let is_dir = path_obj.is_dir();

    serde_json::json!({
        "success": true,
        "path": path,
        "exists": exists,
        "is_file": is_file,
        "is_dir": is_dir,
        "message": if exists {
            if is_file { "是文件" } else if is_dir { "是目录" } else { "存在" }
        } else { "不存在" }
    })
}

async fn tool_list_directory(path: &str, include_hidden: bool) -> serde_json::Value {
    log::info!("[Agent] 列出目录: {}, include_hidden={}", path, include_hidden);

    let mut entries: Vec<serde_json::Value> = vec![];

    match fs::read_dir(path) {
        Ok(dir) => {
            for entry in dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                
                if !include_hidden && name.starts_with('.') {
                    continue;
                }

                let file_type = entry.file_type().ok();
                let is_file = file_type.map(|ft| ft.is_file()).unwrap_or(false);
                let is_dir = file_type.map(|ft| ft.is_dir()).unwrap_or(false);

                entries.push(serde_json::json!({
                    "name": name,
                    "is_file": is_file,
                    "is_dir": is_dir
                }));
            }
            
            entries.sort_by(|a, b| {
                let a_is_dir = a.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                let b_is_dir = b.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                if a_is_dir != b_is_dir {
                    b_is_dir.cmp(&a_is_dir)
                } else {
                    a.get("name").and_then(|v| v.as_str()).unwrap_or("").cmp(
                        b.get("name").and_then(|v| v.as_str()).unwrap_or("")
                    )
                }
            });

            serde_json::json!({
                "success": true,
                "path": path,
                "entries": entries,
                "count": entries.len(),
                "message": format!("列出 {} 个条目", entries.len())
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "path": path,
            "error": format!("读取目录失败: {}", e)
        })
    }
}

async fn tool_run_command_with_retry(
    command: &str,
    max_retries: u32,
    retry_interval_secs: u64,
    timeout_secs: u64,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    log::info!("[Agent] 带重试执行命令: {}, max_retries={}", command, max_retries);

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("🔄 执行命令（最多重试 {} 次）: {}", max_retries, command),
    }));

    let interval = tokio::time::Duration::from_secs(retry_interval_secs);
    let timeout = tokio::time::Duration::from_secs(timeout_secs);
    let mut final_stdout: Option<String> = None;
    let mut final_stderr: Option<String> = None;
    let mut final_exit_code: Option<i32> = None;
    let mut success = false;

    for attempt in 0..max_retries {
        let command_owned = command.to_string();
        
        let result = tokio::time::timeout(timeout, async move {
            tokio::task::spawn_blocking(move || {
                Command::new("sh")
                    .arg("-c")
                    .arg(&command_owned)
                    .output()
            }).await
        }).await;

        match result {
            Ok(Ok(Ok(output))) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                
                if output.status.success() {
                    final_stdout = Some(stdout);
                    final_stderr = Some(stderr);
                    final_exit_code = exit_code;
                    success = true;
                    break;
                }
                final_stdout = Some(stdout);
                final_stderr = Some(stderr);
                final_exit_code = exit_code;
                
                if attempt < max_retries - 1 {
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("⚠️ 第 {} 次尝试失败，{} 秒后重试...", attempt + 1, retry_interval_secs),
                    }));
                    tokio::time::sleep(interval).await;
                }
            }
            Ok(Ok(Err(e))) => {
                if attempt < max_retries - 1 {
                    tokio::time::sleep(interval).await;
                }
            }
            Ok(Err(e)) => {
                if attempt < max_retries - 1 {
                    tokio::time::sleep(interval).await;
                }
            }
            Err(_) => {
                if attempt < max_retries - 1 {
                    tokio::time::sleep(interval).await;
                }
            }
        }
    }

    let message = if success { 
        "命令执行成功".to_string() 
    } else { 
        format!("命令失败，退出码: {:?}", final_exit_code) 
    };
    
    serde_json::json!({
        "success": success,
        "stdout": final_stdout.unwrap_or_default(),
        "stderr": final_stderr.unwrap_or_default(),
        "exit_code": final_exit_code,
        "attempts": max_retries,
        "message": message
    })
}

fn tool_get_ollama_models() -> serde_json::Value {
    log::info!("[Agent] 获取 Ollama 模型列表");

    let ollama_bin = find_ollama_bin().unwrap_or_else(|| "ollama".to_string());
    let ollama_dir = "/Applications/Ollama.app/Contents/Resources";
    let current_path = std::env::var("PATH").unwrap_or_default();
    let enhanced_path = format!("{}:{}", ollama_dir, current_path);

    let output = Command::new("sh")
        .env("PATH", &enhanced_path)
        .arg("-c")
        .arg(format!("{} list 2>/dev/null", ollama_bin))
        .output();

    match output {
        Ok(o) => {
            if !o.status.success() {
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                return serde_json::json!({
                    "success": false,
                    "error": format!("获取模型列表失败: {}", stderr)
                });
            }

            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let mut models: Vec<serde_json::Value> = vec![];

            for line in stdout.lines().skip(1) {
                if line.trim().is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(name) = parts.first() {
                    let size = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                    let modified = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
                    
                    models.push(serde_json::json!({
                        "name": name,
                        "size": size,
                        "modified": modified
                    }));
                }
            }

            serde_json::json!({
                "success": true,
                "models": models,
                "count": models.len(),
                "raw_output": stdout
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "error": format!("执行命令失败: {}", e)
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
                "download_model" => {
                    let source = tool_args.get("source").and_then(|v| v.as_str()).unwrap_or("ollama");
                    let model = tool_args.get("model").and_then(|v| v.as_str()).unwrap_or("");
                    let cache_dir = tool_args.get("cache_dir").and_then(|v| v.as_str());
                    let timeout = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(300);
                    
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📥 下载模型: {} ({})", model, source),
                    }));
                    
                    tool_download_model(source, model, cache_dir, timeout, &app).await
                }
                "start_inference_server" => {
                    let server_type = tool_args.get("server_type").and_then(|v| v.as_str()).unwrap_or("ollama");
                    let model = tool_args.get("model").and_then(|v| v.as_str()).unwrap_or("");
                    let port = tool_args.get("port").and_then(|v| v.as_u64()).unwrap_or(11434) as u16;
                    let gpu_layers = tool_args.get("gpu_layers").and_then(|v| v.as_i64()).map(|v| v as i32);
                    let background = tool_args.get("background").and_then(|v| v.as_bool()).unwrap_or(true);
                    
                    tool_start_inference_server(server_type, model, port, gpu_layers, background, &app).await
                }
                "wait_for_condition" => {
                    let target = tool_args.get("target").and_then(|v| v.as_str()).unwrap_or("");
                    let target_type = tool_args.get("target_type").and_then(|v| v.as_str()).unwrap_or("http");
                    let expected = tool_args.get("expected").and_then(|v| v.as_str()).unwrap_or("");
                    let max_attempts = tool_args.get("max_attempts").and_then(|v| v.as_u64()).unwrap_or(30) as u32;
                    let interval_secs = tool_args.get("interval_seconds").and_then(|v| v.as_u64()).unwrap_or(2);
                    let timeout_secs = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(60);
                    
                    tool_wait_for_condition(target, target_type, expected, max_attempts, interval_secs, timeout_secs).await
                }
                "kill_process" => {
                    let process_name = tool_args.get("process_name").and_then(|v| v.as_str()).unwrap_or("");
                    let force = tool_args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
                    
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("🔪 终止进程: {}", process_name),
                    }));
                    
                    tool_kill_process(process_name, force).await
                }
                "write_file" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let content = tool_args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📝 写文件: {}", path),
                    }));
                    
                    tool_write_file(path, content).await
                }
                "read_file" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📄 读文件: {}", path),
                    }));
                    
                    tool_read_file(path).await
                }
                "file_exists" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    tool_file_exists(path)
                }
                "list_directory" => {
                    let path = tool_args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let include_hidden = tool_args.get("include_hidden").and_then(|v| v.as_bool()).unwrap_or(false);
                    
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": format!("📂 列出目录: {}", path),
                    }));
                    
                    tool_list_directory(path, include_hidden).await
                }
                "run_command_with_retry" => {
                    let command = tool_args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    let max_retries = tool_args.get("max_retries").and_then(|v| v.as_u64()).unwrap_or(3) as u32;
                    let retry_interval = tool_args.get("retry_interval_seconds").and_then(|v| v.as_u64()).unwrap_or(5);
                    let timeout = tool_args.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(30);
                    
                    tool_run_command_with_retry(command, max_retries, retry_interval, timeout, &app).await
                }
                "get_ollama_models" => {
                    let _ = app.emit("workflow-message", serde_json::json!({
                        "type": "progress",
                        "content": "📦 获取 Ollama 模型列表...",
                    }));
                    
                    tool_get_ollama_models()
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

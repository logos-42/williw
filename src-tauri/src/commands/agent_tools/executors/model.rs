/// Model tools executor
///
/// Provides AI model download and inference server management capabilities.

use std::process::Command;
use serde_json;
use tauri::Emitter;

/// Download AI models from Ollama or HuggingFace.
pub async fn download_model(
    source: &str,
    model: &str,
    cache_dir: Option<&str>,
    timeout_secs: u64,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    log::info!("[Agent] 下载模型：source={}, model={}", source, model);

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("📥 开始下载模型：{} (来源：{})", model, source),
    }));

    match source {
        "ollama" => {
            let ollama_bin = find_ollama_bin().unwrap_or_else(|| "ollama".to_string());
            let command = format!("{} pull {}", ollama_bin, model);

            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "progress",
                "content": format!("🔧 执行命令：{}", command),
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
                            "content": format!("✅ 模型下载成功：{}", model),
                        }));
                    }

                    serde_json::json!({
                        "success": success,
                        "source": source,
                        "model": model,
                        "stdout": stdout,
                        "stderr": stderr,
                        "message": if success { format!("模型 {} 下载成功", model) } else { format!("下载失败：{}", stderr) }
                    })
                }
                Ok(Ok(Err(e))) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("执行错误：{}", e)
                }),
                Ok(Err(e)) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("任务错误：{}", e)
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
                        "message": if success { format!("模型 {} 下载成功", model) } else { format!("下载失败：{}", stderr) }
                    })
                }
                Ok(Ok(Err(e))) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("执行错误：{}", e)
                }),
                Ok(Err(e)) => serde_json::json!({
                    "success": false,
                    "source": source,
                    "model": model,
                    "error": format!("任务错误：{}", e)
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
            "error": format!("不支持的模型来源：{}", source)
        })
    }
}

/// Start a local inference server. Supports Ollama, llama.cpp server.
pub async fn start_inference_server(
    server_type: &str,
    model: &str,
    port: u16,
    gpu_layers: Option<i32>,
    background: bool,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    log::info!("[Agent] 启动推理服务器：type={}, model={}, port={}", server_type, model, port);

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("🚀 启动推理服务器：{} (模型：{}, 端口：{})", server_type, model, port),
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
                    "error": format!("启动失败：{}", e)
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
                    "error": format!("启动失败：{}", e)
                })
            }
        }
        _ => serde_json::json!({
            "success": false,
            "error": format!("不支持的服务器类型：{}", server_type)
        })
    }
}

/// Get list of installed Ollama models and their status.
pub fn get_ollama_models() -> serde_json::Value {
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
                    "error": format!("获取模型列表失败：{}", stderr)
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
            "error": format!("执行命令失败：{}", e)
        })
    }
}

/// Find ollama binary path.
fn find_ollama_bin() -> Option<String> {
    let extra_paths = [
        "/Applications/Ollama.app/Contents/Resources/ollama",
        "/usr/local/bin/ollama",
        "/opt/homebrew/bin/ollama",
    ];
    
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

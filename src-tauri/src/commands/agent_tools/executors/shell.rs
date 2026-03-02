/// Shell tools executor
///
/// Provides shell command execution capabilities with retry and timeout support.

use std::process::Command;
use serde_json;
use tauri::Emitter;

/// Execute a shell command on the user's machine.
/// Used for installing software, starting services, downloading models, etc.
/// Note: Commands will be executed in real-time, use with caution.
/// For long-running commands (like ollama pull), wait up to 300 seconds.
pub async fn run_shell_command(
    command: &str,
    timeout_secs: u64,
    background: bool,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    // Display the command about to be executed
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": format!("🔧 执行命令：`{}`", command),
    }));

    log::info!("[Agent] 执行命令：{}", command);

    // Build enhanced PATH including Ollama's known installation directories
    let ollama_dir = "/Applications/Ollama.app/Contents/Resources";
    let current_path = std::env::var("PATH").unwrap_or_default();
    let enhanced_path = if current_path.contains(ollama_dir) {
        current_path.clone()
    } else {
        format!("{}:{}", ollama_dir, current_path)
    };

    if background {
        // Run in background, don't wait
        let result = Command::new("sh")
            .env("PATH", &enhanced_path)
            .arg("-c")
            .arg(format!("{} &", command))
            .spawn();

        match result {
            Ok(_) => {
                // Wait a bit for service to start
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                serde_json::json!({
                    "success": true,
                    "stdout": format!("命令已在后台启动：{}", command),
                    "stderr": ""
                })
            }
            Err(e) => serde_json::json!({
                "success": false,
                "error": format!("启动失败：{}", e)
            })
        }
    } else {
        // Run in foreground, wait for completion
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

                // Truncate output if too long
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
                "error": format!("执行错误：{}", e)
            }),
            Ok(Err(e)) => serde_json::json!({
                "success": false,
                "error": format!("任务错误：{}", e)
            }),
            Err(_) => serde_json::json!({
                "success": false,
                "error": format!("命令超时（{}秒）", timeout_secs)
            })
        }
    }
}

/// Execute a shell command with automatic retry on failure.
pub async fn run_command_with_retry(
    command: &str,
    max_retries: u32,
    retry_interval_secs: u64,
    timeout_secs: u64,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    log::info!("[Agent] 带重试执行命令：{}, max_retries={}", command, max_retries);

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
        format!("命令失败，退出码：{:?}", final_exit_code)
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

/// Execute Python code and return the output.
pub async fn run_python(
    code: &str,
    timeout_secs: u64,
    app: &tauri::AppHandle,
) -> serde_json::Value {
    log::info!("[Agent] 运行 Python 代码 (timeout: {}s)", timeout_secs);

    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "progress",
        "content": "🐍 执行 Python 代码...",
    }));

    let temp_file = format!("/tmp/williw_python_{}.py", std::process::id());
    let write_result = std::fs::write(&temp_file, &code);

    if let Err(e) = write_result {
        return serde_json::json!({
            "success": false,
            "error": format!("写入临时文件失败：{}", e)
        });
    }

    let output = Command::new("python3")
        .arg(&temp_file)
        .output();

    let _ = std::fs::remove_file(&temp_file);

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();

            serde_json::json!({
                "success": o.status.success(),
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": o.status.code()
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "error": format!("执行失败：{}", e)
        })
    }
}

use serde::{Deserialize, Serialize};
use serde_json;
use std::process::Command;
use std::fs;
use tokio::fs as async_fs;
use std::path::Path;
use crate::state::AppState;
use tauri::{State, Emitter};
use crate::commands::tools::definitions::{ToolDefinition, ToolParameter, ToolType};

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Tool executor for running individual tools
pub struct ToolExecutor;

impl ToolExecutor {
    /// Execute a tool with given arguments
    pub async fn execute_tool(
        tool_name: &str,
        args: serde_json::Value,
        app: &tauri::AppHandle,
        state: &State<'_, AppState>,
    ) -> Result<ToolResult, String> {
        let start_time = std::time::Instant::now();
        
        let result = match tool_name {
            "check_system" => Self::tool_check_system().await,
            "run_shell_command" => Self::tool_run_shell_command(args, app).await,
            "check_http_endpoint" => Self::tool_check_http_endpoint(args).await,
            "finish_setup" => Self::tool_finish_setup(args, app).await,
            "report_failure" => Self::tool_report_failure(args, app).await,
            "download_model" => Self::tool_download_model(args, app).await,
            "start_inference_server" => Self::tool_start_inference_server(args, app).await,
            "wait_for_condition" => Self::tool_wait_for_condition(args).await,
            "kill_process" => Self::tool_kill_process(args).await,
            "write_file" => Self::tool_write_file(args).await,
            "read_file" => Self::tool_read_file(args).await,
            "file_exists" => Self::tool_file_exists(args).await,
            "list_directory" => Self::tool_list_directory(args).await,
            "run_command_with_retry" => Self::tool_run_command_with_retry(args, app).await,
            "get_ollama_models" => Self::tool_get_ollama_models().await,
            "search_files" => Self::tool_search_files(args).await,
            "create_plan" => Self::tool_create_plan(args).await,
            "get_todos" => Self::tool_get_todos(args).await,
            "add_todo" => Self::tool_add_todo(args).await,
            "network_diagnosis" => Self::tool_network_diagnosis(args).await,
            "run_python" => Self::tool_run_python(args, app).await,
            "get_system_info" => Self::tool_get_system_info(args).await,
            "copy_file" => Self::tool_copy_file(args).await,
            "delete_file" => Self::tool_delete_file(args).await,
            "create_directory" => Self::tool_create_directory(args).await,
            "get_file_info" => Self::tool_get_file_info(args).await,
            _ => {
                return Err(format!("Unknown tool: {}", tool_name));
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(data) => Ok(ToolResult {
                success: true,
                data,
                error: None,
                execution_time_ms: execution_time,
            }),
            Err(error) => Ok(ToolResult {
                success: false,
                data: serde_json::json!({}),
                error: Some(error),
                execution_time_ms: execution_time,
            }),
        }
    }

    // System tools implementations
    async fn tool_check_system() -> Result<serde_json::Value, String> {
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

        // 检查关键命令是否存在
        let commands_to_check = vec!["ollama", "python3", "python", "pip3", "brew", "curl", "docker"];
        let mut commands_obj = serde_json::json!({});
        for cmd in commands_to_check {
            let exists = Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {} 2>/dev/null", cmd))
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            commands_obj[cmd] = serde_json::json!(exists);
        }
        result["commands"] = commands_obj;

        Ok(result)
    }

    async fn tool_run_shell_command(
        args: serde_json::Value,
        app: &tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'command' parameter")?;
        let timeout_secs = args.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);
        let background = args.get("background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 显示即将执行的命令
        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "progress",
            "content": format!("🔧 执行命令: `{}`", command),
        }));

        if background {
            // 后台运行
            let result = Command::new("sh")
                .arg("-c")
                .arg(format!("{} &", command))
                .spawn();
            
            match result {
                Ok(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    Ok(serde_json::json!({
                        "success": true,
                        "stdout": format!("命令已在后台启动: {}", command),
                        "stderr": ""
                    }))
                }
                Err(e) => Err(format!("启动失败: {}", e)),
            }
        } else {
            // 前台运行
            let timeout = tokio::time::Duration::from_secs(timeout_secs);
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
                    let success = output.status.success();
                    
                    let stdout_trimmed = if stdout.len() > 2000 {
                        format!("{}...(truncated)", &stdout[..2000])
                    } else {
                        stdout
                    };
                    
                    Ok(serde_json::json!({
                        "success": success,
                        "exit_code": output.status.code(),
                        "stdout": stdout_trimmed,
                        "stderr": &stderr[..std::cmp::min(stderr.len(), 500)]
                    }))
                }
                Ok(Ok(Err(e))) => Err(format!("执行错误: {}", e)),
                Ok(Err(e)) => Err(format!("任务错误: {}", e)),
                Err(_) => Err(format!("命令超时（{}秒）", timeout_secs)),
            }
        }
    }

    async fn tool_check_http_endpoint(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = args.get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url' parameter")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
        
        match client.get(url).send().await {
            Ok(resp) => Ok(serde_json::json!({
                "reachable": true,
                "status_code": resp.status().as_u16(),
                "ok": resp.status().is_success()
            })),
            Err(e) => Ok(serde_json::json!({
                "reachable": false,
                "error": format!("{}", e)
            })),
        }
    }

    async fn tool_finish_setup(
        args: serde_json::Value,
        app: &tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let inference_endpoint = args.get("inference_endpoint")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'inference_endpoint' parameter")?;
        let model_name = args.get("model_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'model_name' parameter")?;
        let summary = args.get("summary")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'summary' parameter")?;

        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "success",
            "content": format!("✅ 本地推理环境就绪！\n\n{}\n\n模型: {}\n端点: {}", summary, model_name, inference_endpoint),
        }));

        Ok(serde_json::json!({
            "success": true,
            "inference_endpoint": inference_endpoint,
            "model_name": model_name,
            "summary": summary,
            "using_local_model": false,
        }))
    }

    async fn tool_report_failure(
        args: serde_json::Value,
        app: &tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let reason = args.get("reason")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'reason' parameter")?;
        let suggestion = args.get("suggestion")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'suggestion' parameter")?;

        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "error",
            "content": format!("❌ 配置失败\n\n原因: {}\n\n建议: {}", reason, suggestion),
        }));

        Err(format!("配置失败: {} 建议: {}", reason, suggestion))
    }

    // File tools implementations
    async fn tool_write_file(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let content = args.get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'content' parameter")?;

        let path_obj = Path::new(path);
        
        if let Some(parent) = path_obj.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("创建父目录失败: {}", e))?;
            }
        }

        fs::write(path, content)
            .map_err(|e| format!("写入失败: {}", e))?;

        let bytes_written = content.len();
        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "bytes_written": bytes_written,
            "message": format!("成功写入 {} 字节", bytes_written)
        }))
    }

    async fn tool_read_file(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;

        let content = fs::read_to_string(path)
            .map_err(|e| format!("读取失败: {}", e))?;
        let size = content.len();

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "content": content,
            "size": size,
            "message": format!("成功读取 {} 字节", size)
        }))
    }

    async fn tool_file_exists(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;

        let path_obj = Path::new(path);
        let exists = path_obj.exists();
        let is_file = path_obj.is_file();
        let is_dir = path_obj.is_dir();

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "exists": exists,
            "is_file": is_file,
            "is_dir": is_dir,
            "message": if exists {
                if is_file { "是文件" } else if is_dir { "是目录" } else { "存在" }
            } else { "不存在" }
        }))
    }

    async fn tool_list_directory(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let include_hidden = args.get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut entries: Vec<serde_json::Value> = vec![];

        let dir = fs::read_dir(path)
            .map_err(|e| format!("读取目录失败: {}", e))?;

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

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "entries": entries,
            "count": entries.len(),
            "message": format!("列出 {} 个条目", entries.len())
        }))
    }

    async fn tool_copy_file(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let source = args.get("source")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'source' parameter")?;
        let destination = args.get("destination")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'destination' parameter")?;

        let result = if Path::new(source).is_dir() {
            Command::new("sh")
                .arg("-c")
                .arg(format!("cp -r '{}' '{}'", source, destination))
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(format!("cp '{}' '{}'", source, destination))
                .output()
        };
        
        match result {
            Ok(output) if output.status.success() => {
                Ok(serde_json::json!({
                    "success": true,
                    "source": source,
                    "destination": destination
                }))
            }
            Ok(output) => {
                Err(format!("复制失败: {}", String::from_utf8_lossy(&output.stderr)))
            }
            Err(e) => Err(format!("复制失败: {}", e)),
        }
    }

    async fn tool_delete_file(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let recursive = args.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = if recursive {
            Command::new("sh")
                .arg("-c")
                .arg(format!("rm -rf '{}'", path))
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(format!("rm '{}'", path))
                .output()
        };
        
        match result {
            Ok(output) if output.status.success() => {
                Ok(serde_json::json!({
                    "success": true,
                    "path": path
                }))
            }
            Ok(output) => {
                Err(format!("删除失败: {}", String::from_utf8_lossy(&output.stderr)))
            }
            Err(e) => Err(format!("删除失败: {}", e)),
        }
    }

    async fn tool_create_directory(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        let parents = args.get("parents")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let result = if parents {
            Command::new("sh")
                .arg("-c")
                .arg(format!("mkdir -p '{}'", path))
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(format!("mkdir '{}'", path))
                .output()
        };
        
        match result {
            Ok(output) if output.status.success() => {
                Ok(serde_json::json!({
                    "success": true,
                    "path": path
                }))
            }
            Ok(output) => {
                Err(format!("创建目录失败: {}", String::from_utf8_lossy(&output.stderr)))
            }
            Err(e) => Err(format!("创建目录失败: {}", e)),
        }
    }

    async fn tool_get_file_info(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;

        let metadata = fs::metadata(&path)
            .map_err(|e| format!("获取文件信息失败: {}", e))?;

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_symlink() {
            "symlink"
        } else {
            "file"
        };
        
        let modified = metadata.modified()
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.to_rfc3339()
            })
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "file_type": file_type,
            "size_bytes": metadata.len(),
            "size_human": Self::format_size(metadata.len()),
            "modified": modified,
            "readonly": metadata.permissions().readonly()
        }))
    }

    // Utility tools implementations
    async fn tool_run_command_with_retry(
        args: serde_json::Value,
        app: &tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'command' parameter")?;
        let max_retries = args.get("max_retries")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32;
        let retry_interval = args.get("retry_interval_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);
        let timeout = args.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "progress",
            "content": format!("🔄 执行命令（最多重试 {} 次）: {}", max_retries, command),
        }));

        let interval = tokio::time::Duration::from_secs(retry_interval);
        let timeout = tokio::time::Duration::from_secs(timeout);
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
                            "content": format!("⚠️ 第 {} 次尝试失败，{} 秒后重试...", attempt + 1, retry_interval),
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
        
        Ok(serde_json::json!({
            "success": success,
            "stdout": final_stdout.unwrap_or_default(),
            "stderr": final_stderr.unwrap_or_default(),
            "exit_code": final_exit_code,
            "attempts": max_retries,
            "message": message
        }))
    }

    async fn tool_search_files(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let pattern = args.get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let search_type = args.get("search_type")
            .and_then(|v| v.as_str())
            .unwrap_or("filename");
        let max_results = args.get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;
        
        let search_type = if search_type == "content" { "content" } else { "filename" };
        
        let output = if search_type == "content" {
            Command::new("sh")
                .arg("-c")
                .arg(format!("grep -r -l -- '{}' '{}' 2>/dev/null | head -{}", pattern, path, max_results))
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(format!("find '{}' -name '*{}*' -type f 2>/dev/null | head -{}", path, pattern, max_results))
                .output()
        };
        
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let files: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
                
                Ok(serde_json::json!({
                    "success": true,
                    "pattern": pattern,
                    "search_type": search_type,
                    "results": files,
                    "count": files.len()
                }))
            }
            Err(e) => Err(format!("搜索失败: {}", e)),
        }
    }

    async fn tool_create_plan(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let title = args.get("title")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'title' parameter")?;
        let steps = args.get("steps")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
            .unwrap_or_default();

        Ok(serde_json::json!({
            "success": true,
            "title": title,
            "steps": steps,
            "step_count": steps.len(),
            "created_at": chrono::Utc::now().to_rfc3339()
        }))
    }

    async fn tool_get_todos(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let status = args.get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        Ok(serde_json::json!({
            "success": true,
            "status": status,
            "todos": [],
            "message": "Todo list storage not yet implemented"
        }))
    }

    async fn tool_add_todo(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let title = args.get("title")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'title' parameter")?;
        let description = args.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let priority = args.get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        Ok(serde_json::json!({
            "success": true,
            "title": title,
            "description": description,
            "priority": priority,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "message": "Todo storage not yet implemented"
        }))
    }

    async fn tool_network_diagnosis(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let target = args.get("target")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'target' parameter")?;
        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("ping");
        let port = args.get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(80) as u16;
        
        let mut results = serde_json::json!({});
        
        if operation == "ping" || operation == "all" {
            let ping_output = Command::new("sh")
                .arg("-c")
                .arg(format!("ping -c 3 '{}' 2>&1 | tail -1", target))
                .output();
            
            if let Ok(output) = ping_output {
                results["ping"] = serde_json::json!({
                    "success": output.status.success(),
                    "output": String::from_utf8_lossy(&output.stdout).trim()
                });
            }
        }
        
        if operation == "dns" || operation == "all" {
            let dns_output = Command::new("sh")
                .arg("-c")
                .arg(format!("nslookup '{}' 2>&1", target))
                .output();
            
            if let Ok(output) = dns_output {
                results["dns"] = serde_json::json!({
                    "success": output.status.success(),
                    "output": String::from_utf8_lossy(&output.stdout).trim()
                });
            }
        }
        
        if operation == "port" || operation == "all" {
            let port_output = Command::new("sh")
                .arg("-c")
                .arg(format!("nc -zv -w3 '{}' {} 2>&1", target, port))
                .output();
            
            if let Ok(output) = port_output {
                results["port"] = serde_json::json!({
                    "success": output.status.success(),
                    "port": port,
                    "output": String::from_utf8_lossy(&output.stdout).trim()
                });
            }
        }
        
        Ok(serde_json::json!({
            "success": true,
            "target": target,
            "operation": operation,
            "results": results
        }))
    }

    async fn tool_run_python(
        args: serde_json::Value,
        app: &tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let code = args.get("code")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'code' parameter")?;
        let timeout = args.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "progress",
            "content": "🐍 执行 Python 代码...",
        }));
        
        let temp_file = format!("/tmp/williw_python_{}.py", std::process::id());
        let write_result = std::fs::write(&temp_file, &code);
        
        if let Err(e) = write_result {
            return Err(format!("写入临时文件失败: {}", e));
        }
        
        let output = Command::new("python3")
            .arg(&temp_file)
            .output();
        
        let _ = std::fs::remove_file(&temp_file);
        
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                Ok(serde_json::json!({
                    "success": output.status.success(),
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": output.status.code()
                }))
            }
            Err(e) => Err(format!("执行失败: {}", e)),
        }
    }

    async fn tool_get_system_info(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let category = args.get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        
        let category = if category.is_empty() { "all" } else { category };
        
        let mut info = serde_json::json!({});
        
        if category == "all" || category == "cpu" {
            let cpu_output = Command::new("sh")
                .arg("-c")
                .arg("sysctl -n machdep.cpu.brand_string 2>/dev/null || echo 'Unknown'")
                .output();
            
            let cpu_count = Command::new("sh")
                .arg("-c")
                .arg("nproc 2>/dev/null || sysctl -n hw.logicalcpu 2>/dev/null || echo 4")
                .output();
            
            info["cpu"] = serde_json::json!({
                "name": cpu_output.ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|| "Unknown".to_string()),
                "cores": cpu_count.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().to_string().parse().ok()).unwrap_or(4)
            });
        }
        
        if category == "all" || category == "memory" {
            let mem_output = Command::new("sh")
                .arg("-c")
                .arg("sysctl -n hw.memsize 2>/dev/null")
                .output();
            
            let mem_bytes = mem_output.ok()
                .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().to_string().parse::<u64>().ok())
                .unwrap_or(0);
            
            info["memory"] = serde_json::json!({
                "total_bytes": mem_bytes,
                "total_gb": mem_bytes / (1024 * 1024 * 1024)
            });
        }
        
        if category == "all" || category == "disk" {
            let disk_output = Command::new("sh")
                .arg("-c")
                .arg("df -h . 2>/dev/null | tail -1 | awk '{print $2, $4}'")
                .output();

            if let Ok(output) = disk_output {
                let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
                let parts: Vec<&str> = stdout_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    info["disk"] = serde_json::json!({
                        "total": parts[0],
                        "available": parts[1]
                    });
                }
            }
        }
        
        if category == "all" || category == "network" {
            let hostname = Command::new("sh")
                .arg("-c")
                .arg("hostname")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            
            info["network"] = serde_json::json!({
                "hostname": hostname
            });
        }
        
        Ok(serde_json::json!({
            "success": true,
            "category": category,
            "info": info
        }))
    }

    // Model tools implementations
    async fn tool_download_model(
        args: serde_json::Value,
        app: &tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let source = args.get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("ollama");
        let model = args.get("model")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'model' parameter")?;
        let cache_dir = args.get("cache_dir")
            .and_then(|v| v.as_str());
        let timeout_secs = args.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let _ = app.emit("workflow-message", serde_json::json!({
            "type": "progress",
            "content": format!("📥 开始下载模型: {} (来源: {})", model, source),
        }));

        match source {
            "ollama" => {
                let ollama_bin = Self::find_ollama_bin().unwrap_or_else(|| "ollama".to_string());
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
                        
                        Ok(serde_json::json!({
                            "success": success,
                            "source": source,
                            "model": model,
                            "stdout": stdout,
                            "stderr": stderr,
                            "message": if success { format!("模型 {} 下载成功", model) } else { format!("下载失败: {}", stderr) }
                        }))
                    }
                    Ok(Ok(Err(e))) => Err(format!("执行错误: {}", e)),
                    Ok(Err(e)) => Err(format!("任务错误: {}", e)),
                    Err(_) => Err(format!("下载超时（{}秒）", timeout_secs)),
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
                        
                        Ok(serde_json::json!({
                            "success": success,
                            "source": source,
                            "model": model,
                            "model_path": model_path,
                            "stdout": stdout,
                            "stderr": stderr,
                            "message": if success { format!("模型 {} 下载成功", model) } else { format!("下载失败: {}", stderr) }
                        }))
                    }
                    Ok(Ok(Err(e))) => Err(format!("执行错误: {}", e)),
                    Ok(Err(e)) => Err(format!("任务错误: {}", e)),
                    Err(_) => Err(format!("下载超时（{}秒）", timeout_secs)),
                }
            }
            _ => Err(format!("不支持的模型来源: {}", source)),
        }
    }

    async fn tool_start_inference_server(
        args: serde_json::Value,
        app: &tauri::AppHandle,
    ) -> Result<serde_json::Value, String> {
        let server_type = args.get("server_type")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'server_type' parameter")?;
        let model = args.get("model")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'model' parameter")?;
        let port = args.get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(11434) as u16;
        let gpu_layers = args.get("gpu_layers")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        let background = args.get("background")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

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
                        Ok(serde_json::json!({
                            "success": true,
                            "server_type": server_type,
                            "model": model,
                            "endpoint": endpoint,
                            "pid": child.id(),
                            "message": format!("Ollama 服务已启动 (PID: {})", child.id())
                        }))
                    }
                    Err(e) => Err(format!("启动失败: {}", e)),
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
                        Ok(serde_json::json!({
                            "success": true,
                            "server_type": server_type,
                            "model": model,
                            "endpoint": endpoint,
                            "pid": child.id(),
                            "message": format!("llama.cpp 服务已启动 (PID: {})", child.id())
                        }))
                    }
                    Err(e) => Err(format!("启动失败: {}", e)),
                }
            }
            _ => Err(format!("不支持的服务器类型: {}", server_type)),
        }
    }

    async fn tool_wait_for_condition(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let target = args.get("target")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'target' parameter")?;
        let target_type = args.get("target_type")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'target_type' parameter")?;
        let expected = args.get("expected")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'expected' parameter")?;
        let max_attempts = args.get("max_attempts")
            .and_then(|v| v.as_u64())
            .unwrap_or(30) as u32;
        let interval_secs = args.get("interval_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(2);
        let _timeout_secs = args.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);

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
                        Some(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
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

        Ok(serde_json::json!({
            "success": matched,
            "matched": matched,
            "attempts": attempts,
            "max_attempts": max_attempts,
            "message": if matched { 
                format!("条件在第 {} 次尝试后匹配", attempts) 
            } else { 
                format!("在 {} 次尝试后仍未匹配", max_attempts) 
            }
        }))
    }

    async fn tool_kill_process(args: serde_json::Value) -> Result<serde_json::Value, String> {
        let process_name = args.get("process_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'process_name' parameter")?;
        let force = args.get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

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
            Ok(output) => {
                let success = output.status.success();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                Ok(serde_json::json!({
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
                }))
            }
            Err(e) => Err(format!("执行错误: {}", e)),
        }
    }

    async fn tool_get_ollama_models() -> Result<serde_json::Value, String> {
        let ollama_bin = Self::find_ollama_bin().unwrap_or_else(|| "ollama".to_string());
        let ollama_dir = "/Applications/Ollama.app/Contents/Resources";
        let current_path = std::env::var("PATH").unwrap_or_default();
        let enhanced_path = format!("{}:{}", ollama_dir, current_path);

        let output = Command::new("sh")
            .env("PATH", &enhanced_path)
            .arg("-c")
            .arg(format!("{} list 2>/dev/null", ollama_bin))
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    return Err(format!("获取模型列表失败: {}", stderr));
                }

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
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

                Ok(serde_json::json!({
                    "success": true,
                    "models": models,
                    "count": models.len(),
                    "raw_output": stdout
                }))
            }
            Err(e) => Err(format!("执行命令失败: {}", e)),
        }
    }

    // Helper functions
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

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        
        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}
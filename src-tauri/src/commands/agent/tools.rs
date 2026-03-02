//! Agent 工具定义模块
//!
//! 定义 AI Agent 可用的各种工具的 JSON Schema

use serde_json::{json, Value};
use std::path::Path;

/// 获取所有工具定义（给 LLM 的 JSON schema）
pub fn get_tool_definitions() -> Value {
    json!([
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
                            "description": "Expected pattern (regex)"
                        },
                        "timeout": {
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
                "name": "file_exists",
                "description": "检查文件或目录是否存在。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "要检查的文件或目录路径"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_ollama_models",
                "description": "获取本地 Ollama 已安装的模型列表。",
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
                "name": "create_plan",
                "description": "创建任务计划，列出完成目标需要的步骤。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "计划标题"
                        },
                        "steps": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "步骤列表"
                        }
                    },
                    "required": ["title", "steps"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_todos",
                "description": "获取当前待办事项列表。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["pending", "in_progress", "completed"],
                            "description": "过滤状态"
                        }
                    },
                    "required": []
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "add_todo",
                "description": "添加新的待办事项。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "待办标题"
                        },
                        "description": {
                            "type": "string",
                            "description": "待办描述"
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["low", "medium", "high"],
                            "description": "优先级"
                        }
                    },
                    "required": ["title"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_system_info",
                "description": "获取系统信息（CPU、内存、GPU、磁盘等）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "enum": ["cpu", "memory", "gpu", "disk", "all"],
                            "description": "信息类别"
                        }
                    },
                    "required": ["category"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_file_info",
                "description": "获取文件或目录的详细信息（大小、修改时间等）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "文件或目录路径"
                        }
                    },
                    "required": ["path"]
                }
            }
        }
    ])
}

/// 工具 check_system 的实现
pub fn tool_check_system() -> Value {
    let mut result = json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });

    // 检查关键命令
    let mut commands = serde_json::Map::new();
    for cmd in &["ollama", "python3", "pip", "brew", "curl", "git", "docker"] {
        let which = std::process::Command::new("which")
            .arg(cmd)
            .output();
        
        let exists = which.as_ref().map(|o| o.status.success()).unwrap_or(false);
        let path_str = which.ok().and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        });
        
        commands.insert(
            cmd.to_string(),
            json!({
                "exists": exists,
                "path": path_str
            })
        );
    }
    result["commands"] = Value::Object(commands);

    result
}

/// 工具 file_exists 的实现
pub fn tool_file_exists(path: &str) -> Value {
    log::info!("[Agent] 检查文件是否存在: {}", path);
    
    let p = std::path::Path::new(path);
    let exists = p.exists();
    
    if exists {
        let is_dir = p.is_dir();
        let metadata = p.metadata().ok();
        
        json!({
            "exists": true,
            "is_directory": is_dir,
            "size": metadata.as_ref().map(|m| m.len()),
            "modified": metadata.as_ref().and_then(|m| m.modified().ok())
                .map(|t| {
                    let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap();
                    dur.as_secs() as i64
                })
        })
    } else {
        json!({
            "exists": false
        })
    }
}

/// 工具 get_ollama_models 的实现
pub fn tool_get_ollama_models() -> Value {
    log::info!("[Agent] 获取 Ollama 模型列表");
    
    let output = std::process::Command::new("ollama")
        .args(["list"])
        .output();
    
    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let models: Vec<String> = stdout.lines()
                .skip(1) // 跳过表头
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    parts.first().map(|s| s.to_string())
                })
                .collect();
            
            json!({
                "success": true,
                "models": models
            })
        }
        _ => json!({
            "success": false,
            "error": "Failed to get Ollama models"
        })
    }
}

/// 工具 create_plan 的实现
pub fn tool_create_plan(title: &str, steps: Vec<String>) -> Value {
    log::info!("[Agent] 创建计划: {}", title);
    
    json!({
        "success": true,
        "plan_id": format!("plan_{}", uuid::Uuid::new_v4()),
        "title": title,
        "steps": steps,
        "created_at": chrono::Utc::now().to_rfc3339()
    })
}

/// 工具 get_todos 的实现
pub fn tool_get_todos(status: &str) -> Value {
    log::info!("[Agent] 获取待办事项: status={}", status);
    
    json!({
        "success": true,
        "todos": [
            // 示例数据，实际应该从存储中获取
        ],
        "filter_status": status
    })
}

/// 工具 add_todo 的实现
pub fn tool_add_todo(title: &str, description: &str, priority: &str) -> Value {
    log::info!("[Agent] 添加待办: {} (priority: {})", title, priority);
    
    json!({
        "success": true,
        "todo_id": format!("todo_{}", uuid::Uuid::new_v4()),
        "title": title,
        "description": description,
        "priority": priority,
        "status": "pending",
        "created_at": chrono::Utc::now().to_rfc3339()
    })
}

/// 工具 get_system_info 的实现
pub fn tool_get_system_info(category: &str) -> Value {
    log::info!("[Agent] 获取系统信息: category={}", category);
    
    let mut info = serde_json::Map::new();
    
    match category {
        "cpu" | "all" => {
            let sys = sysinfo::System::new_all();
            info.insert("cpu".to_string(), json!({
                "cores": sys.cpus().len(),
                "architecture": std::env::consts::ARCH
            }));
        }
        _ => {}
    }
    
    // 添加内存信息
    if category == "memory" || category == "all" {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        info.insert("memory".to_string(), json!({
            "total": sys.total_memory(),
            "available": sys.available_memory(),
            "used": sys.used_memory()
        }));
    }
    
    Value::Object(info)
}

/// 工具 get_file_info 的实现
pub fn tool_get_file_info(path: String) -> Value {
    log::info!("[Agent] 获取文件信息: {}", path);
    
    let p = std::path::Path::new(&path);
    
    if !p.exists() {
        return json!({
            "success": false,
            "error": "File not found"
        });
    }
    
    let metadata = p.metadata().ok();
    
    json!({
        "success": true,
        "path": path,
        "is_file": p.is_file(),
        "is_dir": p.is_dir(),
        "size": metadata.as_ref().map(|m| m.len()),
        "modified": metadata.as_ref().and_then(|m| m.modified().ok())
            .map(|t| {
                let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap();
                dur.as_secs() as i64
            })
    })
}

/// 格式化文件大小
pub fn format_size(bytes: u64) -> String {
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
        format!("{} bytes", bytes)
    }
}

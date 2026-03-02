/// AI Agent Tool Definitions Module
///
/// This module contains all tool definitions, metadata structures, and enums
/// used by the AI agent system. Tools are exposed to LLMs via JSON schemas
/// for function calling.
///
/// # Organization
/// - [`ToolCategory`]: Enum for categorizing tools by functionality
/// - [`ToolPriority`]: Enum for tool execution priority levels
/// - [`ToolMetadata`]: Struct containing tool metadata
/// - [`get_tool_definitions()`]: Main function returning all tool JSON schemas
/// - Individual `get_<tool_name>_definition()` functions for each tool
use serde_json;

// ============================================================================
// Tool Metadata Structures
// ============================================================================

/// Represents the category of a tool based on its functionality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    /// System information and diagnostics
    System,
    /// Shell command execution
    Shell,
    /// HTTP endpoint checking and network operations
    Http,
    /// File system operations
    File,
    /// Process management
    Process,
    /// AI model management (download, list, etc.)
    Model,
    /// Inference server management
    Server,
    /// Condition waiting and polling
    Condition,
    /// Agent control flow (finish, report failure)
    Agent,
    /// Task and todo management
    Task,
    /// Network diagnosis
    Network,
    /// Code execution
    Code,
}

impl ToolCategory {
    /// Returns a human-readable string representation of the category
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolCategory::System => "system",
            ToolCategory::Shell => "shell",
            ToolCategory::Http => "http",
            ToolCategory::File => "file",
            ToolCategory::Process => "process",
            ToolCategory::Model => "model",
            ToolCategory::Server => "server",
            ToolCategory::Condition => "condition",
            ToolCategory::Agent => "agent",
            ToolCategory::Task => "task",
            ToolCategory::Network => "network",
            ToolCategory::Code => "code",
        }
    }
}

/// Represents the priority level of a tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPriority {
    /// Critical tools needed for basic operation
    Critical,
    /// High priority tools used frequently
    High,
    /// Normal priority tools
    Normal,
    /// Low priority or specialized tools
    Low,
}

impl ToolPriority {
    /// Returns a numeric priority value (higher = more important)
    pub fn as_u8(&self) -> u8 {
        match self {
            ToolPriority::Critical => 4,
            ToolPriority::High => 3,
            ToolPriority::Normal => 2,
            ToolPriority::Low => 1,
        }
    }
}

/// Metadata associated with a tool definition
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    /// The name of the tool (used in function calling)
    pub name: &'static str,
    /// Human-readable description of what the tool does
    pub description: &'static str,
    /// The category this tool belongs to
    pub category: ToolCategory,
    /// The priority level of this tool
    pub priority: ToolPriority,
    /// Whether the tool requires async execution
    pub is_async: bool,
    /// Whether the tool has side effects (modifies system state)
    pub has_side_effects: bool,
}

impl ToolMetadata {
    /// Creates a new ToolMetadata instance
    pub const fn new(
        name: &'static str,
        description: &'static str,
        category: ToolCategory,
        priority: ToolPriority,
        is_async: bool,
        has_side_effects: bool,
    ) -> Self {
        Self {
            name,
            description,
            category,
            priority,
            is_async,
            has_side_effects,
        }
    }
}

// ============================================================================
// Individual Tool Definition Functions
// ============================================================================

/// Get JSON schema definition for the check_system tool
fn get_check_system_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the run_shell_command tool
fn get_run_shell_command_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the check_http_endpoint tool
fn get_check_http_endpoint_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the finish_setup tool
fn get_finish_setup_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the report_failure tool
fn get_report_failure_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the download_model tool
fn get_download_model_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the start_inference_server tool
fn get_start_inference_server_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the wait_for_condition tool
fn get_wait_for_condition_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the kill_process tool
fn get_kill_process_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the write_file tool
fn get_write_file_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the read_file tool
fn get_read_file_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the file_exists tool
fn get_file_exists_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the list_directory tool
fn get_list_directory_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the run_command_with_retry tool
fn get_run_command_with_retry_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the get_ollama_models tool
fn get_get_ollama_models_definition() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// Get JSON schema definition for the search_files tool
fn get_search_files_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "search_files",
            "description": "Search for files by name or search within files for content patterns.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path to search in"},
                    "pattern": {"type": "string", "description": "File name pattern or content regex"},
                    "search_type": {"type": "string", "enum": ["filename", "content"], "default": "filename"},
                    "max_results": {"type": "integer", "default": 20}
                },
                "required": ["path", "pattern"]
            }
        }
    })
}

/// Get JSON schema definition for the create_plan tool
fn get_create_plan_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "create_plan",
            "description": "Create a task plan with multiple steps.",
            "parameters": {
                "type": "object",
                "properties": {
                    "title": {"type": "string", "description": "Plan title"},
                    "steps": {"type": "array", "items": {"type": "string"}, "description": "List of step descriptions"}
                },
                "required": ["title", "steps"]
            }
        }
    })
}

/// Get JSON schema definition for the get_todos tool
fn get_get_todos_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "get_todos",
            "description": "Get all todo items.",
            "parameters": {
                "type": "object",
                "properties": {
                    "status": {"type": "string", "enum": ["all", "pending", "completed"], "default": "all"}
                },
                "required": []
            }
        }
    })
}

/// Get JSON schema definition for the add_todo tool
fn get_add_todo_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "add_todo",
            "description": "Add a new todo item.",
            "parameters": {
                "type": "object",
                "properties": {
                    "title": {"type": "string", "description": "Todo title"},
                    "description": {"type": "string", "description": "Optional description"},
                    "priority": {"type": "string", "enum": ["low", "medium", "high"], "default": "medium"}
                },
                "required": ["title"]
            }
        }
    })
}

/// Get JSON schema definition for the network_diagnosis tool
fn get_network_diagnosis_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "network_diagnosis",
            "description": "Perform network diagnosis including ping, DNS lookup, and port checking.",
            "parameters": {
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "Target host or IP"},
                    "operation": {"type": "string", "enum": ["ping", "dns", "port", "all"], "default": "ping"},
                    "port": {"type": "integer", "description": "Port number for port check"}
                },
                "required": ["target"]
            }
        }
    })
}

/// Get JSON schema definition for the run_python tool
fn get_run_python_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "run_python",
            "description": "Execute Python code and return the output.",
            "parameters": {
                "type": "object",
                "properties": {
                    "code": {"type": "string", "description": "Python code to execute"},
                    "timeout_seconds": {"type": "integer", "default": 30}
                },
                "required": ["code"]
            }
        }
    })
}

/// Get JSON schema definition for the get_system_info tool
fn get_get_system_info_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "get_system_info",
            "description": "Get detailed system information including CPU, memory, disk, and network.",
            "parameters": {
                "type": "object",
                "properties": {
                    "category": {"type": "string", "enum": ["all", "cpu", "memory", "disk", "network"], "default": "all"}
                },
                "required": []
            }
        }
    })
}

/// Get JSON schema definition for the copy_file tool
fn get_copy_file_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "copy_file",
            "description": "Copy a file or directory from source to destination.",
            "parameters": {
                "type": "object",
                "properties": {
                    "source": {"type": "string", "description": "Source path"},
                    "destination": {"type": "string", "description": "Destination path"}
                },
                "required": ["source", "destination"]
            }
        }
    })
}

/// Get JSON schema definition for the delete_file tool
fn get_delete_file_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "delete_file",
            "description": "Delete a file or directory.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to delete"},
                    "recursive": {"type": "boolean", "default": false, "description": "Delete recursively for directories"}
                },
                "required": ["path"]
            }
        }
    })
}

/// Get JSON schema definition for the create_directory tool
fn get_create_directory_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "create_directory",
            "description": "Create a new directory.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path to create"},
                    "parents": {"type": "boolean", "default": true, "description": "Create parent directories"}
                },
                "required": ["path"]
            }
        }
    })
}

/// Get JSON schema definition for the get_file_info tool
fn get_get_file_info_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "get_file_info",
            "description": "Get file or directory information including size, modified time, and permissions.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File or directory path"}
                },
                "required": ["path"]
            }
        }
    })
}

// ============================================================================
// Main Tool Definitions Function
// ============================================================================

/// Returns all tool definitions as a JSON array.
///
/// This function aggregates all individual tool definitions and returns them
/// as a single JSON array suitable for passing to LLMs for function calling.
///
/// # Returns
/// A `serde_json::Value` containing an array of tool definition objects.
///
/// # Example
/// ```
/// let tools = get_tool_definitions();
/// // tools is a JSON array containing all tool schemas
/// ```
pub fn get_tool_definitions() -> serde_json::Value {
    serde_json::json!([
        get_check_system_definition(),
        get_run_shell_command_definition(),
        get_check_http_endpoint_definition(),
        get_finish_setup_definition(),
        get_report_failure_definition(),
        get_download_model_definition(),
        get_start_inference_server_definition(),
        get_wait_for_condition_definition(),
        get_kill_process_definition(),
        get_write_file_definition(),
        get_read_file_definition(),
        get_file_exists_definition(),
        get_list_directory_definition(),
        get_run_command_with_retry_definition(),
        get_get_ollama_models_definition(),
        get_search_files_definition(),
        get_create_plan_definition(),
        get_get_todos_definition(),
        get_add_todo_definition(),
        get_network_diagnosis_definition(),
        get_run_python_definition(),
        get_get_system_info_definition(),
        get_copy_file_definition(),
        get_delete_file_definition(),
        get_create_directory_definition(),
        get_get_file_info_definition()
    ])
}

// ============================================================================
// Tool Metadata Registry
// ============================================================================

/// Returns metadata for all registered tools.
///
/// This function provides metadata about each tool including its category,
/// priority, and execution characteristics.
///
/// # Returns
/// A vector of `ToolMetadata` instances for all registered tools.
pub fn get_all_tool_metadata() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata::new(
            "check_system",
            "检查当前机器的硬件信息和已安装的软件",
            ToolCategory::System,
            ToolPriority::Critical,
            false,
            false,
        ),
        ToolMetadata::new(
            "run_shell_command",
            "在用户机器上执行 shell 命令",
            ToolCategory::Shell,
            ToolPriority::Critical,
            true,
            true,
        ),
        ToolMetadata::new(
            "check_http_endpoint",
            "检查某个 HTTP 端点是否可达",
            ToolCategory::Http,
            ToolPriority::High,
            true,
            false,
        ),
        ToolMetadata::new(
            "finish_setup",
            "当本地推理环境已就绪时调用此工具",
            ToolCategory::Agent,
            ToolPriority::Critical,
            false,
            false,
        ),
        ToolMetadata::new(
            "report_failure",
            "当无法完成配置时调用，说明原因和建议",
            ToolCategory::Agent,
            ToolPriority::High,
            false,
            false,
        ),
        ToolMetadata::new(
            "download_model",
            "Download AI models from Ollama or HuggingFace",
            ToolCategory::Model,
            ToolPriority::High,
            true,
            true,
        ),
        ToolMetadata::new(
            "start_inference_server",
            "Start a local inference server",
            ToolCategory::Server,
            ToolPriority::High,
            true,
            true,
        ),
        ToolMetadata::new(
            "wait_for_condition",
            "Poll HTTP endpoint, command, or file until expected pattern matches",
            ToolCategory::Condition,
            ToolPriority::Normal,
            true,
            false,
        ),
        ToolMetadata::new(
            "kill_process",
            "Terminate a running process by name",
            ToolCategory::Process,
            ToolPriority::Normal,
            true,
            true,
        ),
        ToolMetadata::new(
            "write_file",
            "Write content to a file",
            ToolCategory::File,
            ToolPriority::High,
            true,
            true,
        ),
        ToolMetadata::new(
            "read_file",
            "Read content from a file",
            ToolCategory::File,
            ToolPriority::High,
            false,
            false,
        ),
        ToolMetadata::new(
            "file_exists",
            "Check if a file or directory exists",
            ToolCategory::File,
            ToolPriority::Normal,
            false,
            false,
        ),
        ToolMetadata::new(
            "list_directory",
            "List files and directories in a path",
            ToolCategory::File,
            ToolPriority::Normal,
            false,
            false,
        ),
        ToolMetadata::new(
            "run_command_with_retry",
            "Execute a shell command with automatic retry",
            ToolCategory::Shell,
            ToolPriority::Normal,
            true,
            true,
        ),
        ToolMetadata::new(
            "get_ollama_models",
            "Get list of installed Ollama models",
            ToolCategory::Model,
            ToolPriority::Normal,
            false,
            false,
        ),
        ToolMetadata::new(
            "search_files",
            "Search for files by name or content",
            ToolCategory::File,
            ToolPriority::Low,
            false,
            false,
        ),
        ToolMetadata::new(
            "create_plan",
            "Create a task plan with multiple steps",
            ToolCategory::Task,
            ToolPriority::Normal,
            false,
            true,
        ),
        ToolMetadata::new(
            "get_todos",
            "Get all todo items",
            ToolCategory::Task,
            ToolPriority::Low,
            false,
            false,
        ),
        ToolMetadata::new(
            "add_todo",
            "Add a new todo item",
            ToolCategory::Task,
            ToolPriority::Low,
            false,
            true,
        ),
        ToolMetadata::new(
            "network_diagnosis",
            "Perform network diagnosis",
            ToolCategory::Network,
            ToolPriority::Normal,
            true,
            false,
        ),
        ToolMetadata::new(
            "run_python",
            "Execute Python code",
            ToolCategory::Code,
            ToolPriority::Normal,
            true,
            false,
        ),
        ToolMetadata::new(
            "get_system_info",
            "Get detailed system information",
            ToolCategory::System,
            ToolPriority::Normal,
            false,
            false,
        ),
        ToolMetadata::new(
            "copy_file",
            "Copy a file or directory",
            ToolCategory::File,
            ToolPriority::Normal,
            true,
            true,
        ),
        ToolMetadata::new(
            "delete_file",
            "Delete a file or directory",
            ToolCategory::File,
            ToolPriority::Normal,
            true,
            true,
        ),
        ToolMetadata::new(
            "create_directory",
            "Create a new directory",
            ToolCategory::File,
            ToolPriority::Normal,
            true,
            true,
        ),
        ToolMetadata::new(
            "get_file_info",
            "Get file or directory information",
            ToolCategory::File,
            ToolPriority::Low,
            false,
            false,
        ),
    ]
}

/// Get metadata for a specific tool by name.
///
/// # Arguments
/// * `name` - The name of the tool to look up
///
/// # Returns
/// `Some(ToolMetadata)` if the tool exists, `None` otherwise.
pub fn get_tool_metadata_by_name(name: &str) -> Option<ToolMetadata> {
    get_all_tool_metadata().into_iter().find(|m| m.name == name)
}

/// Get all tools in a specific category.
///
/// # Arguments
/// * `category` - The category to filter by
///
/// # Returns
/// A vector of `ToolMetadata` instances matching the specified category.
pub fn get_tools_by_category(category: ToolCategory) -> Vec<ToolMetadata> {
    get_all_tool_metadata()
        .into_iter()
        .filter(|m| m.category == category)
        .collect()
}

/// Get all tools at or above a specific priority level.
///
/// # Arguments
/// * `min_priority` - The minimum priority level to include
///
/// # Returns
/// A vector of `ToolMetadata` instances meeting the priority requirement.
pub fn get_tools_by_priority(min_priority: ToolPriority) -> Vec<ToolMetadata> {
    let min_priority_value = min_priority.as_u8();
    get_all_tool_metadata()
        .into_iter()
        .filter(|m| m.priority.as_u8() >= min_priority_value)
        .collect()
}

use serde::{Deserialize, Serialize};
use serde_json;

/// Tool type enumeration for categorization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolType {
    System,
    File,
    Network,
    Model,
    Inference,
    Process,
    Utility,
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

/// Complete tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub tool_type: ToolType,
    pub parameters: Vec<ToolParameter>,
    pub examples: Vec<String>,
}

/// Tool registry for managing all available tools
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Vec<ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register_tool(&mut self, tool: ToolDefinition) {
        self.tools.push(tool);
    }

    pub fn register_tools(&mut self, tools: Vec<ToolDefinition>) {
        self.tools.extend(tools);
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.iter().find(|t| t.name == name)
    }

    pub fn get_all_tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    pub fn get_tools_by_type(&self, tool_type: &ToolType) -> Vec<&ToolDefinition> {
        self.tools.iter().filter(|t| &t.tool_type == tool_type).collect()
    }
}

/// Get all tool definitions
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // System tools
        ToolDefinition {
            name: "check_system".to_string(),
            description: "检查当前机器的硬件信息和已安装的软件。返回 OS、RAM、CPU、GPU、以及关键命令是否存在（ollama, python3, pip, brew, curl）。".to_string(),
            tool_type: ToolType::System,
            parameters: vec![],
            examples: vec![
                "检查系统配置".to_string(),
            ],
        },
        ToolDefinition {
            name: "run_shell_command".to_string(),
            description: "在用户机器上执行 shell 命令。用于安装软件、启动服务、下载模型等操作。注意：命令会真实执行，请谨慎选择。对于长时间运行的命令（如 ollama pull），会等待最多 300 秒。".to_string(),
            tool_type: ToolType::System,
            parameters: vec![
                ToolParameter {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "要执行的命令，例如 'ollama pull qwen2.5:0.5b'".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "timeout_seconds".to_string(),
                    param_type: "integer".to_string(),
                    description: "超时时间（秒），默认 30，下载操作可设为 300".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(30)),
                },
                ToolParameter {
                    name: "background".to_string(),
                    param_type: "boolean".to_string(),
                    description: "是否后台运行（不等待结束），用于启动服务".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(false)),
                },
            ],
            examples: vec![
                "ollama pull qwen2.5:1.5b".to_string(),
                "python3 -m pip install torch".to_string(),
            ],
        },
        ToolDefinition {
            name: "check_http_endpoint".to_string(),
            description: "检查某个 HTTP 端点是否可达（服务是否已启动）。".to_string(),
            tool_type: ToolType::Network,
            parameters: vec![
                ToolParameter {
                    name: "url".to_string(),
                    param_type: "string".to_string(),
                    description: "要检查的 URL，例如 'http://localhost:11434'".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "http://localhost:11434".to_string(),
            ],
        },
        ToolDefinition {
            name: "finish_setup".to_string(),
            description: "当本地推理环境已就绪时调用此工具，告知 Williw 推理端点和模型名称。".to_string(),
            tool_type: ToolType::System,
            parameters: vec![
                ToolParameter {
                    name: "inference_endpoint".to_string(),
                    param_type: "string".to_string(),
                    description: "推理服务的 base URL，例如 'http://localhost:11434/v1'".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "model_name".to_string(),
                    param_type: "string".to_string(),
                    description: "已加载的模型名称，例如 'qwen2.5:1.5b'".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "summary".to_string(),
                    param_type: "string".to_string(),
                    description: "用中文向用户解释配置了什么、为什么选这个模型".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "配置完成，使用 qwen2.5:1.5b 模型".to_string(),
            ],
        },
        ToolDefinition {
            name: "report_failure".to_string(),
            description: "当无法完成配置时调用，说明原因和建议。".to_string(),
            tool_type: ToolType::System,
            parameters: vec![
                ToolParameter {
                    name: "reason".to_string(),
                    param_type: "string".to_string(),
                    description: "失败原因（中文）".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "suggestion".to_string(),
                    param_type: "string".to_string(),
                    description: "给用户的建议（中文）".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "硬件不支持，建议使用 CPU 模式".to_string(),
            ],
        },
        // File tools
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write content to a file. Creates parent directories if needed.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "File path".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    description: "Content to write".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "创建配置文件".to_string(),
            ],
        },
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read content from a file.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "File path to read".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "读取配置文件".to_string(),
            ],
        },
        ToolDefinition {
            name: "file_exists".to_string(),
            description: "Check if a file or directory exists.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to check".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "检查文件是否存在".to_string(),
            ],
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "List files and directories in a path.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Directory path".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "include_hidden".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Include hidden files".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(false)),
                },
            ],
            examples: vec![
                "列出目录内容".to_string(),
            ],
        },
        ToolDefinition {
            name: "copy_file".to_string(),
            description: "Copy a file or directory from source to destination.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "source".to_string(),
                    param_type: "string".to_string(),
                    description: "Source path".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "destination".to_string(),
                    param_type: "string".to_string(),
                    description: "Destination path".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "复制文件".to_string(),
            ],
        },
        ToolDefinition {
            name: "delete_file".to_string(),
            description: "Delete a file or directory.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to delete".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "recursive".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Delete recursively for directories".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(false)),
                },
            ],
            examples: vec![
                "删除文件".to_string(),
            ],
        },
        ToolDefinition {
            name: "create_directory".to_string(),
            description: "Create a new directory.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Directory path to create".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "parents".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Create parent directories".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(true)),
                },
            ],
            examples: vec![
                "创建目录".to_string(),
            ],
        },
        ToolDefinition {
            name: "get_file_info".to_string(),
            description: "Get file or directory information including size, modified time, and permissions.".to_string(),
            tool_type: ToolType::File,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "File or directory path".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "获取文件信息".to_string(),
            ],
        },
        // Model tools
        ToolDefinition {
            name: "download_model".to_string(),
            description: "Download AI models from Ollama or HuggingFace. Supports both sources.".to_string(),
            tool_type: ToolType::Model,
            parameters: vec![
                ToolParameter {
                    name: "source".to_string(),
                    param_type: "string".to_string(),
                    description: "Model source (ollama or huggingface)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "model".to_string(),
                    param_type: "string".to_string(),
                    description: "Model name (e.g., qwen2.5:0.5b or meta-llama/Llama-3.2-1B)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "cache_dir".to_string(),
                    param_type: "string".to_string(),
                    description: "Optional cache directory".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "timeout_seconds".to_string(),
                    param_type: "integer".to_string(),
                    description: "Timeout in seconds".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(300)),
                },
            ],
            examples: vec![
                "下载 qwen2.5:1.5b 模型".to_string(),
            ],
        },
        ToolDefinition {
            name: "start_inference_server".to_string(),
            description: "Start a local inference server. Supports Ollama, llama.cpp server.".to_string(),
            tool_type: ToolType::Inference,
            parameters: vec![
                ToolParameter {
                    name: "server_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Server type (ollama or llama.cpp)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "model".to_string(),
                    param_type: "string".to_string(),
                    description: "Model name or path".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "port".to_string(),
                    param_type: "integer".to_string(),
                    description: "Port number".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(11434)),
                },
                ToolParameter {
                    name: "gpu_layers".to_string(),
                    param_type: "integer".to_string(),
                    description: "GPU layers for llama.cpp (-1 for all)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(-1)),
                },
                ToolParameter {
                    name: "background".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Run in background".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(true)),
                },
            ],
            examples: vec![
                "启动 Ollama 服务".to_string(),
            ],
        },
        ToolDefinition {
            name: "wait_for_condition".to_string(),
            description: "Poll HTTP endpoint, command, or file until expected pattern matches.".to_string(),
            tool_type: ToolType::Utility,
            parameters: vec![
                ToolParameter {
                    name: "target".to_string(),
                    param_type: "string".to_string(),
                    description: "URL, command, or file path".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "target_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Type (http, command, file)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "expected".to_string(),
                    param_type: "string".to_string(),
                    description: "Expected pattern".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "max_attempts".to_string(),
                    param_type: "integer".to_string(),
                    description: "Maximum attempts".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(30)),
                },
                ToolParameter {
                    name: "interval_seconds".to_string(),
                    param_type: "integer".to_string(),
                    description: "Interval between attempts".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(2)),
                },
                ToolParameter {
                    name: "timeout_seconds".to_string(),
                    param_type: "integer".to_string(),
                    description: "Overall timeout".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(60)),
                },
            ],
            examples: vec![
                "等待服务启动".to_string(),
            ],
        },
        ToolDefinition {
            name: "kill_process".to_string(),
            description: "Terminate a running process by name.".to_string(),
            tool_type: ToolType::Process,
            parameters: vec![
                ToolParameter {
                    name: "process_name".to_string(),
                    param_type: "string".to_string(),
                    description: "Process name to kill".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "force".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Force termination".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(false)),
                },
            ],
            examples: vec![
                "终止 ollama 进程".to_string(),
            ],
        },
        ToolDefinition {
            name: "run_command_with_retry".to_string(),
            description: "Execute a shell command with automatic retry on failure.".to_string(),
            tool_type: ToolType::Utility,
            parameters: vec![
                ToolParameter {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "Command to execute".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "max_retries".to_string(),
                    param_type: "integer".to_string(),
                    description: "Maximum retry attempts".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(3)),
                },
                ToolParameter {
                    name: "retry_interval_seconds".to_string(),
                    param_type: "integer".to_string(),
                    description: "Interval between retries".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(5)),
                },
                ToolParameter {
                    name: "timeout_seconds".to_string(),
                    param_type: "integer".to_string(),
                    description: "Command timeout".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(30)),
                },
            ],
            examples: vec![
                "带重试执行命令".to_string(),
            ],
        },
        ToolDefinition {
            name: "get_ollama_models".to_string(),
            description: "Get list of installed Ollama models and their status.".to_string(),
            tool_type: ToolType::Model,
            parameters: vec![],
            examples: vec![
                "获取已安装模型列表".to_string(),
            ],
        },
        ToolDefinition {
            name: "search_files".to_string(),
            description: "Search for files by name or search within files for content patterns.".to_string(),
            tool_type: ToolType::Utility,
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Directory path to search in".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "pattern".to_string(),
                    param_type: "string".to_string(),
                    description: "File name pattern or content regex".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "search_type".to_string(),
                    param_type: "string".to_string(),
                    description: "Search type (filename or content)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("filename")),
                },
                ToolParameter {
                    name: "max_results".to_string(),
                    param_type: "integer".to_string(),
                    description: "Maximum results to return".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(20)),
                },
            ],
            examples: vec![
                "搜索文件".to_string(),
            ],
        },
        ToolDefinition {
            name: "create_plan".to_string(),
            description: "Create a task plan with multiple steps.".to_string(),
            tool_type: ToolType::Utility,
            parameters: vec![
                ToolParameter {
                    name: "title".to_string(),
                    param_type: "string".to_string(),
                    description: "Plan title".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "steps".to_string(),
                    param_type: "array".to_string(),
                    description: "List of step descriptions".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            examples: vec![
                "创建任务计划".to_string(),
            ],
        },
        ToolDefinition {
            name: "get_todos".to_string(),
            description: "Get all todo items.".to_string(),
            tool_type: ToolType::Utility,
            parameters: vec![
                ToolParameter {
                    name: "status".to_string(),
                    param_type: "string".to_string(),
                    description: "Status filter (all, pending, completed)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("all")),
                },
            ],
            examples: vec![
                "获取待办事项".to_string(),
            ],
        },
        ToolDefinition {
            name: "add_todo".to_string(),
            description: "Add a new todo item.".to_string(),
            tool_type: ToolType::Utility,
            parameters: vec![
                ToolParameter {
                    name: "title".to_string(),
                    param_type: "string".to_string(),
                    description: "Todo title".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "description".to_string(),
                    param_type: "string".to_string(),
                    description: "Optional description".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "priority".to_string(),
                    param_type: "string".to_string(),
                    description: "Priority (low, medium, high)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("medium")),
                },
            ],
            examples: vec![
                "添加待办事项".to_string(),
            ],
        },
        ToolDefinition {
            name: "network_diagnosis".to_string(),
            description: "Perform network diagnosis including ping, DNS lookup, and port checking.".to_string(),
            tool_type: ToolType::Network,
            parameters: vec![
                ToolParameter {
                    name: "target".to_string(),
                    param_type: "string".to_string(),
                    description: "Target host or IP".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    description: "Operation type (ping, dns, port, all)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("ping")),
                },
                ToolParameter {
                    name: "port".to_string(),
                    param_type: "integer".to_string(),
                    description: "Port number for port check".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(80)),
                },
            ],
            examples: vec![
                "网络诊断".to_string(),
            ],
        },
        ToolDefinition {
            name: "run_python".to_string(),
            description: "Execute Python code and return the output.".to_string(),
            tool_type: ToolType::Utility,
            parameters: vec![
                ToolParameter {
                    name: "code".to_string(),
                    param_type: "string".to_string(),
                    description: "Python code to execute".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "timeout_seconds".to_string(),
                    param_type: "integer".to_string(),
                    description: "Timeout in seconds".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(30)),
                },
            ],
            examples: vec![
                "执行 Python 代码".to_string(),
            ],
        },
        ToolDefinition {
            name: "get_system_info".to_string(),
            description: "Get detailed system information including CPU, memory, disk, and network.".to_string(),
            tool_type: ToolType::System,
            parameters: vec![
                ToolParameter {
                    name: "category".to_string(),
                    param_type: "string".to_string(),
                    description: "Category (all, cpu, memory, disk, network)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("all")),
                },
            ],
            examples: vec![
                "获取系统信息".to_string(),
            ],
        },
    ]
}

/// Get tool definitions as JSON for AI consumption (backward compatibility)
pub fn get_tool_definitions_json() -> serde_json::Value {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        }
    ])
}
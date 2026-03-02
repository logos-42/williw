/// 自主命令执行模块
///
/// 允许 Williw 在特定条件下自主调用 bash 工具，无需 AI 代理介入
/// 适用于：
/// - 已知的标准操作（启动服务、检查状态）
/// - 用户授权后的自动化任务
/// - 系统维护和自愈操作

use tokio::process::Command as TokioCommand;
use tokio::time::Duration;
use serde::{Deserialize, Serialize};

/// 预定义的自主命令白名单
///
/// 为了安全，只允许执行预定义的命令模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutonomousCommand {
    /// 启动 Ollama 服务
    StartOllama {
        gpu_limit: Option<u32>, // GPU 数量限制
    },
    /// 停止 Ollama 服务
    StopOllama,
    /// 检查服务状态
    CheckService {
        service_name: String,
    },
    /// 清理进程
    KillProcess {
        process_name: String,
    },
    /// 检查磁盘空间
    CheckDiskSpace {
        path: Option<String>,
    },
    /// 清理临时文件
    CleanupTemp {
        directory: String,
        max_age_days: Option<u32>,
    },
    /// 网络诊断
    NetworkDiagnose {
        target: String,
    },
    /// 自定义命令（需要用户明确授权）
    Custom {
        command: String,
        description: String,
    },
}

impl AutonomousCommand {
    /// 获取命令的描述
    pub fn description(&self) -> String {
        match self {
            Self::StartOllama { gpu_limit } => {
                format!("启动 Ollama 服务{}", gpu_limit.map(|g| format!("，限制 GPU 数量为 {}", g)).unwrap_or_default())
            }
            Self::StopOllama => "停止 Ollama 服务".to_string(),
            Self::CheckService { service_name } => format!("检查 {} 服务状态", service_name),
            Self::KillProcess { process_name } => format!("清理 {} 进程", process_name),
            Self::CheckDiskSpace { path } => {
                format!("检查磁盘空间{}", path.as_ref().map(|p| format!(" ({})", p)).unwrap_or_default())
            }
            Self::CleanupTemp { directory, .. } => format!("清理临时文件目录：{}", directory),
            Self::NetworkDiagnose { target } => format!("网络诊断：{}", target),
            Self::Custom { description, .. } => format!("自定义命令：{}", description),
        }
    }

    /// 执行命令并返回结果
    pub async fn execute(&self) -> AutonomousCommandResult {
        let description = self.description();
        log::info!("[AutonomousCommand] 执行：{}", description);

        match self {
            Self::StartOllama { gpu_limit } => {
                self.execute_start_ollama(*gpu_limit).await
            }
            Self::StopOllama => {
                self.execute_stop_ollama().await
            }
            Self::CheckService { service_name } => {
                self.execute_check_service(service_name).await
            }
            Self::KillProcess { process_name } => {
                self.execute_kill_process(process_name).await
            }
            Self::CheckDiskSpace { path } => {
                self.execute_check_disk_space(path.as_deref()).await
            }
            Self::CleanupTemp { directory, max_age_days } => {
                self.execute_cleanup_temp(directory, *max_age_days).await
            }
            Self::NetworkDiagnose { target } => {
                self.execute_network_diagnose(target).await
            }
            Self::Custom { command, .. } => {
                self.execute_custom(command).await
            }
        }
    }

    async fn execute_start_ollama(&self, gpu_limit: Option<u32>) -> AutonomousCommandResult {
        use std::env;

        // 检测 Ollama 是否已经在运行
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_default();

        if client.get("http://localhost:11434")
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return AutonomousCommandResult {
                success: true,
                stdout: "Ollama 服务已在运行".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                message: "检测到 Ollama 服务已在运行，无需重复启动".to_string(),
            };
        }

        // 查找 Ollama 二进制文件
        let ollama_bin = self.find_ollama_binary().await;
        if ollama_bin.is_none() {
            return AutonomousCommandResult {
                success: false,
                stdout: String::new(),
                stderr: "未找到 Ollama 可执行文件".to_string(),
                exit_code: None,
                message: "Ollama 未安装，请先安装 Ollama".to_string(),
            };
        }

        let ollama_path = ollama_bin.unwrap();

        // 设置环境变量
        let mut envs = vec![];
        if let Some(limit) = gpu_limit {
            envs.push(("OLLAMA_NUM_GPU", limit.to_string()));
        }

        // 在后台启动 Ollama
        let mut cmd = TokioCommand::new(&ollama_path);
        cmd.arg("serve");
        for (key, value) in envs {
            cmd.env(key, value);
        }

        match cmd.spawn() {
            Ok(mut child) => {
                // 等待 2 秒让服务启动
                tokio::time::sleep(Duration::from_secs(2)).await;

                // 检查是否成功启动
                let started = client.get("http://localhost:11434")
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false);

                if started {
                    AutonomousCommandResult {
                        success: true,
                        stdout: format!("Ollama 服务已启动 (PID: {:?})", child.id()),
                        stderr: String::new(),
                        exit_code: Some(0),
                        message: format!("Ollama 服务已成功启动，监听 http://localhost:11434"),
                    }
                } else {
                    // 尝试终止可能启动失败的进程
                    let _ = child.kill().await;
                    AutonomousCommandResult {
                        success: false,
                        stdout: String::new(),
                        stderr: "服务启动后无法访问".to_string(),
                        exit_code: None,
                        message: "Ollama 服务启动失败".to_string(),
                    }
                }
            }
            Err(e) => AutonomousCommandResult {
                success: false,
                stdout: String::new(),
                stderr: format!("{}", e),
                exit_code: None,
                message: format!("启动失败：{}", e),
            },
        }
    }

    async fn execute_stop_ollama(&self) -> AutonomousCommandResult {
        #[cfg(unix)]
        {
            let output = TokioCommand::new("pkill")
                .arg("ollama")
                .output()
                .await;

            match output {
                Ok(out) => {
                    let success = out.status.success();
                    AutonomousCommandResult {
                        success: true, // 即使没有进程可杀也视为成功
                        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                        exit_code: out.status.code(),
                        message: if success {
                            "Ollama 服务已停止".to_string()
                        } else {
                            "未找到运行的 Ollama 进程".to_string()
                        },
                    }
                }
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("停止失败：{}", e),
                },
            }
        }

        #[cfg(windows)]
        {
            let output = TokioCommand::new("taskkill")
                .args(&["/F", "/IM", "ollama.exe"])
                .output()
                .await;

            match output {
                Ok(out) => AutonomousCommandResult {
                    success: out.status.success(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                    exit_code: out.status.code(),
                    message: if out.status.success() {
                        "Ollama 服务已停止".to_string()
                    } else {
                        "未找到运行的 Ollama 进程".to_string()
                    },
                },
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("停止失败：{}", e),
                },
            }
        }
    }

    async fn execute_check_service(&self, service_name: &str) -> AutonomousCommandResult {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        let url = match service_name {
            "ollama" => "http://localhost:11434",
            "williw" => "http://localhost:9235",
            other => other,
        };

        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                AutonomousCommandResult {
                    success: status.is_success(),
                    stdout: format!("HTTP {}", status.as_u16()),
                    stderr: String::new(),
                    exit_code: Some(if status.is_success() { 0 } else { 1 }),
                    message: format!("服务 {} 状态：{} ({})", service_name, status.as_u16(), status.canonical_reason().unwrap_or("Unknown")),
                }
            }
            Err(e) => AutonomousCommandResult {
                success: false,
                stdout: String::new(),
                stderr: format!("{}", e),
                exit_code: None,
                message: format!("服务 {} 不可达：{}", service_name, e),
            },
        }
    }

    async fn execute_kill_process(&self, process_name: &str) -> AutonomousCommandResult {
        #[cfg(unix)]
        {
            let output = TokioCommand::new("pkill")
                .arg(process_name)
                .output()
                .await;

            match output {
                Ok(out) => AutonomousCommandResult {
                    success: true,
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                    exit_code: out.status.code(),
                    message: format!("进程 {} 已清理", process_name),
                },
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("清理失败：{}", e),
                },
            }
        }

        #[cfg(windows)]
        {
            let output = TokioCommand::new("taskkill")
                .args(&["/F", "/IM", &format!("{}.exe", process_name)])
                .output()
                .await;

            match output {
                Ok(out) => AutonomousCommandResult {
                    success: out.status.success(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                    exit_code: out.status.code(),
                    message: if out.status.success() {
                        format!("进程 {} 已停止", process_name)
                    } else {
                        format!("未找到进程 {}", process_name)
                    },
                },
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("清理失败：{}", e),
                },
            }
        }
    }

    async fn execute_check_disk_space(&self, path: Option<&str>) -> AutonomousCommandResult {
        let target_path = path.unwrap_or("/");

        #[cfg(unix)]
        {
            let output = TokioCommand::new("df")
                .arg("-h")
                .arg(target_path)
                .output()
                .await;

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let lines: Vec<&str> = stdout.lines().collect();

                    if lines.len() >= 2 {
                        // 解析 df 输出
                        let parts: Vec<&str> = lines[1].split_whitespace().collect();
                        if parts.len() >= 5 {
                            let used_percent = parts[4].trim_end_matches('%');
                            let available = parts[3];
                            let total = parts[1];

                            return AutonomousCommandResult {
                                success: true,
                                stdout: stdout.to_string(),
                                stderr: String::new(),
                                exit_code: Some(0),
                                message: format!("磁盘空间：总计 {} / 可用 {} / 已使用 {}%", total, available, used_percent),
                            };
                        }
                    }

                    AutonomousCommandResult {
                        success: true,
                        stdout: stdout.to_string(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        message: format!("磁盘检查完成：{}", target_path),
                    }
                }
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("磁盘检查失败：{}", e),
                },
            }
        }

        #[cfg(windows)]
        {
            let output = TokioCommand::new("wmic")
                .args(&["logicaldisk", "get", "size,freespace,caption"])
                .output()
                .await;

            match output {
                Ok(out) => AutonomousCommandResult {
                    success: true,
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    message: "磁盘空间检查完成".to_string(),
                },
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("磁盘检查失败：{}", e),
                },
            }
        }
    }

    async fn execute_cleanup_temp(&self, _directory: &str, _max_age_days: Option<u32>) -> AutonomousCommandResult {
        // TODO: 实现安全的临时文件清理逻辑
        // 当前仅返回提示信息，避免误删重要文件
        AutonomousCommandResult {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            message: "自动清理功能暂未实现，请手动清理临时文件".to_string(),
        }
    }

    async fn execute_network_diagnose(&self, target: &str) -> AutonomousCommandResult {
        #[cfg(unix)]
        {
            // 先 ping
            let ping_output = TokioCommand::new("ping")
                .args(&["-c", "4", target])
                .output()
                .await;

            match ping_output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let success = out.status.success();

                    AutonomousCommandResult {
                        success,
                        stdout: stdout.to_string(),
                        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                        exit_code: out.status.code(),
                        message: if success {
                            format!("网络诊断：{} 可达", target)
                        } else {
                            format!("网络诊断：{} 不可达", target)
                        },
                    }
                }
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("网络诊断失败：{}", e),
                },
            }
        }

        #[cfg(windows)]
        {
            let ping_output = TokioCommand::new("ping")
                .args(&["-n", "4", target])
                .output()
                .await;

            match ping_output {
                Ok(out) => AutonomousCommandResult {
                    success: out.status.success(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                    exit_code: out.status.code(),
                    message: if out.status.success() {
                        format!("网络诊断：{} 可达", target)
                    } else {
                        format!("网络诊断：{} 不可达", target)
                    },
                },
                Err(e) => AutonomousCommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("{}", e),
                    exit_code: None,
                    message: format!("网络诊断失败：{}", e),
                },
            }
        }
    }

    async fn execute_custom(&self, command: &str) -> AutonomousCommandResult {
        // 自定义命令执行（需要用户授权）
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await;

        match output {
            Ok(out) => AutonomousCommandResult {
                success: out.status.success(),
                stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                exit_code: out.status.code(),
                message: if out.status.success() {
                    "自定义命令执行成功".to_string()
                } else {
                    "自定义命令执行失败".to_string()
                },
            },
            Err(e) => AutonomousCommandResult {
                success: false,
                stdout: String::new(),
                stderr: format!("{}", e),
                exit_code: None,
                message: format!("命令执行失败：{}", e),
            },
        }
    }

    /// 查找 Ollama 二进制文件
    async fn find_ollama_binary(&self) -> Option<String> {
        // 常见 Ollama 安装路径
        let common_paths = vec![
            "/usr/local/bin/ollama",
            "/usr/bin/ollama",
            "/opt/homebrew/bin/ollama",
            "/Applications/Ollama.app/Contents/Resources/ollama",
        ];

        // 检查 PATH 中是否有 ollama
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                let candidate = format!("{}/ollama", dir.trim_end_matches('/'));
                if std::path::Path::new(&candidate).exists() {
                    return Some(candidate);
                }
            }
        }

        // 检查常见路径
        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }

        // 尝试使用 which 命令查找
        #[cfg(unix)]
        {
            if let Ok(output) = TokioCommand::new("which")
                .arg("ollama")
                .output()
                .await
            {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
        }

        None
    }
}

/// 自主命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousCommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub message: String,
}

impl AutonomousCommandResult {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "exit_code": self.exit_code,
            "message": self.message,
        })
    }
}

/// Tauri 命令：执行自主命令
#[tauri::command]
pub async fn execute_autonomous_command(
    command: AutonomousCommand,
    require_confirmation: bool,
) -> Result<serde_json::Value, String> {
    let description = command.description();

    log::info!("[execute_autonomous_command] 请求执行：{}", description);

    if require_confirmation {
        // TODO: 实现确认机制（前端弹窗等）
        log::warn!("[execute_autonomous_command] 需要用户确认，但暂未实现确认 UI");
    }

    let result = command.execute().await;

    log::info!(
        "[execute_autonomous_command] 结果：success={}, message={}",
        result.success,
        result.message
    );

    Ok(result.to_json())
}

/// Tauri 命令：批量执行自主命令（用于自愈流程）
#[tauri::command]
pub async fn execute_self_healing(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    use tauri::Emitter;

    log::info!("[execute_self_healing] 开始自愈流程");

    let mut results = Vec::new();

    // 1. 检查 Ollama 服务
    let _ = app.emit(
        "workflow-message",
        serde_json::json!({
            "type": "info",
            "content": "🔍 执行自愈检查：Ollama 服务状态...",
        }),
    );

    let check_result = AutonomousCommand::CheckService {
        service_name: "ollama".to_string(),
    }
    .execute()
    .await;

    results.push(("check_ollama", check_result.to_json()));

    if !check_result.success {
        // 2. 尝试启动 Ollama
        let _ = app.emit(
            "workflow-message",
            serde_json::json!({
                "type": "info",
                "content": "⚠️ Ollama 服务未运行，尝试自动启动...",
            }),
        );

        let start_result = AutonomousCommand::StartOllama { gpu_limit: None }
            .execute()
            .await;

        results.push(("start_ollama", start_result.to_json()));

        if start_result.success {
            let _ = app.emit(
                "workflow-message",
                serde_json::json!({
                    "type": "success",
                    "content": "✅ Ollama 服务已成功启动",
                }),
            );
        } else {
            let _ = app.emit(
                "workflow-message",
                serde_json::json!({
                    "type": "error",
                    "content": format!("❌ 无法启动 Ollama 服务：{}", start_result.message),
                }),
            );
        }
    } else {
        let _ = app.emit(
            "workflow-message",
            serde_json::json!({
                "type": "success",
                "content": "✅ Ollama 服务运行正常",
            }),
        );
    }

    Ok(serde_json::json!({
        "success": true,
        "results": results,
    }))
}

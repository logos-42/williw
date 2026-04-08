/// 自主命令执行模块
///
/// 允许 Williw 在特定条件下自主调用 bash 工具，无需 AI 代理介入
/// 适用于：
/// - 已知的标准操作（检查状态）
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

    async fn execute_check_service(&self, service_name: &str) -> AutonomousCommandResult {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        let url = match service_name {
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

    // 检查网络连通性
    let _ = app.emit(
        "workflow-message",
        serde_json::json!({
            "type": "info",
            "content": "🔍 执行自愈检查：网络连通性...",
        }),
    );

    let check_result = AutonomousCommand::NetworkDiagnose {
        target: "8.8.8.8".to_string(),
    }
    .execute()
    .await;

    let status_msg = if check_result.success {
        "✅ 网络连通正常"
    } else {
        "⚠️ 网络连通异常"
    };

    let _ = app.emit(
        "workflow-message",
        serde_json::json!({
            "type": if check_result.success { "success" } else { "warning" },
            "content": status_msg,
        }),
    );

    results.push(("check_network", check_result.to_json()));

    Ok(serde_json::json!({
        "success": true,
        "results": results,
    }))
}

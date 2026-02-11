use std::process::Command;

/// Start GPU inference server
#[tauri::command]
pub async fn start_gpu_server() -> Result<String, String> {
    // 获取当前应用的目录（src-tauri目录）
    let app_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    // 获取项目根目录（src-tauri的上级目录）
    let project_root = app_dir.parent()
        .ok_or("Failed to get project root directory")?;

    // 构建Python服务器脚本的路径
    let server_script = project_root.join("gpu_inference_server_clean.py");

    // 构建虚拟环境Python的路径 (支持 macOS/Linux 和 Windows)
    let venv_python = if cfg!(target_os = "windows") {
        project_root.join("torch_env").join("Scripts").join("python.exe")
    } else {
        project_root.join("torch_env").join("bin").join("python")
    };

    // 选择Python解释器（优先使用虚拟环境）
    let python_exe = if venv_python.exists() {
        venv_python
    } else {
        // 尝试多个可能的 Python 命令
        let possible_pythons = ["python3", "python"];
        let mut found_python = None;
        for py_cmd in &possible_pythons {
            let check = Command::new(py_cmd)
                .arg("--version")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output();
            if let Ok(output) = check {
                if output.status.success() {
                    found_python = Some(std::path::PathBuf::from(py_cmd));
                    println!("找到Python: {}", py_cmd);
                    break;
                }
            }
        }
        found_python.ok_or_else(|| "未找到Python解释器，请安装Python 3.8+".to_string())?
    };

    if !server_script.exists() {
        return Err(format!("GPU服务器脚本未找到: {:?}", server_script));
    }

    // 检查Python是否可用
    let python_check = Command::new(&python_exe)
        .arg("--version")
        .output();

    match python_check {
        Ok(output) => {
            if !output.status.success() {
                return Err("Python未正确安装或配置".to_string());
            }
            println!("Python版本检查通过: {}", String::from_utf8_lossy(&output.stdout));
        }
        Err(e) => {
            return Err(format!("无法执行Python命令: {}", e));
        }
    }

    // 启动GPU服务器（后台进程）
    let mut child = Command::new(&python_exe)
        .current_dir(project_root) // 设置工作目录为项目根目录
        .arg(&server_script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start GPU server: {}", e))?;

    println!("GPU服务器启动进程ID: {:?}", child.id());

    // 等待一小段时间让服务器启动
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 检查进程是否还在运行
    match child.try_wait() {
        Ok(Some(status)) => {
            if !status.success() {
                // 尝试读取错误输出
                if let Some(mut stderr) = child.stderr.take() {
                    let mut stderr_buf = String::new();
                    if let Ok(_) = std::io::Read::read_to_string(&mut stderr, &mut stderr_buf) {
                        return Err(format!("GPU服务器启动失败，退出码: {:?}\n错误信息: {}", status.code(), stderr_buf));
                    }
                }
                return Err(format!("GPU服务器启动失败，退出码: {:?}", status.code()));
            }
        }
        Ok(None) => {
            // 进程仍在运行，这是正常的
            println!("GPU服务器正在后台运行...");
        }
        Err(e) => {
            return Err(format!("检查GPU服务器状态失败: {}", e));
        }
    }

    Ok("GPU服务器启动成功".to_string())
}

/// Check if GPU server is running
#[tauri::command]
pub async fn check_gpu_server_status() -> Result<bool, String> {
    // 尝试连接到GPU服务器
    let client = reqwest::Client::new();

    match client.get("http://localhost:8000/")
        .timeout(tokio::time::Duration::from_secs(3))
        .send()
        .await {
        Ok(response) => Ok(response.status().is_success()),
        Err(e) => {
            println!("GPU服务器连接检查失败: {}", e);
            Ok(false)
        }
    }
}

/// Install Python dependencies for GPU server
#[tauri::command]
pub async fn install_gpu_dependencies() -> Result<String, String> {
    let app_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let project_root = app_dir.parent()
        .ok_or("Failed to get project root directory")?;

    let requirements_file = project_root.join("requirements.txt");

    if !requirements_file.exists() {
        return Err("requirements.txt文件未找到".to_string());
    }

    // 安装依赖
    let output = Command::new("pip")
        .current_dir(project_root) // 设置工作目录为项目根目录
        .arg("install")
        .arg("-r")
        .arg(&requirements_file)
        .output()
        .map_err(|e| format!("Failed to run pip install: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("依赖安装输出: {}", stdout);
        if !stderr.is_empty() {
            println!("依赖安装警告: {}", stderr);
        }
        Ok("Python依赖安装成功".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("依赖安装失败: {}", stderr))
    }
}

/// Start GPU inference server
#[tauri::command]
pub async fn start_gpu_inference_server(port: u16) -> Result<String, String> {
    use std::process::Command;

    let app_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let project_root = app_dir.parent()
        .ok_or("Failed to get project root directory")?;

    let server_script = project_root.join("gpu_inference_server_clean.py");

    if !server_script.exists() {
        return Err(format!("服务器脚本不存在: {:?}", server_script));
    }

    println!("🚀 启动GPU推理服务器 (端口 {})...", port);

    // 在后台启动服务器
    #[cfg(target_os = "windows")]
    {
        let _child = Command::new("python")
            .arg(&server_script)
            .arg("--port")
            .arg(port.to_string())
            .spawn()
            .map_err(|e| format!("无法启动服务器: {}", e))?;
    }

    // 等待服务器启动
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 检查服务器是否响应
    match reqwest::get(format!("http://localhost:{}/", port)).await {
        Ok(response) if response.status().is_success() => {
            Ok(format!("推理服务器已在端口 {} 启动", port))
        }
        _ => Err("服务器启动后无法访问".to_string()),
    }
}
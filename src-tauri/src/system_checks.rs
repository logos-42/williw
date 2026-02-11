use std::process::Command;

/// 检查 Python 是否安装
pub fn check_python() -> Result<(bool, String), String> {
    let output = Command::new("python3")
        .arg("--version")
        .output()
        .map_err(|e| format!("无法执行 Python: {}", e))?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((true, version))
    } else {
        Ok((false, String::new()))
    }
}

/// 检查 pip 包管理器
pub fn check_pip() -> Result<(bool, String), String> {
    let output = Command::new("python3")
        .arg("-m")
        .arg("pip")
        .arg("--version")
        .output()
        .map_err(|e| format!("无法执行 pip: {}", e))?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((true, version))
    } else {
        Ok((false, String::new()))
    }
}

/// 检查 CUDA 和 GPU 可用性
pub fn check_cuda() -> Result<(bool, String), String> {
    // 尝试使用 nvidia-smi 检查 GPU
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total,driver_version")
        .arg("--format=csv,noheader,nounits")
        .output()
        .map_err(|e| format!("无法执行 nvidia-smi: {}", e))?;
    
    if output.status.success() {
        let info = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !info.is_empty() {
            return Ok((true, format!("GPU 可用: {}", info)));
        }
    }
    
    // 检查 CUDA 版本
    let output = Command::new("nvcc")
        .arg("--version")
        .output()
        .map_err(|e| format!("无法执行 nvcc: {}", e))?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok((true, version));
    }
    
    Ok((false, String::from("CUDA 不可用")))
}

/// 检查 PyTorch 安装
pub fn check_pytorch() -> Result<(bool, String), String> {
    let output = Command::new("python3")
        .arg("-c")
        .arg("import torch; print(torch.__version__)")
        .output()
        .map_err(|e| format!("无法检查 PyTorch: {}", e))?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            // 检查 CUDA 是否可用
            let cuda_output = Command::new("python3")
                .arg("-c")
                .arg("import torch; print('CUDA available' if torch.cuda.is_available() else 'CUDA not available')")
                .output()
                .ok();
            
            let cuda_status = if let Some(output) = cuda_output {
                let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                format!(" - {}", status)
            } else {
                String::new()
            };
            
            return Ok((true, format!("{}{}", version, cuda_status)));
        }
    }
    
    Ok((false, String::new()))
}

/// 检查 Transformers 库
pub fn check_transformers() -> Result<(bool, String), String> {
    let output = Command::new("python3")
        .arg("-c")
        .arg("import transformers; print(transformers.__version__)")
        .output()
        .map_err(|e| format!("无法检查 Transformers: {}", e))?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Ok((true, version));
        }
    }
    
    Ok((false, String::new()))
}

/// 安装 Python 依赖
pub fn install_python_dependencies() -> Result<(bool, String), String> {
    // 安装 PyTorch
    println!("正在安装 PyTorch...");
    let output = Command::new("python3")
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("torch")
        .arg("--quiet")
        .output()
        .map_err(|e| format!("无法安装 PyTorch: {}", e))?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    // 安装 Transformers
    println!("正在安装 Transformers...");
    let output = Command::new("python3")
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("transformers")
        .arg("--quiet")
        .output()
        .map_err(|e| format!("无法安装 Transformers: {}", e))?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    Ok((true, String::from("依赖安装成功")))
}

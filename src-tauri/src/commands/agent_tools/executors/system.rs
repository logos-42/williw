/// System tools executor
///
/// Provides system information and health check capabilities.

use std::process::Command;
use serde_json;

/// Check current system hardware and installed software.
/// Returns OS, RAM, CPU, GPU, and whether key commands exist (ollama, python3, pip, brew, curl).
pub fn check_system() -> serde_json::Value {
    let mut result = serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "commands": {}
    });

    // Check memory
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

    // Check CPU cores
    let cpu_output = Command::new("sh")
        .arg("-c")
        .arg("nproc 2>/dev/null || sysctl -n hw.logicalcpu 2>/dev/null || echo 4")
        .output();
    if let Ok(output) = cpu_output {
        let cpu_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let cpu_count: u32 = cpu_str.parse().unwrap_or(4);
        result["cpu_cores"] = serde_json::json!(cpu_count);
    }

    // Check GPU
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

    // Check Apple Silicon
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

    // macOS Ollama common installation paths (installed to /Applications but not in PATH)
    let ollama_extra_paths = vec![
        "/Applications/Ollama.app/Contents/Resources/ollama",
        "/usr/local/bin/ollama",
        "/opt/homebrew/bin/ollama",
    ];

    // Check if key commands exist (check both PATH and known fixed paths)
    let commands_to_check = vec!["ollama", "python3", "python", "pip3", "brew", "curl", "docker"];
    let mut commands_obj = serde_json::json!({});
    for cmd in commands_to_check {
        let in_path = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {} 2>/dev/null", cmd))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // For ollama, additionally check known fixed paths
        let exists = if cmd == "ollama" && !in_path {
            ollama_extra_paths.iter().any(|p| std::path::Path::new(p).exists())
        } else {
            in_path
        };
        commands_obj[cmd] = serde_json::json!(exists);
    }
    result["commands"] = commands_obj;

    // Find ollama actual path (for AI to use full path calls)
    let ollama_bin = {
        let in_path = Command::new("sh")
            .arg("-c")
            .arg("command -v ollama 2>/dev/null")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        if let Some(path) = in_path.filter(|p| !p.is_empty()) {
            path
        } else {
            ollama_extra_paths.iter()
                .find(|p| std::path::Path::new(p).exists())
                .map(|p| p.to_string())
                .unwrap_or_default()
        }
    };
    if !ollama_bin.is_empty() {
        result["ollama_bin_path"] = serde_json::json!(ollama_bin.clone());
    }

    // Check existing ollama models (use actual path)
    let ollama_cmd = if ollama_bin.is_empty() { "ollama".to_string() } else { ollama_bin };
    let ollama_models = Command::new("sh")
        .arg("-c")
        .arg(format!("{} list 2>/dev/null || echo 'ollama not found'", ollama_cmd))
        .output();
    if let Ok(output) = ollama_models {
        let models_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        result["ollama_models"] = serde_json::json!(models_str);
    }

    result
}

/// Get detailed system information including CPU, memory, disk, and network.
pub fn get_system_info(category: &str) -> serde_json::Value {
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

        if let Ok(o) = disk_output {
            let stdout_str = String::from_utf8_lossy(&o.stdout).to_string();
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

    serde_json::json!({
        "success": true,
        "category": category,
        "info": info
    })
}

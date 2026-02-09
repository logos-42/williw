use crate::device::types::{GpuComputeApi, NetworkType};
use std::process::Command;

/// 检测 Linux GPU API（增强版 - 真实检测 GPU 设备）
pub fn detect_gpu_apis() -> Vec<GpuComputeApi> {
    let mut apis = Vec::new();
    
    // 方法1: 使用 lspci 命令检测 GPU 设备
    if let Ok(output) = Command::new("lspci")
        .args(&["-nn"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let output_lower = output_str.to_lowercase();
            
            // 检测 NVIDIA GPU (VGA compatible controller: NVIDIA)
            if output_lower.contains("nvidia") {
                // 检查 CUDA 库
                let cuda_paths = [
                    "/usr/lib/x86_64-linux-gnu/libcuda.so",
                    "/usr/lib/libcuda.so",
                    "/usr/local/cuda/lib64/libcuda.so",
                ];
                for path in &cuda_paths {
                    if std::path::Path::new(path).exists() {
                        apis.push(GpuComputeApi::CUDA);
                        break;
                    }
                }
            }
            
            // 检测 AMD GPU
            if output_lower.contains("amd") || output_lower.contains("radeon") {
                // AMD GPU 通常支持 Vulkan 和 OpenCL
            }
            
            // 检测 Intel GPU
            if output_lower.contains("intel") && output_lower.contains("vga") {
                // Intel GPU 通常支持 Vulkan
            }
        }
    }
    
    // 方法2: 检查 /sys/class/drm 目录（DRM 设备）
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                // 检查是否是 GPU 设备（card0, card1 等）
                if name_str.starts_with("card") && name_str != "card" {
                    // 检查设备信息
                    let device_path = path.join("device");
                    if device_path.exists() {
                        // 读取供应商 ID
                        if let Ok(vendor_path) = device_path.join("vendor").canonicalize() {
                            if let Ok(vendor_id) = std::fs::read_to_string(vendor_path) {
                                let vendor_id = vendor_id.trim();
                                // NVIDIA: 0x10de, AMD: 0x1002, Intel: 0x8086
                                if vendor_id == "0x10de" {
                                    // NVIDIA - 检查 CUDA
                                    let cuda_paths = [
                                        "/usr/lib/x86_64-linux-gnu/libcuda.so",
                                        "/usr/lib/libcuda.so",
                                    ];
                                    for cuda_path in &cuda_paths {
                                        if std::path::Path::new(cuda_path).exists() {
                                            if !apis.contains(&GpuComputeApi::CUDA) {
                                                apis.push(GpuComputeApi::CUDA);
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 方法3: 检测 Vulkan
    let vulkan_paths = [
        "/usr/lib/x86_64-linux-gnu/libvulkan.so",
        "/usr/lib/libvulkan.so",
        "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
    ];
    for path in &vulkan_paths {
        if std::path::Path::new(path).exists() {
            apis.push(GpuComputeApi::Vulkan);
            break;
        }
    }
    
    // 方法4: 检测 OpenCL
    let opencl_paths = [
        "/usr/lib/x86_64-linux-gnu/libOpenCL.so",
        "/usr/lib/libOpenCL.so",
        "/usr/lib/x86_64-linux-gnu/libOpenCL.so.1",
    ];
    for path in &opencl_paths {
        if std::path::Path::new(path).exists() {
            apis.push(GpuComputeApi::OpenCL);
            break;
        }
    }
    
    // 去重
    apis.sort();
    apis.dedup();
    apis
}

/// 检测 Linux TPU/NPU 支持
pub fn detect_tpu() -> Option<bool> {
    // 检查是否有 Google Coral TPU 设备
    if std::path::Path::new("/dev/apex_0").exists() || std::path::Path::new("/dev/apex1").exists() {
        return Some(true);
    }
    
    // 检查是否有其他 AI 加速器设备
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name().unwrap().to_string_lossy();
            
            // 检查常见的 AI 加速器设备名
            if filename.starts_with("apex") ||      // Google Coral TPU
               filename.starts_with("mali") ||      // ARM Mali GPU
               filename.starts_with("neuron") ||    // AWS Inferentia
               filename.contains("ai") || 
               filename.contains("npu") {
                return Some(true);
            }
        }
    }
    
    // 检查系统中是否有 AI 加速器驱动加载
    if let Ok(modules) = std::fs::read_to_string("/proc/modules") {
        if modules.contains("tpu") || modules.contains("neuron") || 
           modules.contains("rockchip_rknpu") || modules.contains("mali") {
            return Some(true);
        }
    }
    
    Some(false)
}

/// 检测 Linux 网络类型（增强版 - 真实检测网络类型）
pub fn detect_network_type() -> NetworkType {
    // 方法1: 使用 nmcli 检测网络类型（最准确）
    if let Ok(output) = Command::new("nmcli")
        .args(&["-t", "-f", "TYPE,STATE", "device", "status"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            for line in output_str.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let net_type = parts[0].to_lowercase();
                    let state = parts[1].to_lowercase();
                    
                    if state == "connected" || state == "connected (externally)" {
                        if net_type == "wifi" || net_type == "802-11-wireless" {
                            return NetworkType::WiFi;
                        }
                        if net_type == "gsm" || net_type == "cdma" || net_type == "umts" ||
                           net_type == "lte" || net_type == "5g" {
                            if net_type == "5g" {
                                return NetworkType::Cellular5G;
                            }
                            return NetworkType::Cellular4G;
                        }
                    }
                }
            }
        }
    }
    
    // 方法2: 检查 /sys/class/net 目录（更底层的方法）
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                
                // 跳过回环接口
                if name_str == "lo" {
                    continue;
                }
                
                // 检查接口是否处于 UP 状态
                let operstate_path = path.join("operstate");
                if let Ok(operstate) = std::fs::read_to_string(&operstate_path) {
                    if operstate.trim() != "up" {
                        continue;
                    }
                
                    // 检查是否是 WiFi 接口（通常以 wlan 或 wlp 开头）
                    if name_str.starts_with("wlan") || name_str.starts_with("wlp") {
                        // 检查接口类型
                        let type_path = path.join("type");
                        if let Ok(type_content) = std::fs::read_to_string(&type_path) {
                            if let Ok(type_num) = type_content.trim().parse::<u32>() {
                                // ARPHRD_IEEE80211 = 801 (WiFi)
                                if type_num == 801 {
                                    return NetworkType::WiFi;
                                }
                            }
                        }
                    }
                    
                    // 检查是否是移动网络接口（通常以 wwan, usb, eth 开头，但需要进一步判断）
                    if name_str.starts_with("wwan") || name_str.starts_with("usb") {
                        // 检查是否是移动网络设备
                        let device_path = path.join("device");
                        if device_path.exists() {
                            // 可以进一步检查设备信息来确定是 4G 还是 5G
                            // 这里先返回 4G，实际可以通过读取更多信息来判断
                            return NetworkType::Cellular4G;
                        }
                    }
            }
        }
    }
    
    // 方法3: 使用 ip 命令检测
    if let Ok(output) = Command::new("ip")
        .args(&["link", "show"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            for line in output_str.lines() {
                let line_lower = line.to_lowercase();
                if line_lower.contains("state up") {
                    if line_lower.contains("wlan") || line_lower.contains("wifi") ||
                       line_lower.contains("wireless") {
                        return NetworkType::WiFi;
                    }
                }
            }
        }
    }
    
    NetworkType::Unknown
}

/// 检测 Linux 电池状态
pub fn detect_battery() -> (Option<f32>, bool) {
    use battery::Manager;
    
    if let Ok(manager) = Manager::new() {
        if let Ok(batteries) = manager.batteries() {
            for battery_result in batteries {
                if let Ok(battery) = battery_result {
                    if let Ok(state) = battery.state() {
                        let is_charging = matches!(
                            state,
                            battery::State::Charging | battery::State::Full
                        );
                        
                        if let Ok(percentage) = battery.state_of_charge() {
                            let level = percentage.value as f32;
                            if level >= 0.0 && level <= 1.0 {
                                return (Some(level), is_charging);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 备选方法：直接读取 /sys/class/power_supply
    if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();
            
            if name.starts_with("BAT") {
                // 读取电量
                let capacity_path = path.join("capacity");
                let status_path = path.join("status");
                
                if let (Ok(capacity_str), Ok(status_str)) = (
                    std::fs::read_to_string(&capacity_path),
                    std::fs::read_to_string(&status_path),
                ) {
                    if let Ok(capacity) = capacity_str.trim().parse::<f32>() {
                        let level = capacity / 100.0;
                        let is_charging = status_str.trim().to_lowercase().contains("charging")
                            || status_str.trim().to_lowercase().contains("full");
                        
                        if level >= 0.0 && level <= 1.0 {
                            return (Some(level), is_charging);
                        }
                    }
                }
            }
        }
    }
    
    (None, false)
}

/// 检测 GPU 使用率
pub fn detect_gpu_usage() -> Vec<crate::device::types::GpuUsageInfo> {
    use crate::device::types::GpuUsageInfo;
    let mut gpu_usages = Vec::new();

    // 方法1: 尝试使用 nvidia-smi（NVIDIA GPU）
    if let Ok(output) = Command::new("nvidia-smi")
        .args(&["--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
                "--format=csv,noheader,nounits"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            for line in output_str.lines() {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 5 {
                    let gpu_name = parts.get(0).unwrap_or(&"").trim().to_string();
                    let usage = parts.get(1).and_then(|s| s.trim().parse::<f32>().ok()).unwrap_or(0.0);
                    let mem_used = parts.get(2).and_then(|s| s.trim().parse::<u64>().ok());
                    let mem_total = parts.get(3).and_then(|s| s.trim().parse::<u64>().ok());
                    let temperature = parts.get(4).and_then(|s| s.trim().parse::<f32>().ok());

                    gpu_usages.push(GpuUsageInfo {
                        gpu_name: format!("NVIDIA {}", gpu_name),
                        usage_percent: usage,
                        memory_used_mb: mem_used.map(|v| v / 1024), // 转换为MB
                        memory_total_mb: mem_total.map(|v| v / 1024),
                        temperature,
                    });
                }
            }
        }
    }

    // 方法2: 尝试使用 AMDGPU（AMD GPU）
    if gpu_usages.is_empty() {
        if let Ok(output) = Command::new("ls")
            .args(&["/sys/class/drm/"])
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                for line in output_str.lines() {
                    let card_name = line.trim();
                    if card_name.starts_with("card") && card_name != "card" {
                        // 读取 GPU 使用率
                        let gpu_busy_path = format!("/sys/class/drm/{}/device/gpu_busy_percent", card_name);
                        if let Ok(busy_str) = std::fs::read_to_string(&gpu_busy_path) {
                            if let Ok(usage) = busy_str.trim().parse::<f32>() {
                                // 读取 GPU 名称
                                let card_path = format!("/sys/class/drm/{}/device", card_name);
                                if let Ok(gpu_info) = std::fs::read_to_string(format!("{}/uevent", card_path)) {
                                    let gpu_name = gpu_info
                                        .lines()
                                        .find(|l| l.starts_with("PRODUCT="))
                                        .map(|l| l.trim_start_matches("PRODUCT=").to_string())
                                        .unwrap_or_else(|| format!("AMD GPU ({})", card_name));

                                    gpu_usages.push(GpuUsageInfo {
                                        gpu_name,
                                        usage_percent: usage,
                                        memory_used_mb: None,
                                        memory_total_mb: None,
                                        temperature: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 方法3: 尝试使用 Intel GPU (intel_gpu_top)
    if gpu_usages.is_empty() {
        if let Ok(output) = Command::new("intel_gpu_top")
            .args(&["-J"])
            .output()
        {
            if let Ok(_output_str) = String::from_utf8(output.stdout) {
                // 解析 Intel GPU 使用率（这里简化处理）
                gpu_usages.push(GpuUsageInfo {
                    gpu_name: "Intel GPU".to_string(),
                    usage_percent: 0.0, // 需要解析 JSON
                    memory_used_mb: None,
                    memory_total_mb: None,
                    temperature: None,
                });
            }
        }
    }

    gpu_usages
}
 

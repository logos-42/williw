use crate::device::types::{GpuComputeApi, NetworkType};
use std::process::Command;

/// 检查框架是否存在（macOS）
fn check_framework_exists(framework: &str) -> bool {
    // 尝试使用 otool 检查框架
    if let Ok(output) = Command::new("otool")
        .args(&["-L", "/usr/bin/true"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            return output_str.contains(framework);
        }
    }
    
    // 备选：检查框架路径
    let framework_path = format!("/System/Library/Frameworks/{}.framework", framework);
    std::path::Path::new(&framework_path).exists()
}

/// 检测 macOS GPU API（增强版 - 真实检测 GPU 设备）
pub fn detect_gpu_apis() -> Vec<GpuComputeApi> {
    let mut apis = Vec::new();
    
    // 方法1: 使用 system_profiler 检测 GPU 设备
    if let Ok(output) = Command::new("system_profiler")
        .args(&["SPDisplaysDataType"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let output_lower = output_str.to_lowercase();
            
            // 检测 Apple Silicon (M1/M2/M3 等) - 支持 Metal
            if output_lower.contains("apple") && (output_lower.contains("m1") || 
                output_lower.contains("m2") || output_lower.contains("m3")) {
                apis.push(GpuComputeApi::Metal);
            }
            
            // 检测 AMD GPU
            if output_lower.contains("amd") || output_lower.contains("radeon") {
                // AMD GPU 在 macOS 上支持 Metal
                apis.push(GpuComputeApi::Metal);
            }
            
            // 检测 NVIDIA GPU（较老的 Mac）
            if output_lower.contains("nvidia") {
                // 较老的 Mac 可能支持 CUDA，但新版本 macOS 不支持
            }
            
            // 检测 Intel GPU
            if output_lower.contains("intel") {
                // Intel GPU 在 macOS 上支持 Metal
                apis.push(GpuComputeApi::Metal);
            }
        }
    }
    
    // 方法2: 检查 Metal 框架（macOS 10.11+ 通常可用）
    if check_framework_exists("Metal") {
        if !apis.contains(&GpuComputeApi::Metal) {
            apis.push(GpuComputeApi::Metal);
        }
    }
    
    // 方法3: 检测 Vulkan（通过 MoltenVK）
    if check_framework_exists("MoltenVK") {
        apis.push(GpuComputeApi::Vulkan);
    }
    
    // 方法4: OpenCL 在较新的 macOS 上已废弃，但可能仍可用
    if check_framework_exists("OpenCL") {
        apis.push(GpuComputeApi::OpenCL);
    }
    
    // 去重
    apis.sort();
    apis.dedup();
    apis
}

/// 检测 macOS TPU/NPU 支持
pub fn detect_tpu() -> Option<bool> {
    // macOS 通常没有原生 TPU 支持，但可能通过 USB 或 Thunderbolt 连接外部设备
    // 检查是否有 Google Coral TPU 设备连接
    if let Ok(output) = Command::new("system_profiler")
        .args(&["SPUSBDataType"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let output_lower = output_str.to_lowercase();
            
            // 检查是否有 TPU 相关设备
            if output_lower.contains("coral") || 
               output_lower.contains("edge tpu") ||
               output_lower.contains("google") {
                return Some(true);
            }
        }
    }
    
    // 检查是否有其他 AI 加速设备
    if let Ok(output) = Command::new("ioreg")
        .args(&["-p", "IOUSB", "-l"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let output_lower = output_str.to_lowercase();
            
            if output_lower.contains("neural") || 
               output_lower.contains("ai") ||
               output_lower.contains("accelerator") {
                return Some(true);
            }
        }
    }
    
    // 检查是否有通过 Homebrew 安装的 TPU 相关库
    let tpu_lib_paths = [
        "/opt/homebrew/lib/libtpu.dylib",
        "/usr/local/lib/libtpu.dylib",
    ];
    
    for path in &tpu_lib_paths {
        if std::path::Path::new(path).exists() {
            return Some(true);
        }
    }
    
    Some(false)
}

/// 检测 macOS 网络类型（增强版 - 真实检测网络类型）
pub fn detect_network_type() -> NetworkType {
    // 方法1: 使用 networksetup 检测 WiFi
    if let Ok(output) = Command::new("networksetup")
        .args(&["-listallhardwareports"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            // 查找 WiFi 接口（通常是 en0 或 en1）
            let mut wifi_interface = None;
            let lines: Vec<&str> = output_str.lines().collect();
            for i in 0..lines.len() {
                if lines[i].contains("Wi-Fi") || lines[i].contains("AirPort") {
                    // 下一行应该包含接口名称
                    if i + 1 < lines.len() {
                        let next_line = lines[i + 1];
                        if next_line.contains("Device:") {
                            let parts: Vec<&str> = next_line.split(':').collect();
                            if parts.len() >= 2 {
                                wifi_interface = Some(parts[1].trim());
                            }
                        }
                    }
                }
            }
            
            // 检查 WiFi 是否已连接
            if let Some(interface) = wifi_interface {
                if let Ok(status_output) = Command::new("networksetup")
                    .args(&["-getairportnetwork", interface])
                    .output()
                {
                    if let Ok(status_str) = String::from_utf8(status_output.stdout) {
                        if !status_str.contains("You are not associated") {
                            return NetworkType::WiFi;
                        }
                    }
                }
            }
        }
    }
    
    // 方法2: 使用 ifconfig 检测网络接口
    if let Ok(output) = Command::new("ifconfig")
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            for line in output_str.lines() {
                let line_lower = line.to_lowercase();
                // 检查 WiFi 接口（通常包含 status: active）
                if (line_lower.contains("en0") || line_lower.contains("en1")) &&
                   line_lower.contains("status: active") {
                    // 进一步检查是否是 WiFi
                    if let Ok(wifi_check) = Command::new("networksetup")
                        .args(&["-getairportnetwork", "en0"])
                        .output()
                    {
                        if let Ok(wifi_str) = String::from_utf8(wifi_check.stdout) {
                            if !wifi_str.contains("You are not associated") {
                                return NetworkType::WiFi;
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 方法3: 使用 scutil 检测网络服务
    if let Ok(output) = Command::new("scutil")
        .args(&["--nc", "list"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            for line in output_str.lines() {
                let line_lower = line.to_lowercase();
                if line_lower.contains("connected") {
                    if line_lower.contains("wi-fi") || line_lower.contains("airport") {
                        return NetworkType::WiFi;
                    }
                    // macOS 通常不直接支持移动网络，但可以通过 USB 调制解调器
                    if line_lower.contains("cellular") || line_lower.contains("mobile") {
                        return NetworkType::Cellular4G;
                    }
                }
            }
        }
    }
    
    NetworkType::Unknown
}

/// 检测 macOS 电池状态
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
    
    (None, false)
}

/// 检测 GPU 使用率
pub fn detect_gpu_usage() -> Vec<crate::device::types::GpuUsageInfo> {
    use crate::device::types::GpuUsageInfo;
    let mut gpu_usages = Vec::new();

    // 方法1: 使用 powermetrics 检测 GPU 使用率（需要管理员权限）
    if let Ok(output) = Command::new("powermetrics")
        .args(&["--samplers", "gpu_power", "-i", "1000", "-n", "1"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            // 解析 GPU 使用率
            for line in output_str.lines() {
                if line.contains("GPU active") {
                    if let Some(value_str) = line.split(':').nth(1) {
                        if let Ok(value) = value_str.trim().trim_end_matches('%').parse::<f32>() {
                            gpu_usages.push(GpuUsageInfo {
                                gpu_name: "Apple GPU (Metal)".to_string(),
                                usage_percent: value.clamp(0.0, 100.0),
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

    // 方法2: 使用 system_profiler 获取 GPU 信息
    if gpu_usages.is_empty() {
        if let Ok(output) = Command::new("system_profiler")
            .args(&["SPDisplaysDataType"])
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                // 提取 GPU 名称
                if let Some(gpu_name) = output_str
                    .lines()
                    .find(|line| line.contains("Chipset Model:"))
                    .map(|line| {
                        line.trim_start_matches("Chipset Model:")
                            .trim()
                            .to_string()
                    })
                {
                    gpu_usages.push(GpuUsageInfo {
                        gpu_name,
                        usage_percent: 0.0, // 无法获取实时使用率
                        memory_used_mb: None,
                        memory_total_mb: None,
                        temperature: None,
                    });
                }
            }
        }
    }

    gpu_usages
}


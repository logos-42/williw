use crate::device::types::{GpuComputeApi, NetworkType, GpuUsageInfo};
use std::process::Command;

/// 检查库是否存在
fn check_library_exists(lib_name: &str) -> bool {
    use libloading::Library;
    unsafe { Library::new(lib_name).is_ok() }
}

/// 检测 Windows GPU API（增强版 - 真实检测 GPU 设备）
pub fn detect_gpu_apis() -> Vec<GpuComputeApi> {
    let mut apis = Vec::new();
    
    // 方法1: 使用 wmic 命令检测 GPU 设备
    if let Ok(output) = Command::new("wmic")
        .args(&["path", "win32_VideoController", "get", "name", "/format:list"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let output_lower = output_str.to_lowercase();
            
            // 检测 NVIDIA GPU
            if output_lower.contains("nvidia") {
                // 检查 CUDA 库
                if check_library_exists("nvcuda.dll") {
                    apis.push(GpuComputeApi::CUDA);
                }
                // NVIDIA 也支持 Vulkan 和 OpenCL
                if check_library_exists("vulkan-1.dll") {
                    apis.push(GpuComputeApi::Vulkan);
                }
                if check_library_exists("OpenCL.dll") {
                    apis.push(GpuComputeApi::OpenCL);
                }
            }
            
            // 检测 AMD GPU
            if output_lower.contains("amd") || output_lower.contains("radeon") {
                // AMD GPU 通常支持 Vulkan 和 OpenCL
                if check_library_exists("vulkan-1.dll") {
                    apis.push(GpuComputeApi::Vulkan);
                }
                if check_library_exists("OpenCL.dll") {
                    apis.push(GpuComputeApi::OpenCL);
                }
                // 检查 AMD ROCm（如果安装）
                if check_library_exists("hipblas.dll") {
                    apis.push(GpuComputeApi::OpenCL); // ROCm 使用 OpenCL 接口
                }
            }
            
            // 检测 Intel GPU
            if output_lower.contains("intel") {
                // Intel GPU 支持 DirectX 和 Vulkan
                if check_library_exists("dxgi.dll") && check_library_exists("d3d12.dll") {
                    apis.push(GpuComputeApi::DirectX);
                }
                if check_library_exists("vulkan-1.dll") {
                    apis.push(GpuComputeApi::Vulkan);
                }
                if check_library_exists("OpenCL.dll") {
                    apis.push(GpuComputeApi::OpenCL);
                }
            }
        }
    }
    
    // 方法2: 检测 DirectX 12（Windows 10+ 通常支持）
    if check_library_exists("dxgi.dll") {
        if check_library_exists("d3d12.dll") {
            apis.push(GpuComputeApi::DirectX);
        }
    }
    
    // 方法3: 检测 Vulkan（通过注册表或库文件）
    if check_library_exists("vulkan-1.dll") {
        apis.push(GpuComputeApi::Vulkan);
    }
    
    // 方法4: 检测 OpenCL
    if check_library_exists("OpenCL.dll") {
        apis.push(GpuComputeApi::OpenCL);
    }
    
    // 去重
    apis.sort();
    apis.dedup();
    apis
}

/// 检测 Windows TPU/NPU 支持
pub fn detect_tpu() -> Option<bool> {
    // Windows 系统通常没有原生 TPU 支持，但可能通过特定驱动或库支持
    // 检查 Google TPU 驱动或其他 AI 加速器
    if check_library_exists("libtpu.dll") {
        Some(true)
    } else {
        // 尝试通过 WMI 查询特定的 AI 加速设备
        if let Ok(output) = Command::new("wmic")
            .args(&["path", "Win32_PnPEntity", "get", "name", "/format:list"])
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                let output_lower = output_str.to_lowercase();
                
                // 检查特定的 AI 加速器设备
                if output_lower.contains("edge tpu") || 
                   output_lower.contains("neural processing unit") ||
                   output_lower.contains("ai coprocessor") {
                    return Some(true);
                }
            }
        }
        
        // 检查其他可能的 AI 加速器
        Some(false)
    }
}

/// 检测 Windows 网络类型（增强版 - 真实检测网络类型）
pub fn detect_network_type() -> NetworkType {
    // 方法1: 使用 netsh 命令检测 WiFi 连接
    if let Ok(output) = Command::new("netsh")
        .args(&["wlan", "show", "interfaces"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            if output_str.contains("State") && output_str.contains("connected") {
                return NetworkType::WiFi;
            }
        }
    }
    
    // 方法2: 使用 wmic 检测网络适配器类型
    if let Ok(output) = Command::new("wmic")
        .args(&["path", "win32_networkadapter", "where", "netenabled=true", "get", "adaptertype,description", "/format:list"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let output_lower = output_str.to_lowercase();
            
            // 检测 WiFi (AdapterType = 9 或描述包含 wireless/wifi)
            if output_lower.contains("wireless") || output_lower.contains("wifi") || 
               output_lower.contains("wi-fi") || output_lower.contains("adaptertype=9") {
                return NetworkType::WiFi;
            }
            
            // 检测移动网络 (AdapterType = 20 或描述包含 cellular/mobile)
            if output_lower.contains("cellular") || output_lower.contains("mobile") ||
               output_lower.contains("adaptertype=20") {
                // 尝试检测是 4G 还是 5G
                if output_lower.contains("5g") || output_lower.contains("lte advanced") {
                    return NetworkType::Cellular5G;
                }
                return NetworkType::Cellular4G;
            }
        }
    }
    
    // 方法3: 使用 PowerShell 检测网络连接类型（更准确）
    if let Ok(output) = Command::new("powershell")
        .args(&["-Command", "Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | Select-Object -ExpandProperty InterfaceDescription"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let output_lower = output_str.to_lowercase();
            if output_lower.contains("wireless") || output_lower.contains("wifi") || 
               output_lower.contains("wi-fi") || output_lower.contains("802.11") {
                return NetworkType::WiFi;
            }
            if output_lower.contains("cellular") || output_lower.contains("mobile") ||
               output_lower.contains("lte") || output_lower.contains("5g") {
                if output_lower.contains("5g") {
                    return NetworkType::Cellular5G;
                }
                return NetworkType::Cellular4G;
            }
        }
    }
    
    NetworkType::Unknown
}

/// 检测 Windows 电池状态（增强版 - 真实检测电池状态）
pub fn detect_battery() -> (Option<f32>, bool) {
    // 方法1: 使用 battery 库（推荐）
    use battery::Manager;
    
    if let Ok(manager) = Manager::new() {
        if let Ok(batteries) = manager.batteries() {
            for battery_result in batteries {
                if let Ok(battery) = battery_result {
                    let state = battery.state();
                    let is_charging = matches!(
                        state,
                        battery::State::Charging | battery::State::Full
                    );
                    
                    let percentage = battery.state_of_charge();
                    let level = percentage.get::<battery::units::ratio::percent>() as f32 / 100.0;
                    if level >= 0.0 && level <= 1.0 {
                        return (Some(level), is_charging);
                    }
                }
            }
        }
    }
    
    // 方法2: 使用 wmic 命令作为备选
    if let Ok(output) = Command::new("wmic")
        .args(&["path", "win32_battery", "get", "batterystatus,estimatedchargeremaining", "/format:list"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let mut level: Option<f32> = None;
            let mut is_charging = false;
            
            for line in output_str.lines() {
                if line.starts_with("EstimatedChargeRemaining=") {
                    if let Ok(value) = line.split('=').nth(1).unwrap_or("").trim().parse::<f32>() {
                        level = Some((value / 100.0).clamp(0.0, 1.0));
                    }
                }
                if line.starts_with("BatteryStatus=") {
                    if let Ok(status) = line.split('=').nth(1).unwrap_or("").trim().parse::<u32>() {
                        // BatteryStatus: 2 = Charging, 4 = AC Power (Full)
                        is_charging = status == 2 || status == 4;
                    }
                }
            }
            
            if let Some(level_val) = level {
                return (Some(level_val), is_charging);
            }
        }
    }
    
    // 方法3: 使用 PowerShell 作为最后备选
    if let Ok(output) = Command::new("powershell")
        .args(&["-Command", "Get-WmiObject -Class Win32_Battery | Select-Object -ExpandProperty EstimatedChargeRemaining"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            if let Ok(percentage) = output_str.trim().parse::<f32>() {
                let level = (percentage / 100.0).clamp(0.0, 1.0);
                // 检查是否在充电
                if let Ok(charging_output) = Command::new("powershell")
                    .args(&["-Command", "Get-WmiObject -Class Win32_Battery | Select-Object -ExpandProperty BatteryStatus"])
                    .output()
                {
                    if let Ok(charging_str) = String::from_utf8(charging_output.stdout) {
                        if let Ok(status) = charging_str.trim().parse::<u32>() {
                            let is_charging = status == 2 || status == 4;
                            return (Some(level), is_charging);
                        }
                    }
                }
                return (Some(level), false);
            }
        }
    }
    
    // 方法4: 检查是否为桌面设备（无电池）
    // 检查系统类型，台式机通常没有电池
    if let Ok(output) = Command::new("wmic")
        .args(&["computersystem", "get", "PCSystemType", "/format:list"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            if output_str.contains("PCSystemType=2") {
                // Desktop
                return (None, false);
            }
        }
    }
    
    (None, false)
}

/// 检测 GPU 使用率
pub fn detect_gpu_usage() -> Vec<GpuUsageInfo> {
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

    // 方法2: 尝试使用 wmic 查询 GPU 性能（通用方法）
    if gpu_usages.is_empty() {
        if let Ok(output) = Command::new("wmic")
            .args(&["path", "win32_VideoController", "get", "name,AdapterRAM,CurrentRefreshRate", "/format:list"])
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                // 解析 GPU 名称和内存
                let mut gpu_name = String::from("Unknown GPU");
                let mut memory_mb: Option<u64> = None;

                for line in output_str.lines() {
                    if line.starts_with("Name=") {
                        if let Some(value) = line.split('=').nth(1) {
                            gpu_name = value.trim().to_string();
                        }
                    } else if line.starts_with("AdapterRAM=") {
                        if let Some(value) = line.split('=').nth(1) {
                            if let Ok(bytes) = value.trim().parse::<u64>() {
                                memory_mb = Some(bytes / (1024 * 1024));
                            }
                        }
                    }
                }

                // 对于非NVIDIA GPU，使用性能计数器获取使用率
                if let Ok(usage) = get_gpu_usage_from_performance_counter() {
                    gpu_usages.push(GpuUsageInfo {
                        gpu_name,
                        usage_percent: usage,
                        memory_used_mb: None,
                        memory_total_mb: memory_mb,
                        temperature: None,
                    });
                }
            }
        }
    }

    gpu_usages
}

/// 通过性能计数器获取 GPU 使用率（Windows 性能计数器）
fn get_gpu_usage_from_performance_counter() -> Result<f32, ()> {
    // 使用 PowerShell 查询 GPU 引擎使用率
    let command = r#"
        $gpu = Get-Counter '\GPU Engine(*)\Utilization Percentage' -ErrorAction SilentlyContinue
        if ($gpu) {
            $total = 0
            $count = 0
            foreach ($sample in $gpu.CounterSamples) {
                if ($sample.InstanceName -like '*3D*') {
                    $total += [float]$sample.CookedValue
                    $count++
                }
            }
            if ($count -gt 0) {
                [math]::Round($total / $count, 2)
            } else {
                0.0
            }
        } else {
            0.0
        }
    "#;

    if let Ok(output) = Command::new("powershell")
        .args(&["-Command", command])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            let trimmed = output_str.trim();
            if let Ok(usage) = trimmed.parse::<f32>() {
                return Ok(usage.clamp(0.0, 100.0));
            }
        }
    }

    Err(())
}


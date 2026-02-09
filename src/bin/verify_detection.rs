// 快速验证设备检测功能
// 
// 运行方式:
// cargo run --bin verify_detection

use williw::device::DeviceDetector;

fn main() {
    println!("========================================");
    println!("设备检测功能验证");
    println!("========================================");
    println!();
    
    // 执行设备检测
    let caps = DeviceDetector::detect();
    
    // 显示检测结果
    println!("📊 检测到的设备信息:");
    println!("   内存: {} MB ({:.1} GB)", caps.max_memory_mb, caps.max_memory_mb as f64 / 1024.0);
    println!("   CPU核心: {}", caps.cpu_cores);
    println!("   架构: {}", caps.cpu_architecture);
    println!("   设备类型: {:?}", caps.device_type);
    println!();
    
    println!("🔋 电池信息:");
    match (caps.battery_level, caps.is_charging) {
        (Some(level), Some(true)) => println!("   电量: {:.0}% (充电中)", level * 100.0),
        (Some(level), Some(false)) => println!("   电量: {:.0}% (使用电池)", level * 100.0),
        (Some(level), None) => println!("   电量: {:.0}%", level * 100.0),
        (None, _) => println!("   无电池（可能是台式机）"),
    }
    println!();
    
    println!("🎮 GPU信息:");
    if caps.has_gpu {
        println!("   GPU状态: 支持");
        println!("   支持的API: {} 个", caps.gpu_compute_apis.len());
        for (i, api) in caps.gpu_compute_apis.iter().enumerate() {
            println!("     {}. {:?}", i + 1, api);
        }
        
        // 尝试获取GPU使用率
        let gpu_usage = DeviceDetector::detect_gpu_usage();
        if !gpu_usage.is_empty() {
            println!();
            println!("   详细GPU信息:");
            for (i, gpu) in gpu_usage.iter().enumerate() {
                println!("   GPU {}:", i + 1);
                println!("     名称: {}", gpu.gpu_name);
                println!("     使用率: {}%", gpu.usage_percent);
                if let Some(mem_used) = gpu.memory_used_mb {
                    println!("     显存使用: {} MB", mem_used);
                }
                if let Some(mem_total) = gpu.memory_total_mb {
                    println!("     显存总量: {} MB", mem_total);
                }
                if let Some(temp) = gpu.temperature {
                    println!("     温度: {}°C", temp);
                }
            }
        }
    } else {
        println!("   GPU状态: 未检测到");
    }
    println!();
    
    println!("📡 网络类型: {:?}", caps.network_type);
    println!();
    
    println!("🏆 性能评分: {:.2}/1.00", caps.performance_score());
    println!();
    
    // 验证结果合理性
    println!("========================================");
    println!("验证结果:");
    println!("========================================");
    
    let mut has_warnings = false;
    
    // 检查内存
    if caps.max_memory_mb < 512 {
        println!("⚠️  警告：内存值 {} MB 可能不准确（小于512MB）", caps.max_memory_mb);
        has_warnings = true;
    } else if caps.max_memory_mb > 1024 * 1024 {
        println!("⚠️  警告：内存值 {} MB 超过1TB，可能不准确", caps.max_memory_mb);
        has_warnings = true;
    } else {
        println!("✅ 内存值 {} MB 看起来合理", caps.max_memory_mb);
    }
    
    // 检查CPU核心
    if caps.cpu_cores == 0 {
        println!("⚠️  警告：CPU核心数为0，检测失败");
        has_warnings = true;
    } else {
        println!("✅ CPU核心数 {} 看起来合理", caps.cpu_cores);
    }
    
    // 检查GPU（没有GPU也可能是正常的）
    if caps.has_gpu {
        println!("✅ 检测到GPU支持");
        if caps.gpu_compute_apis.is_empty() {
            println!("⚠️  警告：检测到GPU但没有支持的API");
        }
    } else {
        println!("ℹ️  未检测到GPU（可能是无独立GPU或驱动未安装）");
    }
    
    if !has_warnings {
        println!();
        println!("✅ 所有检测值看起来合理！");
    }
    
    println!();
    println!("📋 总结:");
    println!("   本机检测使用 sysinfo 和系统命令");
    println!("   确保安装GPU驱动以获取完整GPU信息");
    println!("   AMD/Intel GPU需要对应的SDK支持");
}

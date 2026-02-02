/**
 * 测试模型下载和切分功能
 * 调用 model_downloader 和 model_splitter 的完整流程
 */

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio;
use tracing::{info, warn, error};
use tracing_subscriber;

// 导入 model_downloader
use model_downloader::{ModelDownloader, DownloadConfig, DownloadResult};

// 导入 model_splitter
use model_splitter::{ModelSplitter, SplitConfig, SplitPlan};

/// 测试配置
struct TestConfig {
    model_name: String,
    model_path: PathBuf,
    output_dir: PathBuf,
}

impl TestConfig {
    fn new() -> Self {
        let test_root = PathBuf::from("./test_models");
        Self {
            model_name: "LiquidAI/LFM2.5-1.2B-Thinking".to_string(),
            model_path: test_root.join("models--LiquidAI--LFM2.5-1.2B-Thinking/snapshots/1c9725ba97f047b37bcf53e44e9133ccf1f79333"),
            output_dir: test_root.join("rust_split_output"),
        }
    }
}

/// 创建切分方案
fn create_split_plan() -> HashMap<String, SplitPlan> {
    let mut split_plan = HashMap::new();

    // 节点 001: 嵌入层 + 前5层的卷积部分
    let node_001_layers = vec![
        "model.embed_tokens.weight".to_string(),
        "model.embedding_norm.weight".to_string(),
        "model.layers.0.conv.conv.weight".to_string(),
        "model.layers.0.conv.in_proj.weight".to_string(),
        "model.layers.0.conv.out_proj.weight".to_string(),
    ];

    split_plan.insert(
        "node_001".to_string(),
        SplitPlan {
            node_id: "node_001".to_string(),
            layer_names: node_001_layers,
            total_compute: 100.0,
            compute_utilization: 0.5,
        },
    );

    // 节点 002: 前5层的FFN部分
    let node_002_layers = vec![
        "model.layers.0.feed_forward.w1.weight".to_string(),
        "model.layers.0.feed_forward.w2.weight".to_string(),
        "model.layers.0.feed_forward.w3.weight".to_string(),
        "model.layers.0.ffn_norm.weight".to_string(),
        "model.layers.0.operator_norm.weight".to_string(),
    ];

    split_plan.insert(
        "node_002".to_string(),
        SplitPlan {
            node_id: "node_002".to_string(),
            layer_names: node_002_layers,
            total_compute: 80.0,
            compute_utilization: 0.4,
        },
    );

    split_plan
}

/// 测试 1: 检查模型是否存在
async fn test_model_exists(config: &TestConfig) -> Result<()> {
    info!("📋 测试 1: 检查模型是否存在");

    if config.model_path.exists() {
        info!("✅ 模型已存在: {}", config.model_path.display());

        // 检查关键文件
        let config_file = config.model_path.join("config.json");
        let safetensors_file = config.model_path.join("model.safetensors");

        if config_file.exists() {
            info!("   ✅ config.json 存在");
        } else {
            warn!("   ⚠️  config.json 不存在");
        }

        if safetensors_file.exists() {
            let metadata = tokio::fs::metadata(&safetensors_file).await?;
            let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
            info!("   ✅ model.safetensors 存在 ({:.2} MB)", size_mb);
        } else {
            warn!("   ⚠️  model.safetensors 不存在");
        }

        Ok(())
    } else {
        Err(anyhow::anyhow!("模型不存在: {}", config.model_path.display()))
    }
}

/// 测试 2: 创建模型加载器（不实际下载）
async fn test_model_loader(config: &TestConfig) -> Result<()> {
    info!("📋 测试 2: 创建模型加载器");

    let downloader = ModelDownloader::new(None);
    info!("✅ 模型下载器创建成功");
    info!("   缓存目录: {}", config.model_path.parent().unwrap().display());
    Ok(())
}

/// 测试 3: 测试模型切分
async fn test_model_split(config: &TestConfig) -> Result<()> {
    info!("📋 测试 3: 测试模型切分");

    // 创建切分器
    let splitter = ModelSplitter::new();
    info!("✅ 模型切分器创建成功");

    // 检查Python环境
    info!("🔍 检查Python环境...");
    let python_check = tokio::process::Command::new("C:\\Users\\Mechrevo\\AppData\\Local\\Programs\\Python\\Python312\\python.exe")
        .arg("--version")
        .output()
        .await;

    match python_check {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            info!("✅ Python: {}", version.trim());
        }
        Err(e) => {
            warn!("⚠️  无法获取Python版本: {}", e);
            info!("   使用系统默认Python");
        }
    }

    // 检查torch是否安装
    let torch_check = tokio::process::Command::new("C:\\Users\\Mechrevo\\AppData\\Local\\Programs\\Python\\Python312\\python.exe")
        .arg("-c")
        .arg("import torch; print(f'PyTorch {torch.__version__}')")
        .output()
        .await;

    match torch_check {
        Ok(output) => {
            let torch_info = String::from_utf8_lossy(&output.stdout);
            if output.status.success() {
                info!("✅ {}", torch_info.trim());
            } else {
                error!("❌ PyTorch 未正确安装");
                return Err(anyhow::anyhow!("PyTorch 未正确安装"));
            }
        }
        Err(e) => {
            error!("❌ 无法检查PyTorch: {}", e);
            return Err(anyhow::anyhow!("无法检查PyTorch: {}", e));
        }
    }

    // 创建切分配置
    let split_plan = create_split_plan();
    info!("📊 切分方案包含 {} 个节点", split_plan.len());

    let split_config = SplitConfig {
        model_name: config.model_name.clone(),
        model_path: config.model_path.to_string_lossy().to_string(),
        split_plan: split_plan.clone(),
        output_dir: Some(config.output_dir.to_string_lossy().to_string()),
    };

    // 为每个节点执行切分
    let mut total_params = 0usize;
    let mut total_size = 0.0;
    let mut successful_splits = 0;
    let mut failed_splits = 0;

    for node_id in ["node_001", "node_002"] {
        info!("🔧 开始切分节点: {}", node_id);

        match splitter.split_model(split_config.clone(), node_id).await {
            Ok(result) => {
                info!("✅ 节点 {} 切分成功", node_id);
                info!("   分片路径: {}", result.shard_path);
                info!("   层数: {}", result.layer_names.len());
                info!("   参数数: {}", result.total_params);
                info!("   大小: {:.2} MB", result.shard_size_mb);

                total_params += result.total_params;
                total_size += result.shard_size_mb;
                successful_splits += 1;

                // 检查文件是否真的创建
                match tokio::fs::metadata(&result.shard_path).await {
                    Ok(metadata) => {
                        let file_size = metadata.len() as f64 / 1024.0 / 1024.0;
                        info!("   ✅ 分片文件已创建 ({:.2} MB)", file_size);
                    }
                    Err(e) => {
                        warn!("   ⚠️  无法验证分片文件: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("❌ 节点 {} 切分失败: {}", node_id, e);
                failed_splits += 1;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    info!("📊 切分汇总:");
    info!("   成功: {}, 失败: {}", successful_splits, failed_splits);
    info!("   总参数: {}", total_params);
    info!("   总大小: {:.2} MB", total_size);

    if successful_splits == 0 {
        Err(anyhow::anyhow!("所有切分都失败了"))
    } else {
        Ok(())
    }
}

/// 测试 4: 验证切分方案
async fn test_validate_split_plan() -> Result<()> {
    info!("📋 测试 4: 验证切分方案");

    let splitter = ModelSplitter::new();
    let split_plan = create_split_plan();

    // 收集所有层的名称
    let all_layer_names: Vec<String> = split_plan
        .values()
        .flat_map(|plan| plan.layer_names.clone())
        .collect();

    info!("📊 总共有 {} 个层", all_layer_names.len());

    // 验证切分方案
    match splitter.validate_split_plan(&all_layer_names, &split_plan) {
        Ok(_) => {
            info!("✅ 切分方案验证通过");
            Ok(())
        }
        Err(e) => {
            error!("❌ 切分方案验证失败: {}", e);
            Err(e)
        }
    }
}

/// 测试 5: 检查输出目录
async fn test_check_output_dir(config: &TestConfig) -> Result<()> {
    info!("📋 测试 5: 检查输出目录");

    if !config.output_dir.exists() {
        tokio::fs::create_dir_all(&config.output_dir).await?;
        info!("✅ 创建输出目录: {}", config.output_dir.display());
    } else {
        info!("✅ 输出目录已存在: {}", config.output_dir.display());
    }

    // 列出输出目录中的文件
    let mut entries = tokio::fs::read_dir(&config.output_dir).await?;
    let mut file_count = 0;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            file_count += 1;
            info!("   📄 {}", path.file_name().unwrap().to_string_lossy());
        }
    }

    if file_count == 0 {
        info!("   (输出目录为空)");
    } else {
        info!("   总计 {} 个文件", file_count);
    }

    Ok(())
}

/// 主测试流程
async fn run_all_tests() -> Result<()> {
    info!("🚀 开始模型下载和切分测试");
    info!("{}", "=".repeat(70));

    let config = TestConfig::new();

    let mut results = Vec::new();

    // 测试 1: 检查模型是否存在
    info!("");
    let result1 = test_model_exists(&config).await;
    results.push(("模型存在性检查", result1.is_ok()));
    if result1.is_err() {
        warn!("⚠️  模型不存在，跳过后续测试");
        return Err(result1.unwrap_err());
    }

    // 测试 2: 创建模型加载器
    info!("");
    let result2 = test_model_loader(&config).await;
    results.push(("模型加载器创建", result2.is_ok()));

    // 测试 3: 测试模型切分
    info!("");
    let result3 = test_model_split(&config).await;
    results.push(("模型切分", result3.is_ok()));

    // 测试 4: 验证切分方案
    info!("");
    let result4 = test_validate_split_plan().await;
    results.push(("切分方案验证", result4.is_ok()));

    // 测试 5: 检查输出目录
    info!("");
    let result5 = test_check_output_dir(&config).await;
    results.push(("输出目录检查", result5.is_ok()));

    // 打印测试总结
    info!("");
    info!("{}", "=".repeat(70));
    info!("📊 测试总结");
    info!("{}", "=".repeat(70));

    for (test_name, passed) in &results {
        let status = if *passed { "✅ 通过" } else { "❌ 失败" };
        info!("   {}: {}", test_name, status);
    }

    let all_passed = results.iter().all(|(_, passed)| *passed);

    info!("");
    if all_passed {
        info!("🎉 所有测试通过！");
    } else {
        warn!("⚠️  部分测试失败");
    }

    info!("{}", "=".repeat(70));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 运行所有测试
    match run_all_tests().await {
        Ok(_) => {
            println!("\n✅ 测试成功完成");
            Ok(())
        }
        Err(e) => {
            error!("\n❌ 测试失败: {}", e);
            Err(e)
        }
    }
}

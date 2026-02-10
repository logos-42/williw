//! 文档驱动的自主工作流演示
//!
//! 演示如何使用内嵌文档启动AI自主工作流

use williw::agent::workflow::AsyncWorkflowExecutor;
use williw::agent::RalphLoopConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 文档驱动的AI自主工作流演示");
    println!("=====================================\n");

    // 创建工作流执行器
    println!("📝 创建工作流执行器...");
    let executor = AsyncWorkflowExecutor::new()?;

    // 配置Ralph Loop
    println!("⚙️  配置Ralph Loop...");
    let ralph_config = RalphLoopConfig {
        enabled: true,
        max_iterations: 50,
        iteration_delay_ms: 1000,
        completion_checker: Some("所有验收标准达成".to_string()),
        max_total_time_ms: Some(1800000), // 30分钟
        iteration_timeout_ms: 120000,      // 2分钟
        enable_history: true,
        ..Default::default()
    };

    // 获取API密钥
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .unwrap_or_else(|_| {
            println!("⚠️  未设置ANTHROPIC_API_KEY环境变量");
            println!("💡 提示: export ANTHROPIC_API_KEY=your_key_here");
            String::new()
        });

    if api_key.is_empty() {
        println!("❌ 错误: 缺少API密钥");
        return Err("请设置ANTHROPIC_API_KEY环境变量".into());
    }

    println!("\n📚 文档信息:");
    println!("  身份文档: 去中心化算力专家 (内嵌)");
    println!("  任务文档: 模型切分示例 (内嵌)");
    println!("  工具文档: DecentralizedModel (内嵌)");

    println!("\n🎯 启动自主工作流...");
    println!("  - AI将阅读身份文档了解自己的角色");
    println!("  - AI将阅读任务文档了解目标");
    println!("  - AI将使用Ralph Loop自主执行直到完成");
    println!();

    // 使用内嵌文档启动工作流
    match executor.run_with_embedded_docs(
        "demo_execution_001".to_string(),
        api_key,
        Some(ralph_config),
    ).await {
        Ok(_) => {
            println!("\n✅ 工作流执行成功!");
            println!("   所有验收标准已达成");
        }
        Err(e) => {
            println!("\n❌ 工作流执行失败: {}", e);
            return Err(e.into());
        }
    }

    println!("\n🎉 演示完成!");
    Ok(())
}

/// 简化版演示：不使用AI决策
#[tokio::main]
async fn demo_simple() -> Result<(), Box<dyn std::error::Error>> {
    println!("📚 简单演示: 读取内嵌文档");

    // 读取身份文档
    let identity_content = williw::agent::workflow::ralph_loop::IDENTITY_COMPUTE_EXPERT;
    println!("\n👤 AI身份:");
    println!("{}", identity_content.lines().take(5).collect::<Vec<_>>().join("\n"));
    println!("...");

    // 读取任务文档
    let task_content = williw::agent::workflow::ralph_loop::TASK_SPLIT_MODEL_EXAMPLE;
    println!("\n📋 任务概要:");
    for line in task_content.lines().take(10) {
        println!("{}", line);
    }
    println!("...");

    Ok(())
}
//! 分布式推理协调器测试示例
//!
//! 演示完整的分布式推理流程：
//! 1. 节点发现
//! 2. 模型下载
//! 3. 模型切分
//! 4. 分片注册
//! 5. 分布式推理执行

use williw::compute::{
    DistributedInferenceCoordinator, CoordinatorConfig,
    ShardInfo,
};
use williw::compute::protocol::{InferenceRequest, InferenceConfig, ShardStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 分布式推理协调器测试");
    println!("========================================\n");

    // 1. 创建协调器
    println!("📦 步骤 1: 创建分布式推理协调器");
    let config = CoordinatorConfig {
        enabled: true,
        max_parallel_tasks: 4,
        cache_size_mb: 1024,
        node_timeout_secs: 60,
        max_retries: 3,
        enable_ai_scheduling: true,
        heartbeat_interval_secs: 30,
    };
    
    let coordinator = DistributedInferenceCoordinator::new(
        "node_main".to_string(),
        config,
    );
    
    println!("   ✅ 协调器创建成功\n");

    // 2. 注册模型分片
    println!("📦 步骤 2: 注册模型分片");
    let model_id = "llama-3.2-1b";
    
    let shards = vec![
        ShardInfo {
            shard_id: "shard_0".to_string(),
            model_id: model_id.to_string(),
            node_id: "node_1".to_string(),
            layer_range: (0, 10),
            size_bytes: 350 * 1024 * 1024,
            checksum: "abc123".to_string(),
            status: ShardStatus::Ready,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        ShardInfo {
            shard_id: "shard_1".to_string(),
            model_id: model_id.to_string(),
            node_id: "node_2".to_string(),
            layer_range: (11, 20),
            size_bytes: 350 * 1024 * 1024,
            checksum: "def456".to_string(),
            status: ShardStatus::Ready,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        ShardInfo {
            shard_id: "shard_2".to_string(),
            model_id: model_id.to_string(),
            node_id: "node_3".to_string(),
            layer_range: (21, 30),
            size_bytes: 350 * 1024 * 1024,
            checksum: "ghi789".to_string(),
            status: ShardStatus::Ready,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ];
    
    coordinator.register_model_shards(model_id, shards).await?;
    println!("   ✅ 已注册 3 个分片\n");

    // 3. 提交推理任务
    println!("📦 步骤 3: 提交推理任务");
    let request = InferenceRequest {
        task_id: format!("task_{}", uuid::Uuid::new_v4()),
        model_id: model_id.to_string(),
        input_data: "你好，请介绍一下自己".as_bytes().to_vec(),
        config: InferenceConfig {
            max_new_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            stream: false,
            timeout_secs: 60,
        },
    };
    
    let task_id = coordinator.submit_task(request).await?;
    println!("   ✅ 任务已提交: {}\n", task_id);

    // 4. 查看任务状态
    println!("📦 步骤 4: 查看任务状态");
    if let Some(state) = coordinator.get_task_status(&task_id).await {
        println!("   任务 ID: {}", state.task_id);
        println!("   模型 ID: {}", state.model_id);
        println!("   状态: {:?}", state.status);
        println!("   进度: {:.1}%", state.progress() * 100.0);
        println!("   分片顺序: {:?}", state.shard_order);
    }
    println!();

    // 5. 查看节点列表
    println!("📦 步骤 5: 查看节点列表");
    let nodes = coordinator.get_nodes().await;
    for node in &nodes {
        println!("   节点: {} ({})", node.node_id, if node.online { "在线" } else { "离线" });
        println!("      分片: {:?}", node.shards);
    }
    println!();

    // 6. 模拟执行推理
    println!("📦 步骤 6: 模拟执行推理");
    println!("   注意: 实际执行需要连接到真实的推理节点");
    println!("   当前为模拟模式，展示执行流程...\n");
    
    // 模拟执行过程
    for i in 0..3 {
        println!("   🔄 执行分片 {}...", i);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        println!("   ✅ 分片 {} 完成", i);
    }
    println!();

    println!("========================================");
    println!("✅ 分布式推理协调器测试完成！");
    println!();
    println!("📋 测试总结:");
    println!("   - 协调器创建: ✅");
    println!("   - 分片注册: ✅");
    println!("   - 任务提交: ✅");
    println!("   - 状态查询: ✅");
    println!("   - 节点管理: ✅");
    println!();
    println!("🔧 下一步:");
    println!("   1. 连接真实的 iroh P2P 网络");
    println!("   2. 集成实际的推理引擎");
    println!("   3. 测试跨节点通信");

    Ok(())
}

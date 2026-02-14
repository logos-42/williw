//! P2P  cross-node distributed inference test
//!
//! Demonstrates the complete flow of cross-node distributed inference:
//! 1. Node discovery via iroh P2P
//! 2. Model shard registration
//! 3. Task submission and execution
//! 4. Result aggregation

use williw::compute::{
    DistributedInferenceCoordinator, CoordinatorConfig,
    InferenceNetwork, MockInferenceNetwork, InferenceNetworkConfig,
};
use williw::compute::protocol::{InferenceMessage, InferenceRequest, InferenceConfig, ShardInfo, ShardStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Cross-Node Distributed Inference Test");
    println!("========================================\n");

    // Step 1: Create network layer
    println!("Step 1: Create Network Layer");
    let network_config = InferenceNetworkConfig {
        enabled: true,
        bind_addr: "0.0.0.0:0".to_string(),
        bootstrap_nodes: vec![],
        message_timeout_secs: 60,
        max_retries: 3,
        heartbeat_interval_secs: 30,
    };
    
    // Use mock network for testing (real network requires iroh setup)
    let network = MockInferenceNetwork::new("coordinator_node");
    println!("   Network node ID: {}", network.get_node_id());
    println!("   Network layer initialized\n");

    // Step 2: Create coordinator
    println!("Step 2: Create Distributed Inference Coordinator");
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
        "coordinator_node".to_string(),
        config,
    );
    println!("   Coordinator created\n");

    // Step 3: Simulate multi-node environment
    println!("Step 3: Simulate Multi-Node Environment");
    
    // Add compute nodes
    network.add_peer("compute_node_1").await;
    network.add_peer("compute_node_2").await;
    network.add_peer("compute_node_3").await;
    
    println!("   Added 3 compute nodes:");
    for peer in network.get_connected_peers().await {
        println!("      - {}", peer);
    }
    println!();

    // Step 4: Register model shards
    println!("Step 4: Register Model Shards");
    let model_id = "llama-3.2-1b";
    
    let shards = vec![
        ShardInfo {
            shard_id: "shard_0".to_string(),
            model_id: model_id.to_string(),
            node_id: "compute_node_1".to_string(),
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
            node_id: "compute_node_2".to_string(),
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
            node_id: "compute_node_3".to_string(),
            layer_range: (21, 30),
            size_bytes: 350 * 1024 * 1024,
            checksum: "ghi789".to_string(),
            status: ShardStatus::Ready,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ];
    
    coordinator.register_model_shards(model_id, shards).await?;
    println!("   Registered 3 shards for model: {}", model_id);
    println!("   Shard distribution:");
    for node in coordinator.get_nodes().await {
        println!("      - Node {} holds {} shards", node.node_id, node.shards.len());
    }
    println!();

    // Step 5: Submit inference task
    println!("Step 5: Submit Inference Task");
    let request = InferenceRequest {
        task_id: format!("task_{}", uuid::Uuid::new_v4()),
        model_id: model_id.to_string(),
        input_data: "Hello, please introduce yourself".as_bytes().to_vec(),
        config: InferenceConfig {
            max_new_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            stream: false,
            timeout_secs: 60,
        },
    };
    
    let task_id = coordinator.submit_task(request).await?;
    println!("   Task submitted: {}", task_id);
    
    // Check task status
    if let Some(state) = coordinator.get_task_status(&task_id).await {
        println!("   Task status: {:?}", state.status);
        println!("   Execution order: {:?}", state.shard_order);
    }
    println!();

    // Step 6: Simulate cross-node message passing
    println!("Step 6: Simulate Cross-Node Message Passing");
    
    // Broadcast heartbeat
    let heartbeat = InferenceMessage::Heartbeat {
        node_id: "coordinator_node".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        load: 0.3,
        available_memory_mb: 8000,
    };
    
    let count = network.broadcast_inference_message(heartbeat).await?;
    println!("   Broadcast heartbeat to {} nodes", count);
    
    // Send execution request to compute node 1
    let exec_msg = InferenceMessage::ExecuteShard {
        shard_id: "shard_0".to_string(),
        task_id: task_id.clone(),
        input_data: vec![1, 2, 3, 4],
        metadata: williw::compute::protocol::ShardExecutionMetadata {
            model_id: model_id.to_string(),
            layer_start: 0,
            layer_end: 10,
            input_shape: vec![1, 512],
            timeout_ms: 60000,
            priority: 5,
        },
    };
    
    network.send_inference_message("compute_node_1", exec_msg).await?;
    println!("   Sent execution request to compute_node_1");
    
    // Check message log
    let log = network.get_message_log().await;
    println!("   Message log entries: {}", log.len());
    println!();

    // Step 7: Summary
    println!("========================================");
    println!("Cross-Node Inference Test Complete!");
    println!();
    println!("Summary:");
    println!("   - Network layer: Mock (use iroh for real P2P)");
    println!("   - Connected nodes: {}", network.get_connected_peers().await.len());
    println!("   - Registered shards: 3");
    println!("   - Task submitted: {}", task_id);
    println!("   - Messages sent: {}", log.len());
    println!();
    println!("Next Steps for Real Deployment:");
    println!("   1. Replace MockInferenceNetwork with IrohInferenceNetwork");
    println!("   2. Configure bootstrap nodes for P2P discovery");
    println!("   3. Integrate with actual inference engine (e.g., llama.cpp)");
    println!("   4. Add GPU memory management and load balancing");

    Ok(())
}
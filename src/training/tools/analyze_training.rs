//! 训练结果分析工具
//!
//! 使用方法：
//! ```bash
//! cargo run --bin analyze_training -- --input test_output
//! ```

use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "analyze_training")]
#[command(about = "分析多节点训练结果")]
struct Args {
    /// 输入目录（包含所有节点的统计数据）
    #[arg(short, long, default_value = "test_output")]
    input: String,

    /// 输出报告文件
    #[arg(short, long, default_value = "training_report.json")]
    output: String,
}

#[derive(Debug, serde::Serialize)]
struct TrainingReport {
    total_nodes: usize,
    total_duration_secs: u64,
    total_interactions: u64,
    nodes: Vec<NodeReport>,
    summary: SummaryReport,
}

#[derive(Debug, serde::Serialize)]
struct NodeReport {
    node_id: String,
    tick_count: u64,
    sparse_updates_received: u64,
    dense_snapshots_received: u64,
    sparse_updates_sent: u64,
    dense_snapshots_sent: u64,
    connected_peers: usize,
    model_version: u64,
    model_hash: String,
}

#[derive(Debug, serde::Serialize)]
struct SummaryReport {
    avg_tick_count: f64,
    avg_interactions: f64,
    total_sparse_updates: u64,
    total_dense_snapshots: u64,
    avg_connected_peers: f64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("分析训练结果: {}", args.input);

    let input_dir = PathBuf::from(&args.input);
    if !input_dir.exists() {
        return Err(anyhow::anyhow!("输入目录不存在: {}", args.input));
    }

    let mut nodes = Vec::new();
    let mut total_interactions = 0u64;
    let mut total_sparse_updates = 0u64;
    let mut total_dense_snapshots = 0u64;
    let mut total_ticks = 0u64;
    let mut total_peers = 0usize;
    let mut max_duration = 0u64;

    // 读取所有节点的统计数据
    for entry in fs::read_dir(&input_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.ends_with("_stats.json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        let node_id = file_name.replace("_stats.json", "");
                        
                        let tick_count = json["tick_count"].as_u64().unwrap_or(0);
                        let sparse_received = json["sparse_updates_received"].as_u64().unwrap_or(0);
                        let dense_received = json["dense_snapshots_received"].as_u64().unwrap_or(0);
                        let sparse_sent = json["sparse_updates_sent"].as_u64().unwrap_or(0);
                        let dense_sent = json["dense_snapshots_sent"].as_u64().unwrap_or(0);
                        let connected_peers = json["connected_peers"].as_u64().unwrap_or(0) as usize;
                        let model_version = json["model_version"].as_u64().unwrap_or(0);
                        let model_hash = json["model_hash"].as_str().unwrap_or("").to_string();
                        let elapsed = json["start_time_secs"].as_u64().unwrap_or(0);

                        total_interactions += sparse_received + dense_received + sparse_sent + dense_sent;
                        total_sparse_updates += sparse_received + sparse_sent;
                        total_dense_snapshots += dense_received + dense_sent;
                        total_ticks += tick_count;
                        total_peers += connected_peers;
                        max_duration = max_duration.max(elapsed);

                        nodes.push(NodeReport {
                            node_id,
                            tick_count,
                            sparse_updates_received: sparse_received,
                            dense_snapshots_received: dense_received,
                            sparse_updates_sent: sparse_sent,
                            dense_snapshots_sent: dense_sent,
                            connected_peers,
                            model_version,
                            model_hash,
                        });
                    }
                }
            }
        }
    }

    let node_count = nodes.len();
    if node_count == 0 {
        return Err(anyhow::anyhow!("未找到任何统计数据文件"));
    }

    let report = TrainingReport {
        total_nodes: node_count,
        total_duration_secs: max_duration,
        total_interactions,
        nodes,
        summary: SummaryReport {
            avg_tick_count: total_ticks as f64 / node_count as f64,
            avg_interactions: total_interactions as f64 / node_count as f64,
            total_sparse_updates,
            total_dense_snapshots,
            avg_connected_peers: total_peers as f64 / node_count as f64,
        },
    };

    // 输出报告
    let report_json = serde_json::to_string_pretty(&report)?;
    fs::write(&args.output, report_json)?;

    println!("\n=== 训练分析报告 ===");
    println!("节点数量: {}", report.total_nodes);
    println!("训练时长: {} 秒", report.total_duration_secs);
    println!("总交互次数: {}", report.total_interactions);
    println!("平均 Tick 数: {:.2}", report.summary.avg_tick_count);
    println!("平均交互次数: {:.2}", report.summary.avg_interactions);
    println!("总稀疏更新: {}", report.summary.total_sparse_updates);
    println!("总密集快照: {}", report.summary.total_dense_snapshots);
    println!("平均连接节点数: {:.2}", report.summary.avg_connected_peers);
    println!("\n详细报告已保存到: {}", args.output);

    Ok(())
}


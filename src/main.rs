mod args;
mod agent;
mod comms;
mod config;
mod consensus;
mod crypto;
mod device;
#[cfg(feature = "ffi")]
mod ffi;
mod node;
mod stats;
mod topology;
mod training;
mod types;
mod ai_decision;

use crate::args::{get_stats_output, parse_args_and_build_config};
use crate::node::Node;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::Duration;



#[tokio::main]
async fn main() -> Result<()> {
    let config = parse_args_and_build_config();
    let node = Node::new(config).await?;

    // 如果指定了统计输出文件，设置定期导出
    if let Some(output_path) = get_stats_output() {
        let stats_path = std::path::PathBuf::from(&output_path);
        let stats_manager: Arc<std::sync::Mutex<crate::stats::TrainingStatsManager>> = Arc::clone(&node.stats);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = stats_manager.lock().unwrap().export_json_to_file(&stats_path) {
                    eprintln!("导出统计数据失败: {:?}", e);
                }
            }
        });
    }

    node.run().await
}


// 测试API连接的简单脚本
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 测试连接
    let client = reqwest::Client::new();
    let response = client
        .get("https://williw.sirazede725.workers.dev/api/health")
        .send()
        .await?;

    println!("Status: {}", response.status());
    println!("Response: {}", response.text().await?);
    
    Ok(())
}

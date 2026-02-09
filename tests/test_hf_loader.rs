//! 测试 Hugging Face 模型加载器的基本功能

use std::path::PathBuf;
use williw::training::{create_llama_32_1b_loader, HFModelConfig, LlamaModelLoader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试 Hugging Face 模型加载器...");
    
    // 测试创建加载器
    let cache_dir = PathBuf::from("./test_models");
    let loader = create_llama_32_1b_loader(Some(cache_dir))?;
    
    println!("✓ 成功创建 Llama 3.2 1B 模型加载器");
    
    // 测试配置
    let config = HFModelConfig {
        model_id: "meta-llama/Llama-3.2-1B".to_string(),
        revision: Some("main".to_string()),
        cache_dir: Some(PathBuf::from("./test_cache")),
        token: None,
    };
    
    println!("✓ 成功创建 HF 模型配置");
    
    // 测试模型加载器创建
    let test_loader = LlamaModelLoader::new(config)?;
    println!("✓ 成功创建自定义模型加载器");
    
    println!("所有基本测试通过!");
    Ok(())
}

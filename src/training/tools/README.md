# Hugging Face 模型加载器

这个模块提供了从 Hugging Face Hub 下载和加载 Llama 3.2 1B 模型的功能，并支持按层拆分为多个部分。

## 功能特性

- ✅ 自动下载 Llama 3.2 1B 模型
- ✅ 支持 safetensors 和 PyTorch 格式
- ✅ 按层拆分模型为多个部分
- ✅ JSON 格式保存拆分结果
- ✅ 异步操作支持
- ✅ 详细的日志记录

## 依赖要求

### Python 依赖
在使用之前，需要安装以下 Python 包：

```bash
pip install torch numpy huggingface_hub safetensors
```

### Rust 依赖
项目已包含必要的 Rust 依赖：
- `tokio` - 异步运行时
- `serde` - 序列化/反序列化
- `ndarray` - 数组处理
- `anyhow` - 错误处理
- `tracing` - 日志记录

## 使用方法

### 1. 基本使用

```rust
use williw::training::{create_llama_32_1b_loader, ModelPartition};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建模型加载器
    let cache_dir = PathBuf::from("./models/huggingface");
    let loader = create_llama_32_1b_loader(Some(cache_dir))?;
    
    // 加载并拆分模型
    let partitions = loader.load_and_split().await?;
    
    // 处理拆分结果
    for partition in partitions {
        println!("分区 {}: {} 层, {} 参数", 
                partition.part_id, 
                partition.layers.len(), 
                partition.total_params);
    }
    
    Ok(())
}
```

### 2. 自定义配置

```rust
use williw::training::{LlamaModelLoader, HFModelConfig};
use std::path::PathBuf;

let config = HFModelConfig {
    model_id: "meta-llama/Llama-3.2-1B".to_string(),
    revision: Some("main".to_string()),
    cache_dir: Some(PathBuf::from("./custom_cache")),
    token: Some("your_hf_token".to_string()), // 可选，用于私有模型
};

let loader = LlamaModelLoader::new(config)?;
let partitions = loader.load_and_split().await?;
```

### 3. 使用 Python 脚本

也可以直接使用 Python 脚本：

```bash
# 基本使用
python src/training/tools/hf_model_helper.py

# 自定义参数
python src/training/tools/hf_model_helper.py \
    --model-id "meta-llama/Llama-3.2-1B" \
    --cache-dir "./models" \
    --num-parts 2 \
    --output-dir "./output"
```

## 运行示例

### 检查依赖并运行演示

```bash
# 检查 Python 依赖
python -c "import torch, numpy, huggingface_hub, safetensors"

# 运行 Rust 示例
cargo run --example hf_model_demo
```

### 预期输出

```
INFO  开始 Hugging Face Llama 3.2 1B 模型演示
INFO  创建 Llama 3.2 1B 模型加载器...
INFO  检查 Python 依赖...
INFO  Python 版本: 3.9.0
INFO  ✓ torch 已安装
INFO  ✓ numpy 已安装
INFO  ✓ huggingface_hub 已安装
INFO  ✓ safetensors 已安装
INFO  开始加载和拆分模型...
INFO  开始下载 Llama 3.2 1B 模型...
INFO  模型下载成功: /path/to/models/meta-llama/Llama-3.2-1B
INFO  加载模型参数从: /path/to/models/meta-llama/Llama-3.2-1B
INFO  加载层: model.embed_tokens.weight, 形状: [128256, 2048], 参数数量: 262144000
INFO  加载层: model.layers.0.self_attn.q_proj.weight, 形状: [2048, 2048], 参数数量: 4194304
...
INFO  成功加载 24 个层
INFO  开始拆分模型为两部分...
INFO  模型总参数数量: 1234567890
INFO  模型拆分完成:
INFO    Part 0: 12 层, 617283945 参数
INFO    Part 1: 12 层, 617283945 参数
INFO  演示完成! 模型已成功拆分为 2 部分
```

## 数据结构

### ModelLayer
```rust
pub struct ModelLayer {
    pub name: String,        // 层名称
    pub layer_type: String,  // 层类型
    pub shape: Vec<usize>,    // 张量形状
    pub parameters: Array1<f32>, // 扁平化的参数
}
```

### ModelPartition
```rust
pub struct ModelPartition {
    pub part_id: usize,      // 分区 ID
    pub layers: Vec<ModelLayer>, // 包含的层
    pub total_params: usize,  // 总参数数量
}
```

## 文件结构

模型下载和拆分后的文件结构：

```
models/huggingface/
├── meta-llama/
│   └── Llama-3.2-1B/
│       ├── config.json
│       ├── model.safetensors
│       ├── tokenizer.json
│       ├── ...
│       ├── partition_0.json  # 第一部分模型
│       └── partition_1.json  # 第二部分模型
└── hf_model_helper.py
```

## 注意事项

1. **模型大小**: Llama 3.2 1B 模型约 2GB，确保有足够的磁盘空间
2. **网络**: 首次下载需要稳定的网络连接
3. **内存**: 加载完整模型需要足够的内存
4. **HF Token**: 某些模型可能需要 Hugging Face 访问令牌

## 错误处理

常见错误及解决方案：

- **Python 依赖缺失**: 运行 `pip install torch numpy huggingface_hub safetensors`
- **网络连接问题**: 检查网络连接或使用代理
- **磁盘空间不足**: 清理磁盘空间或更改缓存目录
- **内存不足**: 考虑使用更大的机器或分批处理

## 扩展功能

### 支持其他模型

```rust
// 创建其他模型的加载器
let config = HFModelConfig {
    model_id: "microsoft/DialoGPT-medium".to_string(),
    revision: None,
    cache_dir: None,
    token: None,
};
let loader = LlamaModelLoader::new(config)?;
```

### 自定义拆分策略

可以修改 `split_model_layers` 方法来实现不同的拆分策略，如按参数数量、按层类型等。

## 性能优化

1. **并行下载**: 利用 Rust 的异步特性
2. **缓存机制**: 避免重复下载
3. **内存优化**: 流式处理大型张量
4. **压缩存储**: 可选的参数压缩

## 故障排除

如果遇到问题，请检查：

1. Python 和 Rust 版本兼容性
2. 网络连接和防火墙设置
3. 磁盘空间和内存容量
4. Hugging Face 访问权限

更多详细信息请参考源代码注释。

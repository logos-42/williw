# metadata-generator

Rust 库，用于从 PyTorch 模型提取 state_dict 并生成包含每层算力评估的元数据。

## 功能

- 从 PyTorch 模型提取 state_dict
- 对每一层进行算力评估（基于层类型、参数数量、batch_size、sequence_length）
- 生成包含完整元数据的 JSON 文件
- 支持 Transformer 模型的特殊处理

## 安装

### 作为依赖使用

在 `Cargo.toml` 中添加：

```toml
[dependencies]
metadata-generator = { path = "../rust_modules/metadata-generator" }
# 或者从 git 仓库
# metadata-generator = { git = "https://github.com/williw/metadata-generator" }
```

### 本地开发

```bash
cd rust_modules/metadata-generator
cargo build
cargo test
```

**注意：** 需要安装 Python 3 和以下 Python 包：
- `torch`
- `transformers`
- `numpy`

```bash
pip install torch transformers numpy
```

## 使用示例

```rust
use metadata_generator::{MetadataGenerator, MetadataConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建生成器
    let generator = MetadataGenerator::new();
    
    // 配置参数
    let config = MetadataConfig {
        model_name: "meta-llama/Llama-3.2-1B-Instruct".to_string(),
        model_path: "./models_cache/meta-llama_Llama-3.2-1B-Instruct".to_string(),
        batch_size: 1,
        sequence_length: 512,
        node_id: Some("node_001".to_string()),
    };
    
    // 生成元数据
    let metadata = generator.generate_metadata(config).await?;
    
    // 保存到文件
    generator.save_metadata(&metadata, "./metadata.json").await?;
    
    println!("元数据生成完成！");
    println!("模型: {}", metadata.model_name);
    println!("总层数: {}", metadata.total_layers);
    println!("总算力需求: {:.2}", metadata.total_compute);
    
    Ok(())
}
```

## API 文档

### `MetadataGenerator`

主要的元数据生成器结构体。

#### `new() -> Self`

创建新的生成器实例。

#### `generate_metadata(config: MetadataConfig) -> Result<ModelMetadata>`

生成元数据。内部会调用 Python 脚本来提取 state_dict。

#### `save_metadata(metadata: &ModelMetadata, path: &str) -> Result<()>`

保存元数据到 JSON 文件。

#### `estimate_layer_compute(...) -> f64`

估算单层的算力需求（内部方法）。

### `MetadataConfig`

元数据生成配置。

- `model_name: String` - 模型名称
- `model_path: String` - 模型路径（已下载的模型目录）
- `batch_size: usize` - 批次大小（用于算力评估）
- `sequence_length: usize` - 序列长度（用于算力评估）
- `node_id: Option<String>` - 节点 ID（可选）

### `ModelMetadata`

模型元数据。

- `model_name: String` - 模型名称
- `model_type: String` - 模型类型（如 "transformer"）
- `batch_size: usize` - 批次大小
- `sequence_length: usize` - 序列长度
- `layers: Vec<LayerMetadata>` - 每层的元数据
- `total_compute: f64` - 总算力需求
- `total_layers: usize` - 总层数
- `generated_at: f64` - 生成时间戳
- `node_id: Option<String>` - 节点 ID

### `LayerMetadata`

单层元数据。

- `name: String` - 层名称
- `shape: Vec<usize>` - 层形状
- `num_params: usize` - 参数数量
- `compute_required: f64` - 算力需求
- `layer_type: String` - 层类型（attention、linear、conv2d 等）
- `dtype: String` - 数据类型

## 算力评估算法

算力评估基于以下因素：

1. **层类型**：不同类型的层有不同的算力成本
   - attention: 4.0
   - linear: 2.0
   - conv2d: 2.0
   - layernorm: 1.0
   - embedding: 0.5
   - activation: 0.1
   - pooling: 0.2

2. **参数数量**：参数越多，算力需求越高

3. **批次大小**：批次越大，算力需求越高

4. **序列长度**（Transformer 模型）：序列越长，算力需求越高

公式：
```
layer_compute = num_params × cost_per_param × batch_size × (sequence_length / 512)  # Transformer
layer_compute = num_params × cost_per_param × batch_size  # 其他模型
```

## 特性

- ✅ 支持所有 PyTorch 模型类型
- ✅ 自动识别层类型
- ✅ 精确的算力评估
- ✅ 支持 Transformer 模型的特殊处理
- ✅ 生成标准 JSON 格式

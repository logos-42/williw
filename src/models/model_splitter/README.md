# model-splitter

Rust 库，用于根据 Worker 的分配方案按算力切分模型。

## 功能

- 根据分配方案切分模型（按每层的算力切分）
- 提取指定节点的层并保存为分片
- 验证切分方案的完整性
- 支持按算力切分（每层都有算力评估）

## 安装

### 作为依赖使用

在 `Cargo.toml` 中添加：

```toml
[dependencies]
model-splitter = { path = "../rust_modules/model-splitter" }
# 或者从 git 仓库
# model-splitter = { git = "https://github.com/williw/model-splitter" }
```

### 本地开发

```bash
cd rust_modules/model-splitter
cargo build
cargo test
```

**注意：** 需要安装 Python 3 和以下 Python 包：
- `torch`
- `transformers`

```bash
pip install torch transformers
```

## 使用示例

```rust
use model_splitter::{ModelSplitter, SplitConfig, SplitPlan};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建切分器
    let splitter = ModelSplitter::new();
    
    // 构建切分方案（通常从 Worker 获取）
    let mut split_plan = HashMap::new();
    split_plan.insert(
        "node_001".to_string(),
        SplitPlan {
            node_id: "node_001".to_string(),
            layer_names: vec![
                "transformer.h.0.attn.q_proj.weight".to_string(),
                "transformer.h.0.attn.k_proj.weight".to_string(),
                "transformer.h.0.attn.v_proj.weight".to_string(),
            ],
            total_compute: 100.0,
            compute_utilization: 0.5,
        },
    );
    
    // 配置切分参数
    let config = SplitConfig {
        model_name: "meta-llama/Llama-3.2-1B-Instruct".to_string(),
        model_path: "./models_cache/meta-llama_Llama-3.2-1B-Instruct".to_string(),
        split_plan,
        output_dir: Some("./model_shards".to_string()),
    };
    
    // 切分模型（提取 node_001 的分片）
    let result = splitter.split_model(config, "node_001").await?;
    
    println!("切分完成！");
    println!("节点: {}", result.node_id);
    println!("分片路径: {}", result.shard_path);
    println!("层数: {}", result.layer_names.len());
    println!("参数数: {}", result.total_params);
    println!("大小: {:.2} MB", result.shard_size_mb);
    
    Ok(())
}
```

## API 文档

### `ModelSplitter`

主要的切分器结构体。

#### `new() -> Self`

创建新的切分器实例。

#### `split_model(config: SplitConfig, node_id: &str) -> Result<SplitResult>`

根据分配方案切分模型，提取指定节点的层。

#### `validate_split_plan(all_layers: &[String], split_plan: &HashMap<String, SplitPlan>) -> Result<()>`

验证切分方案的完整性（确保所有层都被分配）。

### `SplitConfig`

切分配置。

- `model_name: String` - 模型名称
- `model_path: String` - 模型路径（已下载的模型目录）
- `split_plan: HashMap<String, SplitPlan>` - 切分方案（节点 ID -> 分配计划）
- `output_dir: Option<String>` - 输出目录（可选，默认为 `./model_shards/{node_id}`）

### `SplitPlan`

单个节点的切分计划。

- `node_id: String` - 节点 ID
- `layer_names: Vec<String>` - 分配给该节点的层名称列表
- `total_compute: f64` - 该节点的总算力需求
- `compute_utilization: f64` - 算力利用率（0.0-1.0）

### `SplitResult`

切分结果。

- `node_id: String` - 节点 ID
- `shard_path: String` - 分片保存路径
- `layer_names: Vec<String>` - 分配的层名称列表
- `total_params: usize` - 总参数数量
- `shard_size_mb: f64` - 分片大小（MB）

## 切分流程

1. **加载模型**：从指定路径加载 PyTorch 模型
2. **提取 state_dict**：获取模型的 state_dict
3. **提取指定层的参数**：根据 `split_plan` 提取分配给该节点的层
4. **保存分片**：将提取的参数保存为 `.pth` 文件

## 特性

- ✅ 支持按算力切分（每层都有算力评估）
- ✅ 精确提取指定层的参数
- ✅ 验证切分方案的完整性
- ✅ 支持大模型切分
- ✅ 异步处理，不阻塞

## 注意事项

1. **模型路径**：确保模型已下载到指定路径
2. **切分方案**：切分方案必须由 Worker 的算法生成
3. **层名称**：层名称必须与模型中的实际层名称完全匹配
4. **内存使用**：大模型切分可能需要较多内存

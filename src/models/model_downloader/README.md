# model-downloader

Rust 库，用于从 Hugging Face 下载模型文件。

## 功能

- 从 Hugging Face 下载模型文件（safetensors、bin、config.json 等）
- 支持自定义缓存目录
- 支持 Hugging Face Token 认证
- 异步下载，支持大文件

## 安装

### 作为依赖使用

在 `Cargo.toml` 中添加：

```toml
[dependencies]
model-downloader = { path = "../rust_modules/model-downloader" }
# 或者从 git 仓库
# model-downloader = { git = "https://github.com/williw/model-downloader" }
```

### 本地开发

```bash
cd rust_modules/model-downloader
cargo build
cargo test
```

## 使用示例

```rust
use model_downloader::{ModelDownloader, DownloadConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建下载器
    let downloader = ModelDownloader::new(Some("your_hf_token".to_string()));
    
    // 配置下载参数
    let config = DownloadConfig {
        model_name: "meta-llama/Llama-3.2-1B-Instruct".to_string(),
        cache_dir: Some("./models_cache".to_string()),
        hf_token: Some("your_hf_token".to_string()),
    };
    
    // 下载模型
    let result = downloader.download_model(config).await?;
    
    println!("下载完成！");
    println!("模型路径: {}", result.model_path);
    println!("下载文件数: {}", result.files_downloaded.len());
    println!("总大小: {:.2} MB", result.total_size_mb);
    
    Ok(())
}
```

## API 文档

### `ModelDownloader`

主要的下载器结构体。

#### `new(hf_token: Option<String>) -> Self`

创建新的下载器实例。

#### `download_model(config: DownloadConfig) -> Result<DownloadResult>`

下载模型文件。

### `DownloadConfig`

下载配置。

- `model_name: String` - 模型名称（如 "meta-llama/Llama-3.2-1B-Instruct"）
- `cache_dir: Option<String>` - 缓存目录（可选，默认为 `./models_cache/{model_name}`）
- `hf_token: Option<String>` - Hugging Face Token（可选，用于私有模型）

### `DownloadResult`

下载结果。

- `model_path: String` - 模型保存路径
- `files_downloaded: Vec<String>` - 已下载的文件列表
- `total_size_mb: f64` - 总大小（MB）

## 特性

- ✅ 支持 safetensors 和 bin 格式
- ✅ 自动下载 config.json、tokenizer.json 等配置文件
- ✅ 支持断点续传（通过文件系统缓存）
- ✅ 异步下载，不阻塞
- ✅ 错误处理和重试机制

# metadata-uploader

Rust 库，用于将元数据 JSON 文件上传到 Hugging Face 公共仓库。

## 功能

- 上传元数据文件到 Hugging Face 仓库
- 支持自定义提交信息
- 检查元数据是否已存在
- 支持私有仓库（需要 Token）

## 安装

### 作为依赖使用

在 `Cargo.toml` 中添加：

```toml
[dependencies]
metadata-uploader = { path = "../rust_modules/metadata-uploader" }
# 或者从 git 仓库
# metadata-uploader = { git = "https://github.com/williw/metadata-uploader" }
```

### 本地开发

```bash
cd rust_modules/metadata-uploader
cargo build
cargo test
```

## 使用示例

```rust
use metadata_uploader::{MetadataUploader, UploadConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建上传器
    let uploader = MetadataUploader::new();
    
    // 配置上传参数
    let config = UploadConfig {
        metadata_file: "./metadata.json".to_string(),
        repo_id: "your-username/model-metadata".to_string(),
        hf_token: "your_hf_token".to_string(),
        commit_message: Some("Upload metadata for Llama-3.2-1B-Instruct".to_string()),
    };
    
    // 上传元数据
    let result = uploader.upload_metadata(config).await?;
    
    println!("上传成功！");
    println!("仓库: {}", result.repo);
    println!("文件名: {}", result.filename);
    println!("URL: {}", result.url);
    println!("提交 URL: {}", result.commit_url);
    
    Ok(())
}
```

## API 文档

### `MetadataUploader`

主要的上传器结构体。

#### `new() -> Self`

创建新的上传器实例。

#### `upload_metadata(config: UploadConfig) -> Result<UploadResult>`

上传元数据文件到 Hugging Face。

#### `check_metadata_exists(repo_id: &str, filename: &str, hf_token: Option<&str>) -> Result<bool>`

检查元数据是否已存在。

### `UploadConfig`

上传配置。

- `metadata_file: String` - 元数据文件路径
- `repo_id: String` - Hugging Face 仓库 ID（如 "username/repo-name"）
- `hf_token: String` - Hugging Face Token（必需）
- `commit_message: Option<String>` - 提交信息（可选）

### `UploadResult`

上传结果。

- `repo: String` - 仓库 ID
- `filename: String` - 文件名
- `url: String` - 文件 URL
- `commit_url: String` - 提交 URL

## 特性

- ✅ 支持公共和私有仓库
- ✅ 自动处理文件上传
- ✅ 支持自定义提交信息
- ✅ 错误处理和重试机制
- ✅ 检查文件是否已存在

## 注意事项

1. **Hugging Face Token**：需要有效的 HF Token 才能上传
2. **仓库权限**：确保 Token 有权限写入目标仓库
3. **文件格式**：只支持 JSON 格式的元数据文件
4. **文件大小**：建议文件大小不超过 100MB

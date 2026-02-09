# Rust 模块说明

本目录包含 4 个 Rust 模块，用于接口层集成。

## 模块列表

1. **model-downloader**: 从 Hugging Face 下载模型
2. **metadata-generator**: 提取 state_dict 生成元数据
3. **metadata-uploader**: 上传元数据到 Hugging Face
4. **model-splitter**: 按算力切分模型

## 构建和测试

### 构建单个模块

```bash
cd model_downloader
cargo build --release
```

### 运行测试

```bash
cd model_downloader
cargo test
```

### 构建所有模块

```bash
for dir in */; do
    cd "$dir"
    cargo build --release
    cd ..
done
```

## 使用方式

### 作为库使用

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
model-downloader = { path = "../rust_modules/model_downloader" }
metadata-generator = { path = "../rust_modules/metadata_generator" }
metadata-uploader = { path = "../rust_modules/metadata_uploader" }
model-splitter = { path = "../rust_modules/model_splitter" }
```

### 作为独立程序使用

每个模块可以编译为独立的可执行文件，通过 FFI 接口供其他语言调用。

## Python 脚本依赖

`metadata-generator` 和 `model-splitter` 模块需要调用 Python 脚本，确保已安装：

```bash
pip install transformers torch
```

## 注意事项

1. **Python 环境**: 确保系统中有 `python3` 命令可用
2. **网络访问**: 下载和上传模块需要网络访问 Hugging Face
3. **Token**: 上传模块需要有效的 Hugging Face Token

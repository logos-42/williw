# 项目存档指南

## 📦 存档方案选择

### 方案1: 推送到远程Git仓库（推荐）

如果已有GitHub/GitLab仓库：

```bash
cd "d:\AI\去中心化训练"

# 1. 添加所有更改
git add .

# 2. 提交更改
git commit -m "chore: archive project state - iroh auto-upload, GPU inference, Workers integration"

# 3. 推送到远程
git push origin master
```

**优点**: 
- 版本控制
- 易于协作
- 可以回滚

---

### 方案2: 创建ZIP压缩包（快速本地存档）

在项目根目录打开PowerShell，运行：

```powershell
# 创建压缩包（排除大文件）
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$archiveName = "williw-archive-$timestamp.zip"

Compress-Archive -Path @(
    "Cargo.toml", "README.md", "package.json", "requirements.txt",
    "src/", "src-tauri/", "examples/", "tests/", "docs/",
    "scripts/", "*.md", "*.py", "*.rs", "*.toml", "*.json", "*.ts"
) -DestinationPath "d:\AI\$archiveName" -Force

Write-Host "✅ 存档已创建: d:\AI\$archiveName"
```

**优点**:
- 快速简单
- 包含所有代码和配置
- 大文件已被.gitignore排除

---

### 方案3: 使用7-Zip创建高压缩率存档

如果安装了7-Zip：

```powershell
# 使用7-Zip创建存档（更高的压缩率）
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$source = "d:\AI\去中心化训练"
$destination = "d:\AI\williw-archive-$timestamp.7z"

# 排除大文件和临时文件
& "C:\Program Files\7-Zip\7z.exe" a -t7z -mx=9 `
  -xr!target/ -xr!test_models/ -xr!torch_env/ -xr!node_modules/ `
  -xr!android_lib_full/ -xr!*.log -xr!*.tmp `
  $destination $source

Write-Host "✅ 高压缩率存档已创建: $destination"
```

---

### 方案4: Git Bundle（包含完整Git历史）

```bash
cd "d:\AI\去中心化训练"
git bundle create d:\AI\williw-bundle.git HEAD master
```

**优点**:
- 包含完整Git历史
- 可以克隆到新的仓库
- 文件体积小

---

## 📊 当前项目状态

### 修改的文件（需要提交）
```
M  .gitignore                          # 已更新，排除大文件
M  src-tauri/Cargo.toml                # 新增log依赖
M  src-tauri/src/api_client.rs         # 新增iroh上传功能
M  src-tauri/src/commands.rs           # 新增命令
M  src-tauri/src/main.rs               # 新增自动上传任务
D  gpu_inference_server_clean.py       # 已删除
```

### 新增文件（未跟踪）
```
?? docs/IROH_NODE_UPLOAD.md           # iroh上传文档
?? docs/IROH_UPLOAD_SUMMARY.md         # 上传功能总结
?? docs/VERIFY_NODE_INFO.md            # 节点验证文档
?? simple_lfm_test.py                  # LFM模型测试
?? test_full_node_upload.py            # 完整上传测试
?? test_node_info_upload.py            # 节点上传测试
?? test_server_gpu.py                  # GPU服务器测试
?? test_with_mock.py                   # Mock测试
```

---

## 📝 快速存档命令（推荐）

在项目根目录打开PowerShell：

```powershell
# 一键存档脚本
Write-Host "🔍 检查Git状态..."
git status --short

Write-Host "`n📦 创建存档..."
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"

# 方案A: ZIP存档（推荐，兼容性好）
Compress-Archive -Path @(
    "Cargo.toml", "package.json", "requirements.txt",
    "src/", "src-tauri/", "examples/", "tests/", "docs/", "scripts/",
    "*.md", "*.py", "*.rs", "*.toml", "*.json", "*.ts"
) -DestinationPath "d:\AI\williw-archive-$timestamp.zip" -Force

Write-Host "`n✅ 存档完成！"
Write-Host "📁 位置: d:\AI\williw-archive-$timestamp.zip"
Write-Host "`n📂 包含内容:"
Write-Host "   - Rust后端代码 (src/, src-tauri/)"
Write-Host "   - Python脚本和测试"
Write-Host "   - 示例和文档"
Write-Host "   - 配置文件"
Write-Host "   - 排除: 大文件(target/, test_models/, node_modules/)"
```

---

## 🚀 执行存档

直接在PowerShell运行：

```powershell
cd "d:\AI\去中心化训练"

# 创建ZIP存档
$files = Get-ChildItem -Path . -File | Where-Object { $_.Extension -match '\.(toml|json|md|py|rs|ts|txt)$' -or $_.Name -match '^(README|Cargo|package|requirements|vite\.config|tsconfig)' }
$dirs = Get-ChildItem -Path . -Directory | Where-Object { $_.Name -match '^(src|src-tauri|examples|tests|docs|scripts)$' }

$items = @()
$items += $files.FullName
$items += $dirs.FullName

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
Compress-Archive -Path $items -DestinationPath "d:\AI\williw-archive-$timestamp.zip" -Force

Write-Host "✅ 存档创建完成: d:\AI\williw-archive-$timestamp.zip"
```

---

## 📂 存档内容清单

存档将包含：

### 核心代码
- ✅ Rust核心库 (`src/`)
- ✅ Tauri桌面应用 (`src-tauri/`)
- ✅ 示例代码 (`examples/`)
- ✅ 测试代码 (`tests/`)

### 配置和脚本
- ✅ Cargo.toml, package.json, requirements.txt
- ✅ 构建配置 (vite.config.ts, tsconfig.json)
- ✅ 脚本 (scripts/)

### 文档
- ✅ README.md
- ✅ docs/ (包含iroh上传、验证等文档)
- ✅ 测试文档

### 测试文件
- ✅ Python测试脚本
- ✅ GPU推理测试

### 排除的文件（大文件）
- ❌ target/ (Rust构建产物)
- ❌ test_models/ (AI模型文件)
- ❌ torch_env/ (Python虚拟环境)
- ❌ node_modules/ (Node.js依赖)
- ❌ android_lib_full/ (Android库)
- ❌ *.log (日志文件)

---

## 🎯 推荐操作

**立即执行**（复制到PowerShell）：

```powershell
# 1. 进入项目目录
cd "d:\AI\去中心化训练"

# 2. 创建ZIP存档
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
Compress-Archive -Path @(
    "Cargo.toml", "README.md", "package.json", "requirements.txt",
    "vite.config.ts", "tsconfig.json", "tsconfig.node.json",
    "src/", "src-tauri/", "examples/", "tests/", "docs/", "scripts/",
    "*.md", "*.py", "*.rs", "*.toml", "*.json", "*.ts", "*.txt"
) -DestinationPath "d:\AI\williw-archive-$timestamp.zip" -Force

# 3. 显示结果
Write-Host "`n✅ 存档创建完成！"
Write-Host "📁 位置: d:\AI\williw-archive-$timestamp.zip"
Write-Host "`n📊 存档大小: $([math]::Round((Get-Item "d:\AI\williw-archive-$timestamp.zip").Length / 1MB, 2)) MB"
```

或者使用7-Zip（如果已安装）：

```powershell
# 7-Zip高压缩率版本
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
& "C:\Program Files\7-Zip\7z.exe" a -t7z -mx=9 `
  -xr!target/ -xr!test_models/ -xr!torch_env/ -xr!node_modules/ `
  -xr!android_lib_full/ -xr!*.log -xr!*.tmp `
  "d:\AI\williw-archive-$timestamp.7z" .

Write-Host "✅ 高压缩率存档: d:\AI\williw-archive-$timestamp.7z"
```

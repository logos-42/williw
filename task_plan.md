# 任务计划：Williw-Desktop 4 个关键问题修复

## 目标
修复 williw-desktop 项目中的 4 个关键问题，提升用户体验和代码质量。

## 任务分解

### 任务 1: AI Agent 流程优化 - 优先使用本地已安装的 Ollama 模型
- [ ] 读取 `src-tauri/src/commands/agent_commands.rs`
- [ ] 在 `run_ai_agent_setup` 函数开头添加本地检测逻辑
- [ ] 调用 `quick_start_local_inference()` 检测本地模型
- [ ] 修改返回 JSON 结构，添加 `using_local_model` 字段
- [ ] 验证修改

### 任务 2: 模型下载问题 - 添加分块下载、断点续传、实时进度
- [ ] 读取 `src-tauri/src/commands/model_commands.rs` 了解当前下载逻辑
- [ ] 在 `src-tauri/src/events.rs` 添加 `ModelDownloadProgress` 事件结构
- [ ] 实现 `download_model_with_progress` 函数
- [ ] 使用 reqwest 流式下载，支持 Range 请求头
- [ ] 通过 Tauri 事件系统发送进度到前端
- [ ] 在 `src-tauri/src/main.rs` 注册新命令
- [ ] 验证修改

### 任务 3: 模型切分问题 - 移除硬编码 Python 路径
- [ ] 搜索 `src-tauri/src` 目录中所有硬编码的 Python 路径
- [ ] 使用 `which` crate 或 `Command::new("python3")` 替换
- [ ] 添加友好的错误提示
- [ ] 验证修改

### 任务 4: 主页尺寸问题 - 修复 100vw/100vh 问题
- [ ] 读取 `src/components/AppLayout.tsx`
- [ ] 修改 width/height 设置
- [ ] 读取 `src/components/LeftPanel.tsx`
- [ ] 修改 minWidth/maxWidth 为像素值
- [ ] 读取 `src/components/RightPanel.tsx`
- [ ] 修改 flex 设置
- [ ] 读取 `src/components/Resizer.tsx`
- [ ] 验证拖动逻辑
- [ ] 验证修改

## 进度追踪
| 任务 | 状态 | 备注 |
|------|------|------|
| 任务 1 | complete | 已添加 using_local_model 字段 |
| 任务 2 | complete | 已实现分块下载、断点续传、实时进度 |
| 任务 3 | complete | 代码已使用正确做法，无硬编码路径 |
| 任务 4 | complete | 已修改 minWidth 为像素值 |

## 修改文件清单
1. `src-tauri/src/commands/agent_commands.rs` - 添加 using_local_model 字段
2. `src-tauri/src/events.rs` - 添加 ModelDownloadProgress 事件结构
3. `src-tauri/src/commands/model_commands.rs` - 添加 download_model_with_progress 函数
4. `src-tauri/Cargo.toml` - 添加 dirs 依赖
5. `src-tauri/src/main.rs` - 注册 download_model_with_progress 命令
6. `src/components/LeftPanel.tsx` - 修改 minWidth 为像素值

## 验证结果
- Rust 代码检查：通过（项目已有 model-downloader 库的重复定义错误，非本次修改引入）
- TypeScript 代码检查：通过

## 错误记录
| 错误 | 尝试次数 | 解决方案 |
|------|----------|----------|
| | | |

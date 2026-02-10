# 文档驱动AI工作流使用说明

## 功能概述

桌面端应用现在支持在首次运行时自动启动AI自主工作流，实现以下功能：

1. **自动启动工作流**：首次点击"运行"按钮时，自动启动文档驱动的AI工作流
2. **流式响应显示**：工作流执行过程中，右侧对话框实时显示进度和消息
3. **自动配置算力**：AI自主阅读文档，理解任务，配置去中心化算力网络
4. **无缝过渡**：工作流完成后，自动进入正常对话模式

## 使用流程

### 1. 启动应用

```bash
# 开发模式
npm run tauri dev

# 生产构建
npm run build
npm run tauri build
```

### 2. 首次运行

1. 点击左上角的**训练开关**（Toggle按钮）
2. 应用会自动：
   - 检查是否首次运行
   - 启动文档驱动工作流
   - 右侧对话框显示工作流进度

### 3. 工作流执行

工作流包含以下步骤：

1. 📖 阅读AI身份文档（去中心化算力专家）
2. 📋 理解任务目标（模型切分和部署）
3. 🔍 分析模型结构
4. 🌐 连接去中心化算力网络
5. ⚙️ 配置算力节点
6. ✂️ 切分模型分片
7. 📤 分发模型分片
8. ✅ 验证分片完整性
9. 🚀 启动推理服务

### 4. 开始对话

工作流完成后，您可以在右侧对话框中：
- 直接与AI模型对话
- 使用去中心化算力执行推理任务
- 监控算力节点状态

## 技术架构

### 后端 (Tauri/Rust)

**新增文件：**
- `src-tauri/src/commands.rs` - 添加 `start_document_driven_workflow` 和 `get_workflow_status` 命令
- `src-tauri/src/state.rs` - 添加 `WorkflowStatus` 结构体

**关键功能：**
```rust
// 启动文档驱动工作流
#[tauri::command]
pub async fn start_document_driven_workflow(
    api_key: String,
    model_path: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String>

// 发送工作流消息事件
app_handle.emit("workflow-message", serde_json::json!({
    "type": "progress",
    "content": "正在配置算力节点...",
    "progress": 0.5,
}))

// 发送工作流状态事件
app_handle.emit("workflow-status", status)
```

### 前端 (React/TypeScript)

**新增文件：**
- `src/store/workflowStore.ts` - 工作流状态管理
- `src/types/index.ts` - 添加 `WorkflowStatus` 类型

**修改文件：**
- `src/components/TrainingSwitch.tsx` - 首次运行时启动工作流
- `src/components/ChatBox.tsx` - 监听和显示工作流消息

**关键功能：**
```typescript
// 监听工作流消息
useEffect(() => {
  const unlistenFn = await listen('workflow-message', (event) => {
    const { type, content, progress } = event.payload;
    // 添加消息到聊天框
    addMessage({ type, content, timestamp: new Date() });
  });
  return () => unlistenFn();
}, []);

// 启动工作流
await invoke('start_document_driven_workflow', {
  apiKey: '',
  modelPath: selectedModel,
});
```

## 文档驱动的核心概念

### 1. AI身份定义

AI通过阅读文档理解自己的角色：

```
角色：去中心化算力专家
专业领域：
  - 模型切分和分片
  - 算力节点管理
  - P2P网络协调
  - 任务调度分发
  - 模型聚合
```

### 2. 任务定义

AI通过阅读文档理解任务目标：

```
目标：完成模型切分并确保所有分片正确分发
验收标准：
  - 模型被切分为N个分片
  - 所有分片完整且可验证
  - 每个分片已分配到目标节点
```

### 3. 工具使用

AI通过阅读工具文档，自主使用工具完成任务

## 配置选项

### 工作流配置

在 `src/agent/workflow/ralph_loop/document_driven.rs` 中可以配置：

```rust
DocumentDrivenConfig {
    use_embedded_docs: true,  // 使用内嵌文档
    enable_doc_reading: true,  // 启用文档阅读
    re_read_docs_per_iteration: false,  // 每次迭代重新阅读文档
}
```

### Ralph Loop配置

```rust
RalphLoopConfig {
    max_iterations: 50,  // 最大迭代次数
    iteration_delay_ms: 1000,  // 迭代延迟
    enable_history: true,  // 启用历史记录
}
```

## 故障排除

### 工作流未启动

1. 检查是否首次运行（清除`isFirstTime`状态）
2. 查看控制台日志确认命令执行
3. 确认Tauri命令注册正确

### 消息未显示

1. 确认事件监听器正确注册
2. 检查消息格式是否正确
3. 查看浏览器控制台错误

### 类型错误

确保：
- Rust后端类型与前端TypeScript类型匹配
- 事件payload格式正确
- 序列化/反序列化正常工作

## 未来扩展

### 可能的功能

1. **多任务工作流**：支持同时运行多个工作流
2. **工作流模板**：预设不同的工作流模板
3. **可视化编辑器**：图形化编辑工作流
4. **工作流调试**：单步调试工作流执行
5. **性能监控**：实时监控工作流性能指标

### 扩展点

1. **自定义文档**：添加更多AI身份和任务文档
2. **自定义工具**：集成更多工具供AI使用
3. **自定义消息**：自定义工作流消息类型和显示

## 相关文档

- [快速开始](./QUICK_START.md)
- [文档驱动工作流](./DOCUMENT_DRIVEN_WORKFLOW.md)
- [自主循环流程](./AUTONOMOUS_LOOP_FLOW.md)

## 贡献

欢迎提交Issue和Pull Request！

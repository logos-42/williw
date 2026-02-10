# 内嵌文档目录

本目录包含文档驱动的AI自主工作流所需的所有文档。

## 目录结构

```
docs/
├── README.md                 # 本文件
├── agents/                   # AI身份文档
│   └── compute_expert.md    # 去中心化算力专家身份
├── tasks/                    # 任务文档
│   └── split_model_example.md  # 模型切分任务示例
└── tools/                    # 工具使用指南
    └── DecentralizedModel.md    # DecentralizedModel工具文档
```

## 如何使用

### 方式1: 直接使用内嵌文档（推荐）

```rust
use williw::agent::workflow::AsyncWorkflowExecutor;

let executor = AsyncWorkflowExecutor::new()?;

// 使用默认内嵌文档
executor.run_with_embedded_docs(
    "execution_id".to_string(),
    api_key,
    None,  // 使用默认Ralph Loop配置
).await?;
```

### 方式2: 使用自定义文档

```rust
use williw::agent::workflow::ralph_loop::DocumentDrivenConfig;

let config = DocumentDrivenConfig {
    use_embedded_docs: false,  // 使用外部文档
    identity_doc_path: Some("path/to/your/agent.md".to_string()),
    task_doc_path: Some("path/to/your/task.md".to_string()),
    ..Default::default()
};

executor.run_document_driven_workflow(
    "execution_id".to_string(),
    config,
    api_key,
    ralph_config,
).await?;
```

## 添加新的文档

### 添加新身份

1. 在 `agents/` 目录下创建新的 `.md` 文件
2. 按照以下模板编写：

```markdown
# 身份名称

## 角色
描述AI的角色

## 专业领域
- 领域1
- 领域2

## 工作原则
- 原则1
- 原则2

## 行为准则
- 准则1
- 准则2

## 核心工具
- 工具1
- 工具2
```

3. 在 `mod.rs` 中添加常量：
```rust
pub const IDENTITY_YOUR_AGENT: &str = include_str!("agents/your_agent.md");
```

### 添加新任务

1. 在 `tasks/` 目录下创建新的 `.md` 文件
2. 按照以下模板编写：

```markdown
# 任务名称

## 目标
任务目标

## 描述
任务描述

## 验收标准
- [ ] 标准1
- [ ] 标准2

## 执行步骤
1. 步骤1
2. 步骤2

## 约束条件
- 约束1
- 约束2
```

3. 在 `mod.rs` 中添加常量：
```rust
pub const TASK_YOUR_TASK: &str = include_str!("tasks/your_task.md");
```

### 添加新工具文档

1. 在 `tools/` 目录下创建新的 `.md` 文件
2. 描述工具的用途、参数、示例

## 内嵌文档的优势

1. **无需外部文件**: 打包到二进制中，运行时无需访问文件系统
2. **版本一致**: 文档版本与代码版本同步
3. **简化部署**: 只需分发一个二进制文件
4. **类型安全**: 编译时检查文档是否存在
5. **性能优化**: 文档在编译时处理，运行时无IO开销

## 参考资源

- 完整使用指南: `/docs/DOCUMENT_DRIVEN_WORKFLOW.md`
- 代码示例: `/examples/document_driven_demo.rs`
- 模块文档: `/src/agent/workflow/ralph_loop/document_driven.rs`
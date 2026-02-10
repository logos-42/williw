# 文档驱动的AI自主工作流

## 核心理念

> **人写文档，AI读文档，AI用工具，形成闭环**

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  人写文档    │────▶│  AI读文档   │────▶│  AI用工具   │
│  (身份/任务) │     │  (理解目标)  │     │  (执行)     │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                                │
                                                ▼
                                         ┌─────────────┐
                                         │  Ralph Loop │
                                         │  (闭环驱动)  │
                                         └──────┬──────┘
                                                │
                         ┌──────────────────────┘
                         ▼
                  ┌─────────────┐
                  │  完成/反馈   │
                  └─────────────┘
```

## 文档内嵌

> **重要**: 所有文档现在都内嵌在代码库中 (`src/agent/workflow/ralph_loop/docs/`)

内嵌的文档在编译时被打包到二进制中，无需外部文件即可运行。

## 文档类型

### 1. 身份文档 (`docs/agents/*.md`)
定义AI的角色、专业领域、工作原则和行为准则。

**关键要素**:
- **角色**：AI是谁，负责什么
- **专业领域**：擅长什么
- **工作原则**：做事的准则
- **行为准则**：具体的操作规范
- **核心工具**：可以使用的工具

**示例**: `docs/agents/compute_expert.md`

### 2. 任务文档 (`docs/tasks/*.md`)
定义具体任务的目标、步骤、验收标准。

**关键要素**:
- **目标**：要达成什么
- **描述**：任务背景
- **验收标准**：完成的标准（复选框形式）
- **执行步骤**：具体的操作步骤
- **约束条件**：时间、资源等限制

**示例**: `docs/tasks/split_model_example.md`

### 3. 工具文档 (`docs/tools/*.md`)
每个工具的详细使用说明。

**关键要素**:
- **用途**：工具能做什么
- **参数**：每个操作的参数说明
- **示例**：具体的调用示例
- **最佳实践**：使用建议

**示例**: `docs/tools/DecentralizedModel.md`

## 快速开始

### 方式1: 使用内嵌文档（推荐）

最简单的方式，直接使用代码库中内嵌的文档：

```rust
use williw::agent::workflow::AsyncWorkflowExecutor;
use williw::agent::workflow::ralph_loop::RalphLoopConfig;

#[tokio::main]
async fn main() -> Result<(), String> {
    // 创建执行器
    let executor = AsyncWorkflowExecutor::new()?;

    // Ralph Loop配置
    let ralph_config = RalphLoopConfig {
        enabled: true,
        max_iterations: 50,
        iteration_delay_ms: 1000,
        completion_checker: Some("所有验收标准达成".to_string()),
        ..Default::default()
    };

    // 直接使用内嵌文档启动
    executor.run_with_embedded_docs(
        "execution_001".to_string(),
        std::env::var("ANTHROPIC_API_KEY").unwrap(),
        Some(ralph_config),
    ).await?;

    Ok(())
}
```

**内嵌的默认文档**:
- 身份: `src/agent/workflow/ralph_loop/docs/agents/compute_expert.md`
- 任务: `src/agent/workflow/ralph_loop/docs/tasks/split_model_example.md`
- 工具: `src/agent/workflow/ralph_loop/docs/tools/DecentralizedModel.md`

### 方式2: 使用自定义文档

如果需要使用自定义文档，可以创建自己的文档文件：

#### 步骤1: 创建身份文档

创建自定义身份文档 `docs/agents/my_agent.md`:

```markdown
# 你的AI身份名称

## 角色
描述AI的角色和职责

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

#### 步骤2: 创建任务文档

创建自定义任务文档 `docs/tasks/my_task.md`:

```markdown
# 任务名称

## 目标
任务要达到的目标

## 描述
任务背景描述

## 验收标准
- [ ] 标准1
- [ ] 标准2
- [ ] 标准3

## 执行步骤
1. 步骤1
2. 步骤2
3. 步骤3

## 约束条件
- 时间限制
- 资源限制
```

#### 步骤3: 启动自主工作流

```rust
use williw::agent::workflow::AsyncWorkflowExecutor;
use williw::agent::workflow::ralph_loop::{DocumentDrivenConfig, RalphLoopConfig};

#[tokio::main]
async fn main() -> Result<(), String> {
    let executor = AsyncWorkflowExecutor::new()?;

    // 配置文档驱动工作流
    let doc_config = DocumentDrivenConfig {
        use_embedded_docs: false,  // 使用外部文档
        identity_doc_path: Some("docs/agents/my_agent.md".to_string()),
        task_doc_path: Some("docs/tasks/my_task.md".to_string()),
        enable_doc_reading: true,
        ..Default::default()
    };

    let ralph_config = RalphLoopConfig {
        enabled: true,
        max_iterations: 50,
        iteration_delay_ms: 1000,
        completion_checker: Some("所有验收标准达成".to_string()),
        ..Default::default()
    };

    executor.run_document_driven_workflow(
        "execution_001".to_string(),
        doc_config,
        std::env::var("ANTHROPIC_API_KEY").unwrap(),
        ralph_config,
    ).await?;

    Ok(())
}
```

## 工作原理

### 1. 文档阅读阶段
AI首先阅读身份文档和任务文档：
- 了解"我是谁"（身份）
- 了解"要做什么"（任务）
- 了解"怎么做"（步骤）
- 了解"怎么算完成"（验收标准）

### 2. 计划生成阶段
基于任务文档中的步骤，生成执行计划：
- 将步骤转换为工作流
- 确定依赖关系
- 识别可并行操作

### 3. Ralph Loop执行阶段
进入闭环执行：
```
执行步骤 → 检查结果 → AI决策 → 执行下一步 → ...
    ↑                                    │
    └──────────── 循环直到完成 ────────────┘
```

AI在每个迭代中：
1. 执行当前步骤
2. 验证结果
3. 检查是否达成验收标准
4. 决定下一步行动（继续/重试/调整/完成）

### 4. 完成判断
当所有验收标准都达成时：
- AI通过检查文档中的复选框判断完成
- Ralph Loop正常退出
- 生成执行报告

## 高级用法

### 动态任务调整

任务文档中可以包含条件逻辑：

```markdown
## 条件分支
- 如果节点数 > 10:
  - 使用分层分发策略
- 否则:
  - 使用直接分发策略
```

AI会根据实际情况选择不同路径。

### 工具链组合

在步骤中指定工具链：

```markdown
## 执行步骤
1. 分析模型 (工具: DecentralizedModel::Analyze)
2. 切分模型 (工具: DecentralizedModel::Split)
3. 分发分片 (工具: DecentralizedModel::Transfer)
4. 验证结果 (工具: DecentralizedModel::Verify)
```

### 失败重试策略

在任务文档中指定：

```markdown
## 故障处理
- **传输失败**: 重试3次，指数退避
- **节点离线**: 跳过并记录，继续其他节点
- **校验失败**: 重新传输该分片
```

## 最佳实践

### 1. 文档清晰原则
- 使用明确的动词开头（"下载"、"切分"、"验证"）
- 验收标准用复选框 `- [ ]` 表示
- 每个步骤有明确的验证条件

### 2. 身份定义原则
- 专业领域要具体，不要太宽泛
- 行为准则要可操作
- 核心工具列表要完整

### 3. 任务设计原则
- 步骤粒度适中（5-15个步骤）
- 验收标准可量化（"4个分片"而不是"多个分片"）
- 包含错误处理说明

### 4. 调试技巧
- 设置 `max_iterations` 防止无限循环
- 使用 `completion_checker` 明确完成条件
- 开启日志查看AI决策过程

## 示例场景

### 场景1: 模型切分分发
```
身份: 去中心化算力专家
任务: 切分LLM模型并分发到4个节点
结果: AI自动完成切分、传输、验证
```

### 场景2: 代码重构
```
身份: 代码重构专家
任务: 将utils.rs中的重复代码提取为通用函数
结果: AI自动分析、重构、验证
```

### 场景3: 文档生成
```
身份: 技术文档专家
任务: 为API生成文档并添加示例
结果: AI自动阅读代码、生成文档
```

## 故障排除

### AI不按照文档执行
- 检查文档格式是否正确
- 确认验收标准是否明确
- 增加更多行为准则约束

### 循环不终止
- 检查验收标准是否可达成
- 调整 `completion_checker`
- 限制 `max_iterations`

### 工具调用失败
- 检查工具文档
- 确认工具已注册
- 查看工具权限设置

## 扩展开发

### 添加新的身份
1. 创建 `docs/agents/new_identity.md`
2. 按照模板填写信息
3. 在代码中引用

### 添加新的任务类型
1. 创建 `docs/tasks/new_task.md`
2. 定义验收标准和步骤
3. 使用 `run_document_driven_workflow` 执行

### 自定义完成检查
```rust
let ralph_config = RalphLoopConfig {
    completion_checker: Some("file:exists:/path/to/COMPLETED".to_string()),
    ..Default::default()
};
```

## 总结

文档驱动的AI自主工作流将传统编程转变为"文档编程"：
- **人负责**: 写清楚目标、约束、验收标准
- **AI负责**: 理解文档、规划步骤、执行工具、自我检查
- **系统负责**: 循环驱动、历史记录、故障恢复

这种模式让AI真正成为执行主体，人只需要定义"做什么"和"怎么算完成"，AI自己决定"怎么做"。
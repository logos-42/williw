# 快速开始：文档驱动的AI自主工作流

## 最简方式（3步启动）

### 步骤1: 准备API密钥

```bash
export ANTHROPIC_API_KEY=your_api_key_here
```

### 步骤2: 运行示例

```bash
cargo run --example document_driven_demo
```

### 步骤3: 观察执行

你会看到：
```
🚀 文档驱动的AI自主工作流演示
=====================================

📚 文档信息:
  身份文档: 去中心化算力专家 (内嵌)
  任务文档: 模型切分示例 (内嵌)
  工具文档: DecentralizedModel (内嵌)

🎯 启动自主工作流...
  - AI将阅读身份文档了解自己的角色
  - AI将阅读任务文档了解目标
  - AI将使用Ralph Loop自主执行直到完成
```

---

## 在代码中使用

### 最简单的调用

```rust
use williw::agent::workflow::AsyncWorkflowExecutor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = AsyncWorkflowExecutor::new()?;

    executor.run_with_embedded_docs(
        "my_execution".to_string(),
        std::env::var("ANTHROPIC_API_KEY")?,
        None,  // 使用默认Ralph Loop配置
    ).await?;

    Ok(())
}
```

### 自定义配置

```rust
use williw::agent::workflow::AsyncWorkflowExecutor;
use williw::agent::workflow::ralph_loop::{DocumentDrivenConfig, RalphLoopConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = AsyncWorkflowExecutor::new()?;

    // 自定义Ralph Loop配置
    let ralph_config = RalphLoopConfig {
        max_iterations: 100,           // 最多100次迭代
        iteration_delay_ms: 500,         // 每次迭代延迟500ms
        completion_checker: Some("所有验收标准达成".to_string()),
        max_total_time_ms: Some(3600000), // 最多1小时
        ..Default::default()
    };

    // 使用默认内嵌文档
    executor.run_with_embedded_docs(
        "my_execution".to_string(),
        std::env::var("ANTHROPIC_API_KEY")?,
        Some(ralph_config),
    ).await?;

    Ok(())
}
```

### 使用自定义文档

```rust
let config = DocumentDrivenConfig {
    use_embedded_docs: false,  // 不使用内嵌文档
    identity_doc_path: Some("docs/agents/my_agent.md".to_string()),
    task_doc_path: Some("docs/tasks/my_task.md".to_string()),
    ..Default::default()
};

executor.run_document_driven_workflow(
    "my_execution".to_string(),
    config,
    api_key,
    ralph_config,
).await?;
```

---

## 内嵌的默认文档

### 1. 去中心化算力专家身份

**位置**: `src/agent/workflow/ralph_loop/docs/agents/compute_expert.md`

**内容预览**:
```markdown
# 去中心化算力专家

## 角色
去中心化算力网络的专业工程师

## 专业领域
- 模型切分和分片
- 算力节点管理
- P2P网络协调
- 任务调度分发

## 工作原则
- 切分粒度适中
- 优先选择低延迟节点
- 保持分片一致性
- 失败时自动重试
```

### 2. 模型切分任务

**位置**: `src/agent/workflow/ralph_loop/docs/tasks/split_model_example.md`

**验收标准**:
- [ ] 模型被切分为4个分片
- [ ] 所有分片完整且可验证
- [ ] 每个分片已分配到目标节点
- [ ] 分片校验和验证通过

### 3. DecentralizedModel工具文档

**位置**: `src/agent/workflow/ralph_loop/docs/tools/DecentralizedModel.md`

**可用操作**:
- Download: 下载模型
- Split: 切分模型
- Transfer: 传输分片
- Communicate: 节点通信
- FullPipeline: 完整流程

---

## 测试系统

运行测试脚本：

```bash
bash scripts/test_document_driven.sh
```

测试内容：
- ✅ 文件结构检查
- ✅ 文档内容验证
- ✅ 编译测试
- ✅ 代码集成验证

---

## 工作原理

```
┌─────────┐    ┌─────────┐    ┌─────────┐
│ 写文档   │───▶│ 读文档   │───▶│ AI决策   │
│  (人)   │    │  (AI)   │    │  (AI)   │
└─────────┘    └─────────┘    └────┬────┘
                                  │
                                  ▼
                           ┌─────────┐
                           │ 用工具   │
                           │  (执行)  │
                           └────┬────┘
                                │
                                ▼
                           ┌─────────┐
                           │ 检查结果 │
                           │ (验收)   │
                           └────┬────┘
                                │
                      ┌─────────┴─────────┐
                      │ 达成?             │
                      ├─否─▶ 继续循环      │
                      └─是─▶ 完成输出      │
```

**特点**:
- 🎯 目标驱动：基于任务文档的验收标准
- 📚 文档驱动：AI从文档学习角色和流程
- 🔄 闭环执行：Ralph Loop自动迭代直到完成
- 🤖 AI自主：不需要人工干预

---

## 常见问题

### Q: 如何添加自己的任务文档？

A: 创建Markdown文件，按以下结构编写：

```markdown
# 任务名称

## 目标
描述目标

## 验收标准
- [ ] 标准1
- [ ] 标准2

## 执行步骤
1. 步骤1
2. 步骤2
```

然后引用：
```rust
let config = DocumentDrivenConfig {
    task_doc_path: Some("your_task.md".to_string()),
    ..Default::default()
};
```

### Q: 如何修改AI的身份？

A: 修改或创建身份文档：

```markdown
# 新身份

## 角色
描述角色

## 专业领域
- 领域1
- 领域2

## 行为准则
- 规则1
- 规则2

## 核心工具
- 工具1
- 工具2
```

### Q: 循环不停止怎么办？

A: 检查以下几点：
1. 验收标准是否可达成
2. `completion_checker` 是否设置正确
3. `max_iterations` 是否有限制
4. 日志中AI的决策是什么

### Q: 如何调试执行过程？

A: 查看日志输出：
```bash
RUST_LOG=williw=debug,agent=trace cargo run --example document_driven_demo
```

关键日志：
- `[DOC-DRIVEN]`: 文档加载信息
- `[RALPH-LOOP]`: 循环执行信息
- `[AI-DECISION]`: AI决策过程
- `[EXECUTOR]`: 工具执行结果

---

## 进阶阅读

- **完整流程**: `/docs/AUTONOMOUS_LOOP_FLOW.md`
- **详细文档**: `/docs/DOCUMENT_DRIVEN_WORKFLOW.md`
- **代码示例**: `/examples/document_driven_demo.rs`
- **文档结构**: `/src/agent/workflow/ralph_loop/docs/README.md`

---

## 总结

**核心价值**：
```
人: 写文档（定义"做什么"和"怎么算完成"）
AI: 读文档（理解"我是谁"和"目标是什么"）
系统: 闭环驱动（自主迭代直到达成）
结果: 自动完成（无需人工干预）
```

**3步启动**：
1. `export ANTHROPIC_API_KEY=xxx`
2. `cargo run --example document_driven_demo`
3. 观察AI自主执行

就这么简单！
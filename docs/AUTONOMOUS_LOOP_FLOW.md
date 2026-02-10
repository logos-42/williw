# AI自主闭环工作流

## 完整流程图

```
┌──────────────────────────────────────────────────────────────────┐
│                   启动阶段（初始化）                        │
└──────────────────────────┬───────────────────────────────────┘
                         │
                         ▼
        ┌────────────────────────────────┐
        │  1. 创建执行器              │
        │  executor = new()         │
        └────────────┬───────────────┘
                     │
                     ▼
        ┌────────────────────────────────┐
        │  2. 读取内嵌文档            │
        │  - 身份文档                 │
        │  - 任务文档                 │
        │  - 工具文档                 │
        └────────────┬───────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────────────────────┐
│                    闭环执行阶段（Ralph Loop）                   │
│                                                            │
│  ┌──────────────┐     ┌──────────────┐     ┌─────────┐ │
│  │  读取文档    │────▶│  AI决策      │────▶│  执行    │ │
│  │  理解目标    │     │  选择下一步    │     │  调用工具 │ │
│  └──────┬───────┘     └──────┬───────┘     └────┬────┘ │
│         │                     │                     │         │
│         ▼                     │                     │         ▼
│  ┌──────────────┐            │              ┌─────────┐  │
│  │  生成计划    │            │              │  获取结果 │  │
│  │  转换步骤    │◀──────────┘              └────┬────┘  │
│  └──────┬───────┘                                 │         │
│         │                                           │         │
│         ▼                                           │         ▼
│  ┌──────────────┐                            ┌─────────┐  │
│  │  检查完成    │                            │  反思学习 │  │
│  │  验收标准    │◀─────────────────────────────────┘          │
│  └──────┬───────┘                                      │
│         │                                              │
│         │ 达成？                                        │
│         ├─否──▶ (继续循环)                               │
│         │                                              │
│         └─是─▶ 退出                                    │
│                                                          │
└───────────────────────────────────────────────────────────────────┘
                          │
                          ▼
              ┌──────────────────┐
              │  完成报告       │
              │  - 迭代次数      │
              │  - 验收标准      │
              │  - 执行历史      │
              └──────────────────┘
```

## 阶段详解

### 阶段1: 初始化

```rust
// 1. 创建执行器
let executor = AsyncWorkflowExecutor::new()?;

// 2. 配置（可选）
let ralph_config = RalphLoopConfig {
    max_iterations: 50,
    completion_checker: Some("所有验收标准达成".to_string()),
    ..Default::default()
};

// 3. 启动
executor.run_with_embedded_docs(
    "exec_id".to_string(),
    api_key,
    Some(ralph_config),
).await?;
```

**执行内容**:
- 加载内嵌文档到内存
- 创建执行上下文
- 初始化Ralph Loop状态

---

### 阶段2: 文档阅读（仅第1次迭代）

```markdown
# 身份文档示例
角色: 去中心化算力专家
专业领域: 模型切分、节点管理
行为准则: 切分前先分析、布置时考虑网络
```

```markdown
# 任务文档示例
目标: 切分模型到4个节点
验收标准:
- [ ] 切分为4个分片
- [ ] 分发完成
- [ ] 校验通过
步骤:
1. 分析模型
2. 执行切分
3. 分发分片
4. 验证结果
```

**AI理解过程**:
```
读取文档 → 解析结构 → 构建理解 → 存储到上下文
```

---

### 阶段3: 循环执行

#### 每个迭代做什么？

```
┌─────────────────────────────────────┐
│  第 N 次迭代                       │
├─────────────────────────────────────┤
│  1. 读取执行上下文                 │
│     - 已完成步骤                    │
│     - 当前位置                      │
│     - 执行历史                      │
│                                  │
│  2. 读取文档（理解角色和任务）         │
│     - 我是去中心化算力专家            │
│     - 目标是切分模型到4个节点         │
│                                  │
│  3. AI决策下一步                   │
│     分析: 还有什么没做？
│     决策: 继续执行步骤2              │
│                                  │
│  4. 执行工具                      │
│     工具: DecentralizedModel::Split  │
│     参数: {model_path, nodes...}      │
│                                  │
│  5. 获取结果                      │
│     - 成功？                       │
│     - 输出内容？                    │
│     - 需要重试？                    │
│                                  │
│  6. 更新上下文                     │
│     - 记录执行历史                    │
│     - 标记完成步骤                    │
│     - 保存学到的知识                   │
│                                  │
│  7. 自我反思（每5次）              │
│     - 什么做得好？                   │
│     - 什么需要改进？                 │
│     - 调整策略                      │
│                                  │
│  8. 检查完成条件                   │
│     所有验收标准都达成？               │
│     ├─是 → 退出循环                 │
│     └─否 → 继续下一次迭代             │
└─────────────────────────────────────┘
```

---

### 阶段4: 完成判断

**验收标准检查**:
```
文档中的:                        执行结果:
- [ ] 切分为4个分片         → 分片文件: shard_001.bin, shard_002.bin, ...
- [ ] 分发完成             → 传输成功: node1 ✅, node2 ✅, ...
- [ ] 校验通过             → 校验: SHA256 all passed

结果: 所有标准达成 ✅
```

**完成信号**:
- AI判断: "所有验收标准已达成"
- 生成报告: execution_report.json
- 退出Ralph Loop

---

## 代码实现流程

### 关键函数调用链

```
main()
  ↓
run_with_embedded_docs()
  ↓
run_document_driven_workflow()
  ├─> parse_identity()         # 解析身份
  ├─> parse_task()              # 解析任务
  ├─> build_workflow_from_task() # 构建工作流
  └─> execute_workflow_with_ralph_loop()
       ↓
    Ralph Loop (循环)
       ├─> execute_workflow_single_iteration()
       │    └─> execute_step_logic() → 工具调用
       ├─> ai_decide_next_action_with_context()
       │    └─> AI分析历史和上下文
       ├─> execute_ai_decision()
       │    ├─> COMPLETED → 退出
       │    ├─> RETRY → 重试
       │    ├─> RESEARCH → 调研
       │    └─> CONTINUE → 继续
       ├─> track_ai_learning_progress()
       └─> check_completion_condition() → 验收标准
```

---

## 数据流

### 上下文传递

```
执行开始
  │
  ├─ Identity: {name, role, expertise...}
  ├─ Task: {goal, acceptance_criteria, steps...}
  └─ Config: {max_iterations, completion_checker...}
  │
  ▼
Ralph Loop (每次迭代)
  │
  ├─ context: ExecutionContext
  │   ├─ current_iteration: N
  │   ├─ execution_history: [...]
  │   ├─ completed_steps: ["step1", "step2"]
  │   └─ learned_knowledge: {...}
  │
  ├─ iteration_result: ToolResult
  │   ├─ success: true/false
  │   ├─ output: "..."
  │   └─ data: {...}
  │
  └─ ai_decision: "CONTINUE" | "COMPLETED"
  │
  ▼
执行结束
  │
  └─ AutonomousResult
      ├─ success: true
      ├─ total_iterations: 5
      ├─ acceptance_criteria_met: ["c1", "c2"]
      └─ execution_log: [...]
```

---

## 实际执行示例

### 切分模型的完整执行

```
[迭代 1]
📚 读取文档
   👤 身份: 去中心化算力专家
   📋 任务: 切分模型到4个节点

🤖 AI决策
   分析: 需要执行步骤1 - 分析模型
   决策: CONTINUE

🔄 执行工具
   工具: DecentralizedModel::Analyze
   参数: {model_path: "/models/llm.bin"}
   ✅ 成功: 模型大小 4GB, 32层

✓ 完成步骤1

---

[迭代 2]
🤖 AI决策
   分析: 需要执行步骤2 - 切分模型
   决策: CONTINUE

🔄 执行工具
   工具: DecentralizedModel::Split
   参数: {model_path, nodes: 4}
   ✅ 成功: 生成4个分片

✓ 完成步骤2

---

[迭代 3]
🤖 AI决策
   分析: 需要执行步骤3 - 分发分片
   决策: CONTINUE

🔄 执行工具
   工具: DecentralizedModel::Transfer (并行)
   参数: [
     {shard: shard_001.bin, node: node1},
     {shard: shard_002.bin, node: node2},
     ...
   ]
   ✅ 成功: 所有分片已分发

✓ 完成步骤3

---

[迭代 4]
🤖 AI决策
   分析: 需要执行步骤4 - 验证结果
   决策: CONTINUE

🔄 执行工具
   工具: DecentralizedModel::Verify
   参数: {shards: 4, nodes: 4}
   ✅ 成功: 所有校验通过

✓ 完成步骤4

---

[迭代 5]
🔍 检查完成条件
   验收标准:
   ✅ [ ] 切分为4个分片 → 已达成
   ✅ [ ] 分发完成       → 已达成
   ✅ [ ] 校验通过       → 已达成

🤖 AI决策
   分析: 所有标准都达成了
   决策: COMPLETED

🎉 任务完成！
   总迭代: 5
   总耗时: 180秒
   达成标准: 3/3
```

---

## 关键特性

### 1. 自我驱动
- AI自主决定下一步
- 不需要人工干预
- 基于文档规则

### 2. 错误恢复
```rust
if 失败 {
    if is_retryable(error) {
        return RETRY;  // 自动重试
    } else if has_alternative() {
        return ADJUST;  // 调整策略
    } else {
        return ESCALATE;  // 上报
    }
}
```

### 3. 学习适应
```rust
if iteration % 5 == 0 {
   反思();
    调整策略();
}
```

### 4. 验收驱动
```rust
for criterion in acceptance_criteria {
    if !check(criterion) {
        return CONTINUE;  // 未达成，继续
    }
}
return COMPLETED;  // 全部达成
```

---

## 扩展方式

### 添加新身份

1. 创建 `src/agent/workflow/ralph_loop/docs/agents/new_agent.md`
2. 在 `mod.rs` 中添加常量
3. 使用时传入

```rust
let config = DocumentDrivenConfig {
    identity_doc_content: Some(IDENTITY_NEW_AGENT.to_string()),
    ..Default::default()
};
```

### 添加新任务

1. 创建 `src/agent/workflow/ralph_loop/docs/tasks/new_task.md`
2. 在 `mod.rs` 中添加常量
3. 使用时传入

### 添加新完成检查器

```rust
let ralph_config = RalphLoopConfig {
    completion_checker: Some("自定义检查逻辑".to_string()),
    ..Default::default()
};
```

---

## 总结

**AI自主闭环工作流** = 文档理解 + AI决策 + 工具执行 + 验收检查

```
人: 写文档（定义"做什么"）
AI: 读文档（理解"目标"）
AI: 做决策（决定"怎么做"）
AI: 用工具（执行"操作"）
系统: 检查（验证"结果"）
循环: 直到"达成目标"
```

真正的AI自主系统！
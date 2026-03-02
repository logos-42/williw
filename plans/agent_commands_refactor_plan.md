# Agent Commands 重构方案

## 背景分析

当前 `src-tauri/src/commands/agent_commands.rs` 文件约 98KB，包含以下功能模块：

1. **工具定义层** (line 19+): 各种 AI 工具的 JSON Schema 定义
2. **工具实现层**: 各种工具函数
   - `tool_check_system` - 系统检查
   - `tool_file_exists` - 文件存在检查
   - `tool_get_ollama_models` - 获取 Ollama 模型
   - `tool_create_plan` / `tool_get_todos` / `tool_add_todo` - 计划管理
   - `tool_get_system_info` - 系统信息
   - `tool_get_file_info` - 文件信息
3. **命令层**:
   - `run_ai_agent_setup` - AI 代理设置流程
   - `warmup_local_model` - 本地模型预热
   - `quick_start_local_inference` - 快速启动本地推理
   - `chat_with_local_endpoint` - 本地聊天

---

## 拆分方案（遵循人月神话原则）

### 原则
1. **每个模块职责单一**：一个模块只做一件事
2. **高内聚低耦合**：相关功能放一起，减少依赖
3. **支持并行开发**：清晰的接口让多人可以同时开发
4. **可测试性**：每个模块可独立测试

### 拆分后模块结构

```
src-tauri/src/commands/agent/
├── mod.rs                    # 模块入口
├── setup/                    # AI 设置流程
│   ├── mod.rs
│   ├── commands.rs           # run_ai_agent_setup 命令
│   └── workflow.rs           # AI 代理工作流逻辑
├── tools/                    # 工具定义和实现
│   ├── mod.rs
│   ├── definitions.rs        # 工具 JSON Schema 定义
│   ├── system.rs             # 系统检查工具
│   ├── file.rs               # 文件操作工具
│   ├── ollama.rs             # Ollama 相关工具
│   └── plan.rs               # 计划管理工具
├── chat/                     # 聊天功能
│   ├── mod.rs
│   ├── local.rs              # 本地聊天命令
│   └── warmup.rs             # 模型预热命令
└── task/                     # Task 执行系统（新增）
    ├── mod.rs
    ├── manifest.rs           # Task 定义
    ├── executor.rs           # Task 执行器
    └── swarm.rs              # Agent Swarm 支持
```

---

## Skills 规范设计

### 全局 Skills 目录结构

```
skills/                              # 全局 williw 目录
├── manifest.json                    # Skills 清单索引
├── builtin/                         # 内置 Skills
│   ├── compute_expert/
│   │   ├── SKILL.md
│   │   └── implementation/
│   ├── model_downloader/
│   │   └── SKILL.md
│   └── ...
├── agent/                           # Agent Skills
│   ├── code_reviewer/
│   │   └── SKILL.md
│   └── ...
└── custom/                          # 用户自定义 Skills
```

### SKILL.md 格式

```yaml
---
name: skill_name
display_name: 显示名称
description: 技能描述
category: agent|tool|workflow
version: 1.0.0
author: author_name
tags: [tag1, tag2]
---

# 角色定义
你是一个...

# 能力
- 能力1
- 能力2

# 约束
- 约束1

# 输入参数
| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| param1 | string | 是 | 参数1描述 |

# 输出格式
返回 JSON 格式的结果

# 执行流程
1. 步骤1
2. 步骤2
```

---

## Task 规范设计

### Task 定义格式

```yaml
---
name: task_name
display_name: 任务显示名称
description: 任务描述
type: sequential|parallel|swarm
version: 1.0.0
---

## 目标
任务要达成什么目标

## 输入参数
- param1: 类型 - 描述

## 验收标准
- [ ] 标准1
- [ ] 标准2

## 执行步骤
### 步骤1: 操作名称
- 操作: 具体操作
- 验证: 如何验证成功

## 并行设计（可选）
如果 type 为 parallel 或 swarm：
### 子任务
- task_1: 任务1描述
- task_2: 任务2描述

### 依赖关系
- task_2 依赖 task_1 的输出
```

### Agent Swarm 支持

```rust
// Task 执行器支持
pub enum TaskExecutionMode {
    /// 顺序执行
    Sequential,
    /// 并行执行
    Parallel,
    /// Agent Swarm（多智能体协作）
    Swarm {
        /// 参与 Agent 数量
        agent_count: usize,
        /// 协作策略
        strategy: SwarmStrategy,
    },
}

/// Swarm 协作策略
pub enum SwarmStrategy {
    /// 广播：所有 Agent 收到相同任务
    Broadcast,
    /// 分片：将任务拆分给不同 Agent
    Shard,
    /// 投票：Agent 投票决策
    Vote,
    /// 层级：Leader + Workers
    Hierarchical { leader_prompt: String },
}
```

---

## 去中心化算力流程优化

### 现有流程
```
用户请求 → AI Agent Setup → 配置推理服务 → 完成
```

### 优化后的流程

```
用户请求
    │
    ├─→ [Skill Router] ─→ 加载相关 Skills
    │        │
    │        ├─→ 本地已有模型？ → 直接使用
    │        │
    │        └─→ 需要配置？ → 启动 Agent Swarm
    │                          │
    │                          ├─→ Agent 1: 检查系统
    │                          ├─→ Agent 2: 下载模型 (并行)
    │                          └─→ Agent 3: 配置服务 (并行)
    │
    └─→ [Task Executor] ─→ 执行 Task
             │
             ├─→ 顺序 Task
             ├─→ 并行 Task
             └─→ Swarm Task
```

### 稳定性增强

1. **任务队列**：支持任务持久化，重启后可恢复
2. **Agent 池**：维护多个 Agent 实例，失败时自动切换
3. **健康检查**：定期检查 Agent 状态
4. **熔断机制**：连续失败后暂停并告警

---

## 实现步骤

### Phase 1: 模块拆分
1. 创建 `src-tauri/src/commands/agent/` 目录
2. 拆分工具定义为独立模块
3. 拆分命令为独立模块
4. 更新 `mod.rs` 导出

### Phase 2: Skills 集成
1. 创建全局 skills 目录结构
2. 实现 skills 加载器
3. 将现有工具转换为 skills

### Phase 3: Task 系统
1. 定义 Task 格式和解析
2. 实现 Task 执行器
3. 实现 Agent Swarm 支持

### Phase 4: 去中心化优化
1. 集成 Skills Router
2. 实现任务队列
3. 添加健康检查和熔断

---

## 向后兼容

- 保持现有 Tauri 命令接口不变
- 内部实现使用新模块
- 渐进式迁移，逐步替换

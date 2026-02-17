# williw 算力共享文档更新计划

> 基于《人月神话》原则的渐进式更新方案

---

## 📋 现状分析

### 当前架构理解

```
┌─────────────────────────────────────────────────────────────────┐
│                    williw 算力共享架构                           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────┐
│  Tauri App          │  用户接口层
│  (williw-master)    │  - 用户请求
│                     │  - 设备信息
└──────────┬──────────┘
           │ HTTP POST /api/inference
           ↓
┌─────────────────────────────────────────────────────────────────┐
│  边缘服务器 (williw-workers/edge_server)                         │
│  运行方式：宿主机直接运行（需要 GPU 访问）                          │
│                                                                 │
│  核心工作流 (workflow_orchestrator.py):                         │
│  1. 模型获取 (model_fetcher.py)                                 │
│  2. 模型转换 (model_converter.py)                               │
│  3. 算力估算 (compute_estimator.py) - 保守策略                   │
│  4. 节点信息获取 (interface_layer/node_info_api.py)             │
│  5. 算法层调用 (algorithms/)                                     │
│     - 节点选择 (node_selection.py)                              │
│     - 路径优化 (path_optimizer.py)                              │
│     - 资源分配 (resource_allocator.py)                          │
│     - 模型切分 (model_splitter.py)                              │
│     - 任务调度 (task_scheduler.py)                              │
│  6. 分布式推理 (models/inference_engine.py)                     │
│  7. 结果集成 (models/result_merger.py)                          │
└────────────┬────────────────────────────────────────────────────┘
             │
             ├────→ Worker A (层 1-4) → 退出
             ├────→ Worker B (层 5-8) → 退出
             └────→ Worker C (层 9-12) → 退出
```

### 现有文档问题

| 文档 | 问题 | 状态 |
|------|------|------|
| `docs/COMPUTE_SHARING.md` | 已删除（广播发现机制，与实际不符） | ❌ |
| `docs/WORKERS_DEPLOYMENT.md` | 只讲部署，未讲算力共享机制 | ⚠️ |
| `docs/WORKERS_MECHANISM.md` | 只讲机制，缺少使用示例 | ⚠️ |
| `ARCHITECTURE_SUMMARY.md` | 架构总结，非算力共享专题 | ⚠️ |

---

## 🎯 《人月神话》原则指导

### 1. 渐进增强 (Pilot System)

> "先实现核心功能，再逐步增强"

**应用:**
```
阶段 1: 文档描述单机算力共享（Mac 或 Windows）
阶段 2: 文档描述局域网算力共享（Mac ↔ Windows）
阶段 3: 文档描述分布式算力共享（多设备）
```

---

### 2. 概念完整性 (Conceptual Integrity)

> "文档应该反映一套统一的设计哲学"

**应用:**
```
统一术语:
- "Workers" = 临时加入的算力节点
- "边缘服务器" = 算力调度中心
- "Rust 节点" = P2P 通信节点（不参与算力调度）

统一流程:
1. 推理请求 → 2. 算力估算 → 3. 节点选择 → 
4. 模型切分 → 5. 分布式推理 → 6. 结果回收
```

---

### 3. 计划废止 (Planned Obsolescence)

> "设计时要考虑未来的替换和升级"

**应用:**
```
文档结构预留扩展点:
- 当前：单机算力共享
- 预留：多机算力共享
- 预留：跨地域算力共享

API 文档预留:
- 当前：HTTP API
- 预留：gRPC API
- 预留：P2P 直接通信
```

---

### 4. 外科手术团队 (Surgical Team)

> "关键模块需要专人负责和详细文档"

**应用:**
```
核心模块独立文档:
- 算力估算模块 (compute_estimator.py)
- 节点选择算法 (node_selection.py)
- 模型切分器 (model_splitter.py)
- Workers 生命周期管理
```

---

### 5. 第二系统效应 (Second-System Effect)

> "避免过度设计，聚焦核心功能"

**应用:**
```
文档聚焦:
✅ 核心：算力共享机制与使用
❌ 推迟：高级配置、优化技巧（放到进阶文档）
```

---

## 📝 文档更新计划

### 新文档结构

```
docs/
├── COMPUTE_SHARING.md          ← 新建：算力共享总览
│   ├── 什么是算力共享
│   ├── 架构概览
│   ├── 快速开始
│   └── 使用场景
│
├── COMPUTE_SHARING_WORKERS.md  ← 新建：Workers 机制详解
│   ├── Workers 生命周期
│   ├── Workers 注册与发现
│   ├── 任务分配机制
│   └── Workers 管理
│
├── COMPUTE_SHARING_ALGORITHMS.md ← 新建：算法层说明
│   ├── 节点选择算法
│   ├── 路径优化算法
│   ├── 资源分配算法
│   └── 模型切分算法
│
├── COMPUTE_SHARING_EXAMPLES.md ← 新建：使用示例
│   ├── 单机推理示例
│   ├── 局域网推理示例
│   └── 自定义 Workers 示例
│
├── WORKERS_DEPLOYMENT.md       ← 保留：部署指南
├── WORKERS_MECHANISM.md        ← 合并到 COMPUTE_SHARING_WORKERS.md
└── ARCHITECTURE_SUMMARY.md     ← 保留：架构总结
```

---

## 📋 详细更新任务

### 任务 1: 创建 `COMPUTE_SHARING.md`（总览）

**内容:**
```markdown
# williw 算力共享指南

## 什么是算力共享

williw 允许你：
- 在单设备上使用本地 GPU（MPS/CUDA）进行推理
- 在局域网上多台设备间共享算力
- 动态分配算力任务给最优设备

## 架构概览

[架构图]

## 快速开始

### 前置条件
- Python 3.8+
- PyTorch (支持 MPS 或 CUDA)
- Docker Desktop (可选，用于 Rust 节点)

### 一键启动
./start.sh --workers

### 测试推理
curl -X POST http://localhost:8080/api/inference \
  -d '{"model_name": "bert-base", "input_data": {"text": "Hello"}}'
```

**预计时间:** 2 小时

---

### 任务 2: 创建 `COMPUTE_SHARING_WORKERS.md`（Workers 机制）

**内容:**
```markdown
# Workers 机制详解

## Workers 生命周期

1. Worker 启动 → 2. 注册到边缘服务器 → 
3. 等待任务 → 4. 执行任务 → 5. 返回结果 → 6. 退出

## Workers 注册

```python
# 示例代码：注册 Worker
POST /api/worker/register
{
    "worker_id": "worker-1",
    "capabilities": {
        "device_type": "cuda",
        "gpu_name": "RTX 3080",
        "memory_gb": 24,
        "compute_power": 68.0
    }
}
```

## 任务分配

边缘服务器根据：
- 算力需求
- 网络延迟
- 设备可用性
- 可靠性评分

## Workers 管理

- 健康检查
- 超时处理
- 崩溃恢复
- 结果验证
```

**预计时间:** 3 小时

---

### 任务 3: 创建 `COMPUTE_SHARING_ALGORITHMS.md`（算法层）

**内容:**
```markdown
# 算力共享算法

## 节点选择算法

基于：
- 算力匹配度
- 网络延迟
- 设备可靠性
- 地理位置

## 路径优化 (D-CACO)

蚁群算法优化数据传输路径

## 资源分配

遗传算法 + 粒子群优化

## 模型切分

按层切分策略：
- 均匀切分
- 按算力切分
- 按内存切分
```

**预计时间:** 4 小时

---

### 任务 4: 创建 `COMPUTE_SHARING_EXAMPLES.md`（使用示例）

**内容:**
```markdown
# 算力共享示例

## 示例 1: 单机推理

```python
from interface_layer.app_client import InferenceClient

client = InferenceClient("http://localhost:8080")
result = client.inference(
    model_name="bert-base-uncased",
    input_data={"text": "Hello world"}
)
```

## 示例 2: 局域网多机推理

Mac (M2) + Windows (RTX 3080)

## 示例 3: 自定义 Workers

实现自定义 Worker 逻辑
```

**预计时间:** 3 小时

---

### 任务 5: 更新 `WORKERS_DEPLOYMENT.md`（部署指南）

**变更:**
```diff
+ 添加：算力共享部署场景
+ 添加：多设备部署配置
- 删除：广播发现相关内容
```

**预计时间:** 1 小时

---

### 任务 6: 删除/合并过时文档

**操作:**
```bash
# 删除
rm docs/COMPUTE_SHARING.md  # 已删除
rm docs/WORKERS_MECHANISM.md  # 合并到 COMPUTE_SHARING_WORKERS.md

# 保留
docs/WORKERS_DEPLOYMENT.md  # 更新后保留
docs/ARCHITECTURE_SUMMARY.md  # 保留
```

**预计时间:** 30 分钟

---

## 📅 实施计划

### 阶段 1: 核心文档（本周）

```
Day 1: 创建 COMPUTE_SHARING.md (总览)
Day 2: 创建 COMPUTE_SHARING_WORKERS.md (Workers 机制)
Day 3: 更新 WORKERS_DEPLOYMENT.md (部署指南)
```

### 阶段 2: 算法文档（下周）

```
Day 1: 创建 COMPUTE_SHARING_ALGORITHMS.md (算法层)
Day 2: 创建 COMPUTE_SHARING_EXAMPLES.md (使用示例)
Day 3: 删除/合并过时文档
```

### 阶段 3: 审查与完善（下下周）

```
Day 1: 技术审查（确保准确性）
Day 2: 文字审查（确保可读性）
Day 3: 用户测试（邀请用户试用文档）
```

---

## ✅ 验收标准

### 文档质量

- [ ] 架构图清晰
- [ ] 代码示例可运行
- [ ] 术语统一
- [ ] 链接有效

### 用户体验

- [ ] 新用户能在 5 分钟内理解算力共享
- [ ] 开发者能找到 API 参考
- [ ] 运维人员能找到部署指南

### 技术准确性

- [ ] 与代码实现一致
- [ ] 无过时信息
- [ ] 无矛盾描述

---

## 📊 工作量估算

| 任务 | 预计时间 | 优先级 |
|------|---------|--------|
| COMPUTE_SHARING.md | 2 小时 | P0 |
| COMPUTE_SHARING_WORKERS.md | 3 小时 | P0 |
| COMPUTE_SHARING_ALGORITHMS.md | 4 小时 | P1 |
| COMPUTE_SHARING_EXAMPLES.md | 3 小时 | P1 |
| 更新 WORKERS_DEPLOYMENT.md | 1 小时 | P0 |
| 删除/合并文档 | 30 分钟 | P2 |
| 审查与完善 | 4 小时 | P1 |

**总计:** 17.5 小时

---

## 🎯 下一步

1. **立即开始**: 创建 `COMPUTE_SHARING.md`（总览）
2. **本周完成**: 核心文档（P0 任务）
3. **下周完成**: 算法文档与示例（P1 任务）

---

*创建时间：2024-02-17*
*基于《人月神话》原则设计*

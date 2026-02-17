# williw 算力共享文档索引

> **快速导航** - 从入门到精通

---

## 🚀 快速开始

### 5 分钟上手

1. **阅读**: [算力共享总览](docs/COMPUTE_SHARING.md)
2. **安装**: `pip install -r williw-workers/requirements.txt`
3. **启动**: `./start.sh --workers`
4. **测试**: `curl http://localhost:8080/api/health`

---

## 📚 文档导航

### 入门级

| 文档 | 用途 | 阅读时间 |
|------|------|---------|
| [算力共享总览](docs/COMPUTE_SHARING.md) | 了解什么是算力共享 | 10 分钟 |
| [快速开始](docs/COMPUTE_SHARING.md#快速开始) | 5 分钟上手指南 | 5 分钟 |

### 进阶级

| 文档 | 用途 | 阅读时间 |
|------|------|---------|
| [Workers 机制](docs/COMPUTE_SHARING_WORKERS.md) | 深入理解 Workers | 20 分钟 |
| [算法层说明](docs/COMPUTE_SHARING_ALGORITHMS.md) | 了解核心算法 | 30 分钟 |

### 实战级

| 文档 | 用途 | 阅读时间 |
|------|------|---------|
| [使用示例](docs/COMPUTE_SHARING_EXAMPLES.md) | 6 个完整示例 | 40 分钟 |
| [部署指南](docs/WORKERS_DEPLOYMENT.md) | 生产环境部署 | 20 分钟 |

---

## 🎯 按场景查找

### 场景 1: 单机推理（Mac/Windows）

**推荐阅读:**
1. [算力共享总览 - 快速开始](docs/COMPUTE_SHARING.md#快速开始)
2. [使用示例 - 示例 1](docs/COMPUTE_SHARING_EXAMPLES.md#示例 -1-单机推理 mac-mps)
3. [使用示例 - 示例 2](docs/COMPUTE_SHARING_EXAMPLES.md#示例 -2-单机推理 windows-cuda)

**代码:**
```bash
# 启动服务
./start.sh --workers

# 测试推理
python3 williw-workers/example_usage.py
```

---

### 场景 2: 局域网多机推理

**推荐阅读:**
1. [算力共享总览 - 使用场景](docs/COMPUTE_SHARING.md#使用场景)
2. [使用示例 - 示例 3](docs/COMPUTE_SHARING_EXAMPLES.md#示例 -3-局域网多机推理)
3. [Workers 机制 - 注册](docs/COMPUTE_SHARING_WORKERS.md#workers-注册)

**代码:**
```bash
# Mac 上启动边缘服务器
python -m edge_server.api_server --port 8080

# Windows 上注册 Worker
curl -X POST http://192.168.1.100:8080/api/worker/register ...
```

---

### 场景 3: 自定义 Workers

**推荐阅读:**
1. [Workers 机制 - Workers 管理](docs/COMPUTE_SHARING_WORKERS.md#workers-管理)
2. [使用示例 - 示例 4](docs/COMPUTE_SHARING_EXAMPLES.md#示例 -4-自定义-workers)
3. [算法层 - 节点选择](docs/COMPUTE_SHARING_ALGORITHMS.md#2-节点选择算法)

**代码:**
```python
class CustomWorker:
    def register(self): ...
    async def execute_task(self, task): ...
```

---

## 🔧 故障排查

### 常见问题

| 问题 | 解决方案 |
|------|---------|
| 服务无法启动 | [部署指南 - 故障排查](docs/WORKERS_DEPLOYMENT.md#故障排查) |
| 推理超时 | [使用示例 - 故障排查](docs/COMPUTE_SHARING_EXAMPLES.md#故障排查) |
| GPU 不可用 | [总览 - 故障排查](docs/COMPUTE_SHARING.md#故障排查) |

---

## 📖 完整学习路径

### 第 1 天：入门

- [ ] 阅读 [算力共享总览](docs/COMPUTE_SHARING.md)
- [ ] 完成 [快速开始](docs/COMPUTE_SHARING.md#快速开始)
- [ ] 运行 [示例 1](docs/COMPUTE_SHARING_EXAMPLES.md#示例 -1-单机推理 mac-mps)

**预计时间:** 1 小时

---

### 第 2 天：理解机制

- [ ] 阅读 [Workers 机制](docs/COMPUTE_SHARING_WORKERS.md)
- [ ] 理解 Workers 生命周期
- [ ] 尝试注册 Worker

**预计时间:** 2 小时

---

### 第 3 天：深入算法

- [ ] 阅读 [算法层说明](docs/COMPUTE_SHARING_ALGORITHMS.md)
- [ ] 理解算力估算公式
- [ ] 了解节点选择算法

**预计时间:** 3 小时

---

### 第 4 天：实战演练

- [ ] 完成 [所有示例](docs/COMPUTE_SHARING_EXAMPLES.md)
- [ ] 尝试局域网推理
- [ ] 性能对比测试

**预计时间:** 4 小时

---

### 第 5 天：生产部署

- [ ] 阅读 [部署指南](docs/WORKERS_DEPLOYMENT.md)
- [ ] 配置生产环境
- [ ] 监控与优化

**预计时间:** 3 小时

---

## 📊 文档统计

| 指标 | 数值 |
|------|------|
| 文档总数 | 6 篇 |
| 总字数 | ~19,000 |
| 代码行数 | ~1,550 |
| 图表数量 | 28 个 |
| 示例数量 | 6 个 |

---

## 🔗 相关资源

### 代码仓库

- [williw-workers](../williw-workers/)
- [williw-master](../src/)
- [Tauri App](../src-tauri/)

### API 参考

- [边缘服务器 API](docs/COMPUTE_SHARING.md#api-参考)
- [Workers 注册 API](docs/COMPUTE_SHARING_WORKERS.md#workers-注册)

### 架构文档

- [架构总结](../ARCHITECTURE_SUMMARY.md)
- [架构评审](../ARCHITECTURE_REVIEW.md)

---

## 📝 更新日志

### 2024-02-17

- ✅ 新建算力共享系列文档（4 篇）
- ✅ 更新部署指南
- ✅ 创建文档索引（本文档）

---

## 💬 反馈与支持

遇到问题？

1. 查看 [故障排查](docs/COMPUTE_SHARING.md#故障排查)
2. 查看 [FAQ](docs/COMPUTE_SHARING_EXAMPLES.md#故障排查)
3. 提交 Issue

---

*最后更新：2024-02-17*

# Williw MVP 计划

> **目标**: 一个普通用户能安装、启动、并真实体验去中心化AI对话的桌面APP

---

## 现状分析（基于代码实际情况）

### ✅ 已有且可用的
| 功能 | 代码位置 | 状态 |
|---|---|---|
| iroh P2P 节点自动启动 | `src-tauri/src/main.rs` | ✅ 完整实现 |
| 对话聊天界面 | `src/components/ChatBox.tsx` | ✅ 完整实现 |
| 外部 API 调用 (OpenAI/DeepSeek等) | `src-tauri/src/commands/external_api_commands.rs` | ✅ 完整实现 |
| 节点信息上报 (Cloudflare Workers) | `src-tauri/src/main.rs` (AutoUpload) | ✅ 每30秒上报 |
| 设备信息检测 (CPU/GPU/内存) | `src-tauri/src/state.rs` | ✅ 完整实现 |
| API Key 管理 | `src-tauri/src/commands/api_key_commands.rs` | ✅ 完整实现 |
| 节点 on/off 开关 | `src/components/TrainingSwitch.tsx` | ✅ 完整实现 |
| 连接的 Peer 节点列表 | `src-tauri/src/commands/node_commands.rs` | ✅ 完整实现 |

### ⚠️ 存在的问题
1. **首次体验差** — 用户打开APP不知道要干什么，没有引导
2. **左侧面板太复杂** — TrainingDashboard 展示太多技术细节，普通用户看不懂
3. **ModelSelector 有硬编码路径** — ChatBox 里有 `D:\\AI\\...` 的Windows路径
4. **无持久化存储** — API Key 重启后丢失（内存存储）
5. **无积分/贡献显示** — 用户不知道自己贡献了什么价值

---

## MVP 范围定义

### 🎯 MVP 核心体验（一句话）
> 用户打开APP → 输入自己的AI API Key → 开始对话 → 同时贡献算力到网络 → 获得积分

### MVP 必须有 ✅
1. **简单上手流程**: 首次启动显示引导（填写API Key → 完成）
2. **可用的对话**: 输入问题 → 得到AI回答（通过外部API）
3. **节点运行状态**: 清晰显示"节点运行中/离线"、节点ID
4. **贡献积分显示**: 即使是模拟数据，也要显示"你已贡献XXX次"
5. **API Key 持久化**: 重启后不丢失配置

### MVP 先不做 ❌
- 真实的去中心化推理（先用外部API代替）
- Solana/Base 链上质押
- Android 移动端
- Python 算法层（蚁群、遗传算法）
- GPU 本地推理服务器
- 复杂的拓扑可视化

---

## 实施计划

### Phase 1 — 修复基础体验（当前优先级）

#### 1.1 API Key 持久化
- **问题**: 当前API Key存内存，重启丢失
- **方案**: 用 `tauri-plugin-store` 或写文件到 app data 目录
- **文件**: `src-tauri/src/commands/api_key_commands.rs`

#### 1.2 去掉 ChatBox 里的硬编码路径
- **问题**: `D:\\AI\\...` Windows路径会让非Windows/非特定路径用户无法用
- **方案**: 如果外部API失败，显示"请配置外部API"而不是尝试本地路径
- **文件**: `src/components/ChatBox.tsx`

#### 1.3 首次启动引导
- **方案**: 检测是否有已配置的API，没有则显示欢迎弹窗引导用户配置
- **文件**: 新建 `src/components/OnboardingDialog.tsx`

#### 1.4 简化左侧面板
- **方案**: 左侧主要显示：节点状态（在线/离线）+ 节点ID + 贡献统计 + 连接的Peer数
- **文件**: `src/components/TrainingDashboard.tsx`

### Phase 2 — 增强网络体验

#### 2.1 贡献积分系统（模拟）
- 每处理一条消息 +1 积分
- 节点在线时间积分
- 显示在UI上

#### 2.2 Peer 发现展示
- 显示当前连接了多少个 Peer
- 显示网络拓扑（简单文字描述）

### Phase 3 — 打包发布

#### 3.1 构建测试
```bash
cd src-tauri && cargo build --release
npm run build
npm run tauri build
```

#### 3.2 写 5 分钟 Quick Start 文档

#### 3.3 录制 Demo 视频

---

## 立即要做的事（今天）

```
优先级 P0 (阻塞MVP):
□ 修复 ChatBox 硬编码路径问题
□ API Key 持久化（写入文件）
□ 首次启动引导弹窗

优先级 P1 (MVP 质量):
□ 左侧面板简化 - 突出显示节点状态
□ 贡献积分 UI（模拟数据）

优先级 P2 (打磨):
□ 错误提示更友好
□ 节点ID复制按钮
□ 打包构建测试
```

---

## 技术债务（不影响MVP，后续解决）

1. `williw-workers/` 整个Python项目 — MVP完全不需要，可以忽略
2. `android_lib_full/` — 移动端，MVP后做
3. `decentralized-training-contract/` — Solana合约，激励系统第二期
4. `src/training/` `src/consensus/` 等复杂模块 — 分布式训练核心，第三期

---

## 成功标准

MVP完成的标志：
1. 从零安装，5分钟内能开始对话
2. 节点正常启动并在P2P网络中可见
3. 关闭重启后，API配置还在
4. 用户能看到自己"贡献"了多少次

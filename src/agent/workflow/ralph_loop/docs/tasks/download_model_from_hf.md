# 任务：从HuggingFace下载AI模型

## 目标
从HuggingFace Hub下载指定的AI模型到本地目录，并为后续的模型切分和分布式推理做好准备。

## 描述
这是一个模型下载任务。你需要连接到HuggingFace Hub，下载指定的预训练大语言模型。模型下载完成后，需要验证文件完整性，并准备进行模型切分。

## 输入参数
- **模型名称**: 可以在以下位置指定
  - 任务参数 `model_name`
  - 或者询问用户
- **模型ID**: 如 `meta-llama/Llama-2-7b-hf`, `Qwen/Qwen-7B`, `microsoft/Phi-3-mini-4k-instruct` 等
- **本地存储路径**: `./models/` (默认)
- **模型类型**: LLM (大语言模型)

## 验收标准（必须全部达成）
- [ ] 成功连接到HuggingFace Hub
- [ ] 模型文件完整下载到本地目录
- [ ] 生成文件校验和并验证完整性
- [ ] 下载完成报告生成
- [ ] 准备好进行模型切分

## 执行步骤

### 步骤1: 检查环境和依赖
- **操作**: 检查Python环境和huggingface_hub库是否可用
- **验证**: 确认可以导入huggingface_hub
- **工具**: 使用BashTool执行Python命令检查

### 步骤2: 确定下载方式
- **操作**: 选择最佳的下载方式
- **决策**:
  - 如果有 huggingface_hub 库：使用Python下载
  - 如果只有git：使用git clone
  - 如果只有huggingface-cli：使用命令行工具
- **工具**: 使用BashTool执行相应命令

### 步骤3: 下载模型
- **操作**: 执行模型下载
- **验证**: 监控下载进度，确认文件大小合理
- **超时**: 大模型可能需要较长时间，设置600秒以上

**推荐的下载命令（使用Python）：**
```python
from huggingface_hub import snapshot_download
snapshot_download(repo_id="meta-llama/Llama-2-7b-hf", local_dir="./models/llama-2-7b")
```

**如果需要认证：**
```python
from huggingface_hub import HfApi, login
# 先登录
login(token="你的hf_xxx token")
# 然后下载
snapshot_download(repo_id="meta-llama/Llama-2-7b-hf", local_dir="./models/llama-2-7b")
```

### 步骤4: 验证下载完整性
- **操作**: 检查下载的文件，验证大小和数量
- **验证**: 
  - 文件数量与预期一致
  - 文件大小合理（非0字节）
  - 关键文件存在（如config.json, model.safetensors）
- **工具**: 使用BashTool执行ls命令检查

### 步骤5: 生成下载报告
- **操作**: 记录下载的模型信息
- **输出**: 报告包含：
  - 模型ID
  - 下载路径
  - 文件列表
  - 文件大小
  - 下载时间

## 约束条件
- **网络要求**: 稳定的互联网连接
- **存储空间**: 确保有足够的磁盘空间（模型通常1GB-100GB）
- **下载超时**: 大模型可能需要30分钟以上

## 故障处理
1. **认证失败**: 提示用户获取HuggingFace token并设置环境变量
2. **网络中断**: 支持断点续传（huggingface_hub默认支持）
3. **磁盘空间不足**: 清理空间后重试
4. **模型不存在**: 检查模型ID是否正确

## 完成信号
任务完成时，应该：
1. 模型文件已下载到 `./models/<model_name>/` 目录
2. 生成下载报告
3. 可以开始执行模型切分任务

## 重要提示
- **必须使用BashTool** 调用Python或shell命令下载模型
- DecentralizedModel::Download 操作当前是stub，不能直接使用
- 下载大模型需要较长时间，请耐心等待
- 建议使用镜像站或国内网络加速（如果可用）

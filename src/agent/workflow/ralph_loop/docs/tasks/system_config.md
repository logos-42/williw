# 任务：系统配置

## 目标
配置去中心化算力网络的系统环境，包括 Python 环境、依赖安装、模型配置和 P2P 网络设置。

## 描述
本任务需要完成整个系统环境的初始化，确保所有必要的软件和工具都已正确安装和配置。任务包括环境检测、依赖安装、模型准备和网络配置四个主要阶段。

## 验收标准
1. ✅ Python 3.8+ 已安装且可用
2. ✅ pip 包管理器已安装
3. ✅ PyTorch 深度学习框架已安装
4. ✅ Transformers 库已安装
5. ✅ Iroh P2P 工具已安装
6. ✅ GPU/CUDA 检测完成（如可用）
7. ✅ 模型可以正常加载和切分
8. ✅ P2P 网络可以正常通信

## 执行步骤

### Phase 1: 环境检测
1. **检测 Python**
   - 工具: BashTool
   - 命令: `python --version || python3 --version`
   - 验证: 获取到版本号（如 Python 3.9.0）

2. **检测 pip**
   - 工具: BashTool
   - 命令: `pip --version || pip3 --version`
   - 验证: 获取到 pip 版本号

3. **检测 GPU/CUDA**
   - 工具: BashTool
   - 命令: `nvcc --version 2>/dev/null || python3 -c "import torch; print(torch.cuda.is_available())"`
   - 验证: 知道 GPU 是否可用

### Phase 2: 依赖安装
4. **安装 PyTorch**
   - 工具: BashTool
   - 命令: `pip install --upgrade torch`
   - 验证: `python -c "import torch; print(torch.__version__)"`

5. **安装 Transformers**
   - 工具: BashTool
   - 命令: `pip install --upgrade transformers accelerate sentencepiece protobuf`
   - 验证: `python -c "import transformers; print(transformers.__version__)"`

6. **安装 Iroh**
   - 工具: BashTool
   - 命令: `pip install --upgrade iroh`
   - 验证: `python -c "import iroh; print('iroh installed')"`

### Phase 3: 模型配置
7. **配置模型**
   - 工具: DecentralizedModel
   - 操作: FullPipeline
   - 输入: model_name="default_model", model_source="huggingface"
   - 验证: 模型可以正常加载

### Phase 4: 网络配置
8. **初始化 Iroh**
   - 工具: IrohComms
   - 操作: GetNodeId
   - 验证: 获取到节点ID

9. **测试网络**
   - 工具: IrohComms
   - 操作: BroadcastMessage
   - 消息: "节点已就绪"
   - 验证: 消息发送成功

## 输入参数
- python_version: "3.8+"
- install_packages: ["torch", "transformers", "iroh"]
- model_name: "default_model"
- network_enabled: true

## 约束条件
- 最大迭代次数: 50
- 超时时间: 30分钟
- 内存限制: 4GB
- 重试次数: 3次

## 注意事项
1. 如果 Python 未安装，提示用户先安装 Python
2. 如果 GPU 不可用，使用 CPU 模式继续
3. 安装失败时等待2秒后重试
4. 每个步骤完成后验证结果
5. 记录所有安装输出用于调试

## 成功标准
- 所有必要软件已安装
- 环境检测通过
- 模型可以正常加载
- 网络通信正常
- 可以开始接收推理任务

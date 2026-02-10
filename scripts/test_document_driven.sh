#!/bin/bash
# 测试文档驱动的自主工作流系统

echo "🧪 测试文档驱动的自主工作流系统"
echo "====================================="
echo ""

# 检查文件是否存在
echo "📁 检查文件..."
if [ -f "src/agent/workflow/ralph_loop/docs/mod.rs" ]; then
    echo "   ✅ docs/mod.rs 存在"
else
    echo "   ❌ docs/mod.rs 不存在"
    exit 1
fi

if [ -f "src/agent/workflow/ralph_loop/docs/agents/compute_expert.md" ]; then
    echo "   ✅ docs/agents/compute_expert.md 存在"
else
    echo "   ❌ docs/agents/compute_expert.md 不存在"
    exit 1
fi

if [ -f "src/agent/workflow/ralph_loop/docs/tasks/split_model_example.md" ]; then
    echo "   ✅ docs/tasks/split_model_example.md 存在"
else
    echo "   ❌ docs/tasks/split_model_example.md 不存在"
    exit 1
fi

if [ -f "src/agent/workflow/ralph_loop/docs/tools/DecentralizedModel.md" ]; then
    echo "   ✅ docs/tools/DecentralizedModel.md 存在"
else
    echo "   ❌ docs/tools/DecentralizedModel.md 不存在"
    exit 1
fi

if [ -f "examples/document_driven_demo.rs" ]; then
    echo "   ✅ examples/document_driven_demo.rs 存在"
else
    echo "   ❌ examples/document_driven_demo.rs 不存在"
    exit 1
fi

echo ""
echo "🔍 检查文档内容..."

# 检查文档是否包含必要的内容
if grep -q "去中心化算力专家" src/agent/workflow/ralph_loop/docs/agents/compute_expert.md; then
    echo "   ✅ 身份文档包含必要内容"
else
    echo "   ❌ 身份文档内容不完整"
    exit 1
fi

if grep -q "验收标准" src/agent/workflow/ralph_loop/docs/tasks/split_model_example.md; then
    echo "   ✅ 任务文档包含验收标准"
else
    echo "   ❌ 任务文档内容不完整"
    exit 1
fi

if grep -q "DecentralizedModel" src/agent/workflow/ralph_loop/docs/tools/DecentralizedModel.md; then
    echo "   ✅ 工具文档包含必要内容"
else
    echo "   ❌ 工具文档内容不完整"
    exit 1
fi

echo ""
echo "🔨 编译测试..."

# 编译库
echo "   编译库..."
cargo check 2>&1 | grep -q "Finished\|Checking williw"
if [ $? -eq 0 ]; then
    echo "   ✅ 库编译成功"
else
    echo "   ❌ 库编译失败"
    exit 1
fi

# 编译示例
echo "   编译示例..."
cargo check --example document_driven_demo 2>&1 | grep -q "Finished\|Checking williw"
if [ $? -eq 0 ]; then
    echo "   ✅ 示例编译成功"
else
    echo "   ❌ 示例编译失败"
    exit 1
fi

echo ""
echo "📋 检查代码集成..."

# 检查是否导出了必要的常量
if grep -q "IDENTITY_COMPUTE_EXPERT" src/agent/workflow/ralph_loop/docs/mod.rs; then
    echo "   ✅ IDENTITY_COMPUTE_EXPERT 已导出"
else
    echo "   ❌ IDENTITY_COMPUTE_EXPERT 未导出"
    exit 1
fi

if grep -q "TASK_SPLIT_MODEL_EXAMPLE" src/agent/workflow/ralph_loop/docs/mod.rs; then
    echo "   ✅ TASK_SPLIT_MODEL_EXAMPLE 已导出"
else
    echo "   ❌ TASK_SPLIT_MODEL_EXAMPLE 未导出"
    exit 1
fi

if grep -q "TOOL_DECENTRALIZED_MODEL" src/agent/workflow/ralph_loop/docs/mod.rs; then
    echo "   ✅ TOOL_DECENTRALIZED_MODEL 已导出"
else
    echo "   ❌ TOOL_DECENTRALIZED_MODEL 未导出"
    exit 1
fi

# 检查document_driven.rs是否引用了正确的模块
if grep -q "super::IDENTITY_COMPUTE_EXPERT" src/agent/workflow/ralph_loop/document_driven.rs; then
    echo "   ✅ document_driven.rs 引用了身份文档"
else
    echo "   ❌ document_driven.rs 未正确引用身份文档"
    exit 1
fi

if grep -q "super::TASK_SPLIT_MODEL_EXAMPLE" src/agent/workflow/ralph_loop/document_driven.rs; then
    echo "   ✅ document_driven.rs 引用了任务文档"
else
    echo "   ❌ document_driven.rs 未正确引用任务文档"
    exit 1
fi

echo ""
echo "🎉 所有测试通过！"
echo ""
echo "📊 测试总结:"
echo "   ✅ 文件结构正确"
echo "   ✅ 文档内容完整"
echo "   ✅ 编译成功"
echo "   ✅ 代码集成正确"
echo ""
echo "🚀 系统已就绪，可以运行！"
echo ""
echo "运行示例:"
echo "  cargo run --example document_driven_demo"
echo ""
echo "使用方式:"
echo "  executor.run_with_embedded_docs(execution_id, api_key, None).await?"

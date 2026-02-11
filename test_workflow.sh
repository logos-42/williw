#!/bin/bash

# 测试 Williw 工作流

echo "🧪 测试 Williw AI 工作流"
echo "========================"

# 检查应用是否运行
echo ""
echo "1️⃣ 检查应用状态..."
if pgrep -x "williw-desktop" > /dev/null; then
    echo "✅ 应用正在运行"
    PID=$(pgrep -x "williw-desktop")
    echo "   PID: $PID"
else
    echo "❌ 应用未运行"
    exit 1
fi

# 检查端口
echo ""
echo "2️⃣ 检查网络端口..."
PORT_1420=$(lsof -ti:1420 2>/dev/null | wc -l)
PORT_3000=$(lsof -ti:3000 2>/dev/null | wc -l)

if [ "$PORT_1420" -gt 0 ]; then
    echo "✅ 端口 1420 (Vite) 正在使用"
fi

if [ "$PORT_3000" -gt 0 ]; then
    echo "✅ 端口 3000 (开发服务器) 正在使用"
fi

# 检查文件完整性
echo ""
echo "3️⃣ 检查关键文件..."
FILES=(
    "src-tauri/src/commands/workflow_commands.rs"
    "src-tauri/src/agent/workflow/ralph_loop/document_driven.rs"
    "src-tauri/src/agent/workflow/ralph_loop/ai_decision.rs"
    "src-tauri/src/agent/workflow/ralph_loop/docs/agents/system_config_expert.md"
    "src-tauri/src/agent/workflow/ralph_loop/docs/tasks/system_config.md"
)

for file in "${FILES[@]}"; do
    if [ -f "/Users/apple/Downloads/williw-master/$file" ]; then
        echo "✅ $file"
    else
        echo "❌ $file 不存在"
    fi
done

# 检查 Rust 编译
echo ""
echo "4️⃣ 检查 Rust 代码编译状态..."
cd /Users/apple/Downloads/williw-master/src-tauri
if cargo check --quiet 2>/dev/null; then
    echo "✅ Rust 代码编译正常"
else
    echo "⚠️ Rust 代码有警告或错误"
fi

echo ""
echo "========================"
echo "测试完成！"
echo ""
echo "💡 使用说明："
echo "1. 打开浏览器访问: http://localhost:1420"
echo "2. 点击左上角的 '运行' 按钮"
echo "3. 查看右侧对话框中的工作流进度"
echo "4. 观察终端日志了解 AI 决策过程"
echo ""
echo "📊 工作流特性："
echo "- 文档驱动: AI 读取身份和任务文档"
echo "- Ralph Loop: AI 自主决策和迭代"
echo "- 智能重试: 失败时自动重试"
echo "- 工具调用: BashTool、DecentralizedModelTool、IrohCommsTool"

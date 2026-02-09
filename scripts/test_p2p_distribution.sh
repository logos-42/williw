#!/bin/bash

# P2P 模型分发测试脚本
# 测试发送端和接收端的完整功能

set -e

echo "🚀 开始 P2P 模型分发测试"

# 检查必要的目录
if [ ! -d "./test_models/test_models/simple_split" ]; then
    echo "❌ 错误: 找不到模型分片目录 ./test_models/test_models/simple_split"
    echo "请先运行模型切分脚本"
    exit 1
fi

# 创建测试输出目录
TEST_OUTPUT_DIR="./test_output/p2p_test_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$TEST_OUTPUT_DIR"
RECEIVED_DIR="$TEST_OUTPUT_DIR/received"
mkdir -p "$RECEIVED_DIR"

echo "📁 测试输出目录: $TEST_OUTPUT_DIR"

# 编译项目
echo "🔨 编译项目..."
cargo build --release --example p2p_model_distribution_demo

# 步骤1: 测试文件完整性
echo ""
echo "🔍 步骤1: 测试文件完整性..."
if [ -f "./test_models/test_models/simple_split/node_001.json" ]; then
    cargo run --release --example p2p_model_distribution_demo -- test-integrity \
        --file-path "./test_models/test_models/simple_split/node_001.json" \
        --algorithm sha256
else
    echo "⚠️  跳过完整性测试（未找到测试文件）"
fi

# 步骤2: 启动接收端（后台）
echo ""
echo "📡 步骤2: 启动接收端..."
RECEIVER_LOG="$TEST_OUTPUT_DIR/receiver.log"
cargo run --release --example p2p_model_distribution_demo -- receive \
    --node-id "test_receiver" \
    --output-dir "$RECEIVED_DIR" \
    --port 9236 \
    --auto-accept > "$RECEIVER_LOG" 2>&1 &
RECEIVER_PID=$!

echo "   接收端 PID: $RECEIVER_PID"
echo "   日志文件: $RECEIVER_LOG"

# 等待接收端启动
echo "⏳ 等待接收端启动..."
sleep 3

# 检查接收端是否正常启动
if ! kill -0 $RECEIVER_PID 2>/dev/null; then
    echo "❌ 接收端启动失败"
    cat "$RECEIVER_LOG"
    exit 1
fi

echo "✅ 接收端已启动"

# 步骤3: 启动发送端
echo ""
echo "📤 步骤3: 启动发送端..."
SENDER_LOG="$TEST_OUTPUT_DIR/sender.log"
cargo run --release --example p2p_model_distribution_demo -- send \
    --node-id "test_sender" \
    --target-peer "test_receiver" \
    --shard-dir "./test_models/test_models/simple_split" \
    --chunk-size 1048576 \
    --port 9235 > "$SENDER_LOG" 2>&1 &

SENDER_PID=$!
echo "   发送端 PID: $SENDER_PID"
echo "   日志文件: $SENDER_LOG"

# 等待发送完成
echo "⏳ 等待发送完成..."
wait $SENDER_PID
SENDER_EXIT_CODE=$?

echo "发送端退出代码: $SENDER_EXIT_CODE"

# 等待一段时间确保接收完成
echo "⏳ 等待接收完成..."
sleep 5

# 停止接收端
echo "🛑 停止接收端..."
kill $RECEIVER_PID 2>/dev/null || true
wait $RECEIVER_PID 2>/dev/null || true

# 步骤4: 验证结果
echo ""
echo "🔍 步骤4: 验证传输结果..."

# 统计源文件
SOURCE_FILES=$(find "./test_models/test_models/simple_split" -name "*.json" -o -name "*.pth" -o -name "*.safetensors" | wc -l)
SOURCE_SIZE=$(du -sh "./test_models/test_models/simple_split" | cut -f1)

echo "📊 源文件统计:"
echo "   文件数量: $SOURCE_FILES"
echo "   总大小: $SOURCE_SIZE"

# 统计接收文件
RECEIVED_FILES=$(find "$RECEIVED_DIR" -type f | wc -l)
if [ $RECEIVED_FILES -gt 0 ]; then
    RECEIVED_SIZE=$(du -sh "$RECEIVED_DIR" | cut -f1)
else
    RECEIVED_SIZE="0"
fi

echo "📊 接收文件统计:"
echo "   文件数量: $RECEIVED_FILES"
echo "   总大小: $RECEIVED_SIZE"

# 验证文件完整性
echo ""
echo "🔍 验证接收文件完整性..."
VALIDATION_FAILED=0

for file in "./test_models/test_models/simple_split"/*.json; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        received_file="$RECEIVED_DIR/$filename"
        
        if [ -f "$received_file" ]; then
            # 比较文件大小
            source_size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null)
            received_size=$(stat -f%z "$received_file" 2>/dev/null || stat -c%s "$received_file" 2>/dev/null)
            
            if [ "$source_size" -eq "$received_size" ]; then
                echo "✅ $filename (大小匹配)"
            else
                echo "❌ $filename (大小不匹配: $source_size vs $received_size)"
                VALIDATION_FAILED=1
            fi
        else
            echo "❌ $filename (未接收到)"
            VALIDATION_FAILED=1
        fi
    fi
done

# 步骤5: 生成测试报告
echo ""
echo "📋 生成测试报告..."
REPORT_FILE="$TEST_OUTPUT_DIR/test_report.json"

cat > "$REPORT_FILE" << EOF
{
    "test_type": "p2p_model_distribution",
    "timestamp": "$(date -Iseconds)",
    "source": {
        "directory": "./test_models/test_models/simple_split",
        "file_count": $SOURCE_FILES,
        "total_size": "$SOURCE_SIZE"
    },
    "received": {
        "directory": "$RECEIVED_DIR",
        "file_count": $RECEIVED_FILES,
        "total_size": "$RECEIVED_SIZE"
    },
    "sender": {
        "exit_code": $SENDER_EXIT_CODE,
        "log_file": "$SENDER_LOG"
    },
    "receiver": {
        "log_file": "$RECEIVER_LOG"
    },
    "validation": {
        "passed": $([ $VALIDATION_FAILED -eq 0 ] && echo true || echo false),
        "failed_files": $VALIDATION_FAILED
    },
    "success": $([ $SENDER_EXIT_CODE -eq 0 ] && [ $VALIDATION_FAILED -eq 0 ] && echo true || echo false)
}
EOF

echo "📁 测试报告已保存: $REPORT_FILE"

# 显示测试结果摘要
echo ""
echo "📊 测试结果摘要:"
echo "   测试目录: $TEST_OUTPUT_DIR"
echo "   源文件数: $SOURCE_FILES"
echo "   接收文件数: $RECEIVED_FILES"
echo "   发送端状态: $([ $SENDER_EXIT_CODE -eq 0 ] && echo "成功" || echo "失败")"
echo "   验证状态: $([ $VALIDATION_FAILED -eq 0 ] && echo "通过" || echo "失败")"

if [ $SENDER_EXIT_CODE -eq 0 ] && [ $VALIDATION_FAILED -eq 0 ]; then
    echo ""
    echo "🎉 P2P 模型分发测试成功完成！"
    echo ""
    echo "📁 查看详细日志:"
    echo "   发送端: cat $SENDER_LOG"
    echo "   接收端: cat $RECEIVER_LOG"
    echo ""
    echo "📁 查看接收的文件:"
    echo "   ls -la $RECEIVED_DIR/"
else
    echo ""
    echo "❌ P2P 模型分发测试失败"
    echo ""
    echo "🔍 查看错误日志:"
    echo "   发送端: cat $SENDER_LOG"
    echo "   接收端: cat $RECEIVER_LOG"
    exit 1
fi

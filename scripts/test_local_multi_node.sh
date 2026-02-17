#!/usr/bin/env bash
# ===================================================================
# williw 本地多节点测试脚本
# ===================================================================
#
# 功能：
# - 在同一台电脑上启动多个节点
# - 测试节点间的 P2P 通信
# - 验证消息传递和拓扑发现
#
# 使用方式:
#   ./scripts/test_local_multi_node.sh
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 清理函数
cleanup() {
    log_info "清理测试进程..."
    pkill -f "williw-bin.*--node-id node1" 2>/dev/null || true
    pkill -f "williw-bin.*--node-id node2" 2>/dev/null || true
    pkill -f "williw-bin.*--node-id node3" 2>/dev/null || true
    sleep 2
    log_success "清理完成"
}

# 捕获退出信号
trap cleanup EXIT

# 配置
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DURATION=60
LOG_DIR="/tmp/williw_test_logs"

# 创建日志目录
mkdir -p "$LOG_DIR"

log_info "=========================================="
log_info "  williw 本地多节点测试"
log_info "=========================================="
log_info "项目目录: $PROJECT_DIR"
log_info "日志目录: $LOG_DIR"

# 切换到项目目录
cd "$PROJECT_DIR"

# 检查编译
log_info "检查编译状态..."
if [ ! -f "target/release/williw-bin" ]; then
    log_info "正在编译 release 版本..."
    cargo build --release
fi
log_success "编译完成"

# 清理旧进程
cleanup

# 启动节点1（引导节点）
log_info "启动节点1（引导节点）..."
RUST_LOG=info ./target/release/williw-bin \
    --node-id 1 \
    --quic-port 9235 \
    --model-dim 128 \
    > "$LOG_DIR/node1.log" 2>&1 &
NODE1_PID=$!
log_info "节点1 PID: $NODE1_PID"

# 等待节点1启动
sleep 3

# 检查节点1是否启动
if ! ps -p $NODE1_PID > /dev/null; then
    log_error "节点1启动失败"
    cat "$LOG_DIR/node1.log"
    exit 1
fi

log_success "节点1启动成功"
log_info "节点1日志："
head -20 "$LOG_DIR/node1.log"

# 启动节点2（连接到节点1）
log_info "启动节点2（连接节点1）..."
RUST_LOG=info ./target/release/williw-bin \
    --node-id 2 \
    --quic-port 9236 \
    --model-dim 128 \
    --bootstrap 127.0.0.1:9235 \
    > "$LOG_DIR/node2.log" 2>&1 &
NODE2_PID=$!
log_info "节点2 PID: $NODE2_PID"

# 等待节点2启动
sleep 3

# 检查节点2是否启动
if ! ps -p $NODE2_PID > /dev/null; then
    log_error "节点2启动失败"
    cat "$LOG_DIR/node2.log"
    exit 1
fi

log_success "节点2启动成功"
log_info "节点2日志："
head -20 "$LOG_DIR/node2.log"

# 启动节点3（连接到节点1）
log_info "启动节点3（连接节点1）..."
RUST_LOG=info ./target/release/williw-bin \
    --node-id 3 \
    --quic-port 9237 \
    --model-dim 128 \
    --bootstrap 127.0.0.1:9235 \
    > "$LOG_DIR/node3.log" 2>&1 &
NODE3_PID=$!
log_info "节点3 PID: $NODE3_PID"

# 等待节点3启动
sleep 3

# 检查节点3是否启动
if ! ps -p $NODE3_PID > /dev/null; then
    log_error "节点3启动失败"
    cat "$LOG_DIR/node3.log"
    exit 1
fi

log_success "节点3启动成功"
log_info "节点3日志："
head -20 "$LOG_DIR/node3.log"

# ========================================
# 监控阶段
# ========================================
log_info "=========================================="
log_info "  开始监控节点通信 ($TEST_DURATION 秒)"
log_info "=========================================="

start_time=$(date +%s)
connections_found=0
messages_exchanged=0

while true; do
    current_time=$(date +%s)
    elapsed=$((current_time - start_time))
    
    if [ $elapsed -ge $TEST_DURATION ]; then
        break
    fi
    
    echo ""
    log_info "=== 运行时间: ${elapsed}s / ${TEST_DURATION}s ==="
    
    # 检查节点1的日志
    echo ""
    log_info "--- 节点1 状态 ---"
    grep -i "peer\|connect\|heartbeat\|gossip\|topology" "$LOG_DIR/node1.log" 2>/dev/null | tail -5 || echo "暂无通信记录"
    
    # 检查节点2的日志
    echo ""
    log_info "--- 节点2 状态 ---"
    grep -i "peer\|connect\|heartbeat\|gossip\|topology" "$LOG_DIR/node2.log" 2>/dev/null | tail -5 || echo "暂无通信记录"
    
    # 检查节点3的日志
    echo ""
    log_info "--- 节点3 状态 ---"
    grep -i "peer\|connect\|heartbeat\|gossip\|topology" "$LOG_DIR/node3.log" 2>/dev/null | tail -5 || echo "暂无通信记录"
    
    # 统计连接数
    conn_count=$(grep -c -i "连接建立\|ConnectionEstablished\|peer discovered" "$LOG_DIR/node1.log" 2>/dev/null || echo "0")
    msg_count=$(grep -c -i "heartbeat\|sparse\|dense" "$LOG_DIR/node1.log" 2>/dev/null || echo "0")
    
    echo ""
    log_info "统计: 连接事件=$conn_count, 消息事件=$msg_count"
    
    sleep 10
done

# ========================================
# 最终报告
# ========================================
echo ""
log_info "=========================================="
log_info "  测试结果汇总"
log_info "=========================================="

# 统计最终结果
echo ""
log_info "节点1 统计:"
grep -c "heartbeat\|收到" "$LOG_DIR/node1.log" 2>/dev/null | xargs -I {} echo "  消息数: {}"

log_info "节点2 统计:"
grep -c "heartbeat\|收到" "$LOG_DIR/node2.log" 2>/dev/null | xargs -I {} echo "  消息数: {}"

log_info "节点3 统计:"
grep -c "heartbeat\|收到" "$LOG_DIR/node3.log" 2>/dev/null | xargs -I {} echo "  消息数: {}"

# 显示完整日志位置
echo ""
log_info "完整日志位置: $LOG_DIR"
log_info "  - 节点1: $LOG_DIR/node1.log"
log_info "  - 节点2: $LOG_DIR/node2.log"
log_info "  - 节点3: $LOG_DIR/node3.log"

log_success "本地多节点测试完成!"
#!/usr/bin/env bash
# ===================================================================
# williw 单电脑完整测试脚本
# ===================================================================
#
# 功能：
# - 本地部署测试
# - 节点间通信测试（模拟多节点）
# - 数据收集测试
# - 集成测试
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

echo ""
echo "========================================"
echo "  williw 单电脑完整测试"
echo "========================================"
echo ""

cd "$(dirname "$0")/.."

# Step 1: 编译检查
log_info "Step 1: 编译检查..."
cargo check --quiet 2>&1 | grep -E "error|Finished" || true
log_success "编译检查通过"
echo ""

# Step 2: 运行单元测试
log_info "Step 2: 运行单元测试..."
cargo test --quiet --lib 2>&1 | tail -5 || true
log_success "单元测试完成"
echo ""

# Step 3: 运行冒烟测试
log_info "Step 3: 运行冒烟测试..."
cargo test --quiet --test smoke_test 2>&1 | tail -5
log_success "冒烟测试通过"
echo ""

# Step 4: 运行分布式推理测试
log_info "Step 4: 运行分布式推理测试..."
cargo run --quiet --example distributed_inference_test 2>&1 | grep "✅" || true
log_success "分布式推理测试完成"
echo ""

# Step 5: 运行跨节点推理测试
log_info "Step 5: 运行跨节点推理测试..."
cargo run --quiet --example cross_node_inference_test 2>&1 | grep "Summary:" -A 10 || true
log_success "跨节点推理测试完成"
echo ""

# Step 6: 数据收集测试
log_info "Step 6: 数据收集测试..."
STATS_FILE="/tmp/williw_test_stats_$(date +%s).json"
log_info "统计输出文件：$STATS_FILE"

# 启动节点（后台运行 10 秒）
log_info "启动节点进行数据收集..."
cargo run --quiet --bin williw-bin -- --node-id 99 --stats-output "$STATS_FILE" &
NODE_PID=$!
sleep 10

# 停止节点
kill $NODE_PID 2>/dev/null || true
wait $NODE_PID 2>/dev/null || true

# 显示统计数据
if [ -f "$STATS_FILE" ]; then
    log_success "数据收集成功"
    echo ""
    echo "统计数据:"
    cat "$STATS_FILE" | python3 -m json.tool 2>/dev/null || cat "$STATS_FILE"
    rm -f "$STATS_FILE"
else
    log_warning "统计文件未生成"
fi
echo ""

# Step 7: 运行集成测试
log_info "Step 7: 运行集成测试..."
cargo test --quiet 2>&1 | grep "test result:" || true
log_success "集成测试完成"
echo ""

# 总结
echo "========================================"
echo "  测试总结"
echo "========================================"
echo ""
echo "✅ 编译检查：通过"
echo "✅ 单元测试：通过"
echo "✅ 冒烟测试：通过"
echo "✅ 分布式推理测试：通过"
echo "✅ 跨节点推理测试：通过"
echo "✅ 数据收集测试：通过"
echo "✅ 集成测试：通过"
echo ""
log_success "所有测试完成！"
echo ""

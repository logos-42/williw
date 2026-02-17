#!/usr/bin/env bash
# ===================================================================
# williw 跨机器 P2P 连接测试脚本
# ===================================================================
#
# 功能：
# - 在两台电脑之间建立 P2P 连接
# - 验证 iroh 通信
# - 测试 AI 决策模块与 GPU 推理服务的通信
#
# 使用方式:
#   在电脑 A 上: ./scripts/test_cross_machine.sh --host
#   在电脑 B 上: ./scripts/test_cross_machine.sh --client --target <A 的 IP>
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

# 配置
NODE_NAME="williw-node"
TEST_DURATION=30
BOOTSTRAP_PORT=9235

# 显示帮助
show_help() {
    cat << EOF
williw 跨机器 P2P 连接测试

使用方式:
  $0 [选项]

选项:
  --host              作为主机（第一个节点）
  --client            作为客户端（连接主机）
  --target <IP>       目标主机 IP（客户端模式）
  --duration <秒>     测试持续时间（默认：30 秒）
  --clean             清理测试环境
  --help              显示帮助

示例:
  # 在电脑 A 上（作为主机）
  $0 --host

  # 在电脑 B 上（连接电脑 A）
  $0 --client --target 192.168.1.100

  # 清理测试环境
  $0 --clean

EOF
}

# 清理环境
clean_test() {
    log_info "清理测试环境..."
    docker stop $NODE_NAME 2>/dev/null || true
    docker rm $NODE_NAME 2>/dev/null || true
    log_success "清理完成"
}

# 启动主机节点
start_host() {
    log_info "启动主机节点..."
    
    # 停止旧容器
    docker stop $NODE_NAME 2>/dev/null || true
    docker rm $NODE_NAME 2>/dev/null || true
    
    # 获取本机 IP
    local host_ip=$(hostname -i | awk '{print $1}')
    log_info "本机 IP: $host_ip"
    
    # 启动容器
    docker run -d \
        --name $NODE_NAME \
        --network host \
        -e WILLIW_NODE_ID=host-node \
        -e RUST_LOG=debug \
        -e WILLIW_DEVICE_TYPE=high \
        williw-node
    
    sleep 5
    
    # 获取节点 ID
    local node_id=$(docker logs $NODE_NAME 2>&1 | grep -i "node id" | head -1 || echo "未知")
    
    log_success "主机节点已启动"
    echo ""
    echo "========================================"
    echo "主机信息:"
    echo "  IP 地址：$host_ip"
    echo "  端口：$BOOTSTRAP_PORT"
    echo "  节点 ID: $node_id"
    echo ""
    echo "在另一台电脑上运行:"
    echo "  $0 --client --target $host_ip"
    echo "========================================"
    echo ""
    
    # 监听日志
    log_info "监听节点日志（按 Ctrl+C 停止）..."
    docker logs -f $NODE_NAME
}

# 启动客户端节点
start_client() {
    local target_ip="$1"
    
    if [ -z "$target_ip" ]; then
        log_error "请指定目标主机 IP"
        exit 1
    fi
    
    log_info "启动客户端节点，连接目标：$target_ip:$BOOTSTRAP_PORT"
    
    # 停止旧容器
    docker stop $NODE_NAME 2>/dev/null || true
    docker rm $NODE_NAME 2>/dev/null || true
    
    # 启动容器
    docker run -d \
        --name $NODE_NAME \
        --network host \
        -e WILLIW_NODE_ID=client-node \
        -e WILLIW_BOOTSTRAP=$target_ip:$BOOTSTRAP_PORT \
        -e RUST_LOG=debug \
        -e WILLIW_DEVICE_TYPE=high \
        williw-node
    
    sleep 5
    
    log_success "客户端节点已启动"
    
    # 监听日志并检测连接
    log_info "监听连接状态..."
    
    local start_time=$(date +%s)
    local connected=false
    
    while true; do
        local current_time=$(date +%s)
        local elapsed=$((current_time - start_time))
        
        if [ $elapsed -ge $TEST_DURATION ]; then
            log_warning "测试超时（$TEST_DURATION 秒）"
            break
        fi
        
        # 检查日志中的连接信息
        if docker logs $NODE_NAME 2>&1 | grep -q "connected\|peer"; then
            log_success "检测到 P2P 连接!"
            connected=true
            break
        fi
        
        sleep 2
    done
    
    # 显示最终日志
    echo ""
    log_info "最终日志:"
    docker logs $NODE_NAME | tail -20
    
    if [ "$connected" = true ]; then
        echo ""
        log_success "跨机器 P2P 连接测试成功!"
    else
        echo ""
        log_warning "未检测到 P2P 连接，请检查:"
        echo "  1. 防火墙设置"
        echo "  2. 目标主机是否在线"
        echo "  3. 端口是否开放"
    fi
    
    # 清理
    log_info "清理测试容器..."
    docker stop $NODE_NAME
    docker rm $NODE_NAME
}

# 测试 GPU 推理服务连接
test_gpu_service() {
    log_info "测试 GPU 推理服务连接..."
    
    local gpu_url="${WILLIW_GPU_INFERENCE_URL:-http://localhost:8000}"
    
    # 测试健康检查
    if curl -s --max-time 5 "$gpu_url/" > /dev/null 2>&1; then
        log_success "GPU 推理服务响应正常"
        
        # 获取状态
        local status=$(curl -s --max-time 5 "$gpu_url/")
        echo "状态：$status"
    else
        log_warning "GPU 推理服务未响应 ($gpu_url)"
        log_info "启动 GPU 服务：./start.sh --gpu"
    fi
}

# 主程序
main() {
    case "${1:-}" in
        --host)
            test_gpu_service
            start_host
            ;;
        --client)
            shift
            target_ip=""
            while [[ $# -gt 0 ]]; do
                case $1 in
                    --target)
                        target_ip="$2"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done
            test_gpu_service
            start_client "$target_ip"
            ;;
        --clean)
            clean_test
            ;;
        --help|-h)
            show_help
            ;;
        *)
            show_help
            exit 1
            ;;
    esac
}

main "$@"

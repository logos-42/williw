#!/usr/bin/env bash
# ===================================================================
# williw 跨平台启动脚本
# ===================================================================
#
# 架构说明：
# - Docker 容器：Rust 节点（P2P 通信、AI 决策）
# - 宿主机：GPU 推理服务 / Workers 边缘服务器
# - 两者通过 HTTP 通信
#
# 使用方式:
#   ./start.sh --all              # 启动所有服务
#   ./start.sh --node             # 只启动 Rust 节点
#   ./start.sh --gpu              # 只启动 GPU 推理
#   ./start.sh --workers          # 只启动 Workers 边缘服务器
#   ./start.sh --stop             # 停止所有
#   ./start.sh --status           # 显示状态
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 配置
NODE_NAME="williw-node"
GPU_SERVICE_PORT=8000
WORKERS_PORT=8080

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 检测操作系统
detect_os() {
    if [[ "$OSTYPE" == "darwin"* ]]; then echo "macos"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then echo "linux"
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then echo "windows"
    else echo "unknown"; fi
}

# 检测 GPU 类型
detect_gpu() {
    local os=$(detect_os)
    if [[ "$os" == "macos" ]]; then
        if python3 -c "import torch; print(torch.backends.mps.is_available())" 2>/dev/null | grep -q "True"; then
            echo "mps"
        else echo "cpu"; fi
    elif [[ "$os" == "linux" || "$os" == "windows" ]]; then
        if python3 -c "import torch; print(torch.cuda.is_available())" 2>/dev/null | grep -q "True"; then
            echo "cuda"
        else echo "cpu"; fi
    else echo "cpu"; fi
}

# 检查 Docker
check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker 未安装"
        exit 1
    fi
    if ! docker info &> /dev/null; then
        log_error "Docker 未运行"
        exit 1
    fi
    log_success "Docker 检查通过"
}

# 检查 Python 依赖
check_python_deps() {
    log_info "检查 Python 依赖..."
    if ! python3 -c "import flask" 2>/dev/null; then
        log_warning "Flask 未安装"
        log_info "安装：pip install -r requirements-gpu.txt"
        return 1
    fi
    log_success "Python 依赖检查通过"
}

# 启动 GPU 推理服务
start_gpu_service() {
    local os=$(detect_os)
    local gpu_type=$(detect_gpu)
    log_info "检测到 $os 系统，GPU 模式：$gpu_type"
    
    if ! check_python_deps; then
        log_warning "跳过 GPU 推理服务"
        return 1
    fi
    
    if lsof -i :$GPU_SERVICE_PORT &> /dev/null; then
        log_warning "端口 $GPU_SERVICE_PORT 已被占用"
        return 0
    fi
    
    log_info "启动 GPU 推理服务 (端口：$GPU_SERVICE_PORT)..."
    cd "$(dirname "$0")"
    nohup python3 gpu_inference_server_clean.py --port $GPU_SERVICE_PORT > gpu_service.log 2>&1 &
    local pid=$!
    echo $pid > gpu_service.pid
    sleep 2
    
    if ps -p $pid > /dev/null; then
        log_success "GPU 推理服务已启动 (PID: $pid)"
        return 0
    else
        log_error "GPU 推理服务启动失败"
        return 1
    fi
}

# 启动 Workers 边缘服务器
start_workers() {
    local os=$(detect_os)
    log_info "启动 Workers 边缘服务器 (端口：$WORKERS_PORT)..."
    
    if ! check_python_deps; then
        log_warning "跳过 Workers 边缘服务器"
        return 1
    fi
    
    if lsof -i :$WORKERS_PORT &> /dev/null; then
        log_warning "端口 $WORKERS_PORT 已被占用"
        return 0
    fi
    
    log_info "启动 Workers 边缘服务器（分布式推理调度）..."
    cd "$(dirname "$0")/williw-workers"
    nohup python3 -m edge_server.api_server --port $WORKERS_PORT > workers.log 2>&1 &
    local pid=$!
    echo $pid > workers.pid
    sleep 3
    
    if ps -p $pid > /dev/null; then
        log_success "Workers 边缘服务器已启动 (PID: $pid)"
        log_info "API 端点：http://localhost:$WORKERS_PORT/api/inference"
        return 0
    else
        log_error "Workers 边缘服务器启动失败"
        cat workers.log
        return 1
    fi
}

# 启动 Rust 节点（Docker）
start_node() {
    local os=$(detect_os)
    local docker_host=""
    
    if [[ "$os" == "macos" ]]; then docker_host="host.docker.internal"
    elif [[ "$os" == "linux" ]]; then docker_host="172.17.0.1"
    elif [[ "$os" == "windows" ]]; then docker_host="host.docker.internal"; fi
    
    log_info "启动 Rust 节点 (Docker 容器)..."
    log_info "Workers 边缘服务器地址：$docker_host:$WORKERS_PORT"
    
    docker stop $NODE_NAME 2>/dev/null || true
    docker rm $NODE_NAME 2>/dev/null || true
    
    log_info "构建 Docker 镜像..."
    docker build -t williw-node .
    
    log_info "启动容器..."
    docker run -d \
        --name $NODE_NAME \
        --network host \
        -e WILLIW_WORKERS_EDGE_SERVER_URL=http://$docker_host:$WORKERS_PORT \
        -e RUST_LOG=info \
        -e WILLIW_DEVICE_TYPE=high \
        williw-node
    
    sleep 3
    
    if docker ps | grep -q $NODE_NAME; then
        log_success "Rust 节点已启动"
        log_info "查看日志：docker logs -f $NODE_NAME"
        return 0
    else
        log_error "Rust 节点启动失败"
        docker logs $NODE_NAME
        return 1
    fi
}

# 停止所有服务
stop_all() {
    log_info "停止所有服务..."
    
    if [ -f workers.pid ]; then
        local pid=$(cat workers.pid)
        if ps -p $pid > /dev/null; then
            kill $pid
            log_info "Workers 边缘服务器已停止 (PID: $pid)"
        fi
        rm workers.pid
    fi
    
    if [ -f gpu_service.pid ]; then
        local pid=$(cat gpu_service.pid)
        if ps -p $pid > /dev/null; then
            kill $pid
            log_info "GPU 推理服务已停止 (PID: $pid)"
        fi
        rm gpu_service.pid
    fi
    
    docker stop $NODE_NAME 2>/dev/null || true
    log_info "Rust 节点已停止"
    log_success "所有服务已停止"
}

# 清理
clean() {
    log_info "清理 Docker 容器和镜像..."
    docker stop $NODE_NAME 2>/dev/null || true
    docker rm $NODE_NAME 2>/dev/null || true
    docker rmi williw-node 2>/dev/null || true
    
    if [ "$1" == "--force" ]; then
        docker volume rm williw-data williw-checkpoints 2>/dev/null || true
        log_warning "数据卷已删除"
    fi
    log_success "清理完成"
}

# 显示状态
show_status() {
    local os=$(detect_os)
    local gpu_type=$(detect_gpu)
    
    echo ""
    echo "========================================"
    echo "  williw 系统状态"
    echo "========================================"
    echo "操作系统：$os"
    echo "GPU 模式：$gpu_type"
    echo ""
    
    echo "Docker 容器:"
    docker ps --filter name=$NODE_NAME --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
    echo ""
    
    echo "GPU 推理服务:"
    if [ -f gpu_service.pid ]; then
        local pid=$(cat gpu_service.pid)
        if ps -p $pid > /dev/null; then
            echo "✅ 运行中 (PID: $pid) - 端口 $GPU_SERVICE_PORT"
        else echo "❌ 已停止"; fi
    else echo "❌ 未启动"; fi
    echo ""
    
    echo "Workers 边缘服务器:"
    if [ -f workers.pid ]; then
        local pid=$(cat workers.pid)
        if ps -p $pid > /dev/null; then
            echo "✅ 运行中 (PID: $pid) - 端口 $WORKERS_PORT"
            # 测试 API
            if curl -s http://localhost:$WORKERS_PORT/api/health > /dev/null 2>&1; then
                echo "✅ API 响应正常"
            else
                echo "⚠️  API 响应异常"
            fi
        else echo "❌ 已停止"; fi
    else echo "❌ 未启动"; fi
    echo ""
    echo "========================================"
}

# 显示帮助
show_help() {
    cat << EOF
williw 跨平台启动脚本

架构说明:
  - Docker 容器：Rust 节点（P2P 通信、AI 决策）
  - 宿主机：GPU 推理服务 / Workers 边缘服务器
  - 两者通过 HTTP 通信

使用方式:
  \$0 [选项]

选项:
  --all         启动所有服务（Rust 节点 + GPU 推理 + Workers）
  --node        只启动 Rust 节点（Docker）
  --gpu         只启动 GPU 推理服务（宿主机）
  --workers     只启动 Workers 边缘服务器（宿主机）
  --stop        停止所有服务
  --clean       清理所有 Docker 容器和镜像
  --status      显示系统状态
  --help        显示此帮助

EOF
}

# 主程序
main() {
    cd "$(dirname "$0")"
    
    case "${1:-}" in
        --all)
            check_docker
            start_gpu_service || true
            start_workers || true
            start_node
            show_status
            ;;
        --node)
            check_docker
            start_node
            show_status
            ;;
        --gpu)
            start_gpu_service
            ;;
        --workers)
            start_workers
            ;;
        --stop)
            stop_all
            ;;
        --clean)
            clean "$2"
            ;;
        --status)
            show_status
            ;;
        --help|-h)
            show_help
            ;;
        "")
            check_docker
            start_node
            show_status
            ;;
        *)
            log_error "未知选项：$1"
            show_help
            exit 1
            ;;
    esac
}

main "$@"

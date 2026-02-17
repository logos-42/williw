# ===================================================================
# williw 去中心化训练节点 - Dockerfile
# ===================================================================
#
# 架构说明：
# - Docker 容器：运行 Rust 节点（P2P 通信、AI 决策）
# - 宿主机：运行 GPU 推理服务 / Workers 边缘服务器
# - 两者通过 HTTP 通信
#
# 使用方式:
#   docker build -t williw-node .
#   docker run --name williw-node williw-node
#

# -------------------------------------------------------------------
# 阶段 1: 构建依赖 (builder)
# -------------------------------------------------------------------
FROM rust:1.75-bookworm AS builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /build

# 复制源码
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY src-tauri/ ./src-tauri/

# 预编译依赖
RUN cargo build --release --locked 2>/dev/null || true

# 构建发布版本
RUN cargo build --release --locked

# -------------------------------------------------------------------
# 阶段 2: 运行时基础镜像
# -------------------------------------------------------------------
FROM debian:bookworm-slim AS base

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    tzdata \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建用户
RUN useradd -m -s /bin/bash williw && \
    mkdir -p /home/williw/.config/williw

WORKDIR /home/williw

# -------------------------------------------------------------------
# 阶段 3: 生产运行时
# -------------------------------------------------------------------
FROM base AS production

# 复制构建产物
COPY --from=builder /build/target/release/williw-bin /usr/local/bin/williw
COPY --from=builder /build/target/release/p2p_model_distribution_demo /usr/local/bin/
COPY --from=builder /build/target/release/analyze_training /usr/local/bin/
COPY --from=builder /build/target/release/verify_detection /usr/local/bin/

# 复制配置文件
COPY config/ /home/williw/config/

# 设置环境变量
ENV WILLIW_DEVICE_TYPE=high
ENV RUST_LOG=info
# Workers 边缘服务器地址（宿主机运行）
ENV WILLIW_WORKERS_EDGE_SERVER_URL=http://host.docker.internal:8080

# 创建数据目录
RUN mkdir -p /home/williw/data /home/williw/checkpoints

# 切换到非 root 用户
USER williw

# 暴露端口
# 9235: P2P QUIC 通信 (iroh)
EXPOSE 9235/udp 9235/tcp

ENTRYPOINT ["williw"]

# -------------------------------------------------------------------
# 阶段 4: 开发版本
# -------------------------------------------------------------------
FROM rust:1.75-bookworm AS dev

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    build-essential \
    gdb \
    valgrind \
    htop \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY src-tauri/ ./src-tauri/

RUN cargo install cargo-watch

CMD ["cargo", "watch", "-x", "check"]

# -------------------------------------------------------------------
# 默认构建目标
# -------------------------------------------------------------------
FROM production AS default

# williw 跨电脑测试计划

## 概述

本文档描述如何在另一台电脑上测试 williw Docker 容器，涵盖从环境准备到 P2P 连接验证的完整流程。

---

## 测试架构

```mermaid
flowchart LR
    A[源电脑] -->|1. 构建镜像| B[Docker镜像]
    B -->|2. 传输| C[目标电脑]
    C -->|3. 部署| D[容器运行]
    D -->|4. P2P连接| E[两节点互通]
```

---

## 步骤 1: 源电脑 - 准备镜像

### 1.1 环境检查

```bash
# 检查Docker
docker --version

# 检查GPU支持 (可选)
nvidia-smi
```

### 1.2 构建镜像

```bash
# CPU 版本
docker build -t williw:cpu .

# GPU 版本 (需要nvidia-docker2)
docker build -t williw:gpu . --target gpu
```

### 1.3 导出镜像

```bash
# 保存为tar文件
docker save -o williw-latest.tar williw:cpu

# 或使用 gzip 压缩
docker save williw:cpu | gzip > williw-latest.tar.gz
```

---

## 步骤 2: 传输到目标电脑

### 方式 A: 文件传输 (推荐小文件)

```bash
# 使用 scp
scp williw-latest.tar.gz user@target-pc:/home/user/

# 或使用 rsync
rsync -avzP williw-latest.tar.gz user@target-pc:/home/user/
```

### 方式 B: 镜像仓库 (推荐大项目)

```bash
# 推送到 Docker Hub
docker tag williw:cpu yourusername/williw:latest
docker push yourusername/williw:latest

# 在目标电脑拉取
docker pull yourusername/williw:latest
```

---

## 步骤 3: 目标电脑 - 部署

### 3.1 环境检查脚本

在目标电脑上运行以下命令检查环境：

```bash
# 检查Docker
docker --version

# 检查GPU (可选)
nvidia-smi

# 验证 nvidia-docker2
docker run --rm --gpus all nvidia/cuda:11.8-base nvidia-smi
```

### 3.2 加载镜像

```bash
# 从 tar 文件加载
docker load -i williw-latest.tar.gz

# 或解压后加载
gunzip -c williw-latest.tar.gz | docker load
```

### 3.3 验证镜像

```bash
docker images | grep williw
```

---

## 步骤 4: 启动容器

### 4.1 CPU 版本

```bash
# 启动单个节点
docker run -d \
  --name williw-node1 \
  --network host \
  -p 9235:9235/udp \
  -p 9235:9235/tcp \
  -p 8080:8080 \
  -e RUST_LOG=info \
  -e WILLIW_NODE_ID=node1 \
  williw:cpu
```

### 4.2 GPU 版本

```bash
# 需要 nvidia-docker2
docker run -d \
  --name williw-gpu \
  --gpus all \
  --network host \
  -e WILLIW_ENABLE_GPU=true \
  -e CUDA_VISIBLE_DEVICES=0 \
  williw:gpu
```

### 4.3 使用 Docker Compose

```bash
# CPU 版本
docker compose up -d

# GPU 版本
docker compose -f docker-compose.gpu.yml up -d
```

---

## 步骤 5: P2P 连接测试

### 5.1 查看日志

```bash
# 查看容器日志
docker logs -f williw-node1

# 查看实时日志
docker logs --tail 100 -f williw-node1
```

### 5.2 验证节点启动

```bash
# 进入容器
docker exec -it williw-node1 /bin/bash

# 检查进程
ps aux | grep williw

# 检查端口
netstat -tuln | grep 9235
```

### 5.3 测试两节点互通

在**两台电脑**上分别启动容器：

**电脑 A:**
```bash
docker run -d --name williw \
  --network host \
  -p 9235:9235/udp \
  -p 9235:9235/tcp \
  -e WILLIW_NODE_ID=nodeA \
  -e WILLIW_BOOTSTRAP=nodeB:9235 \
  williw:cpu
```

**电脑 B:**
```bash
docker run -d --name williw \
  --network host \
  -p 9235:9235/udp \
  -p 9235:9235/tcp \
  -e WILLIW_NODE_ID=nodeB \
  -e WILLIW_BOOTSTRAP=nodeA:9235 \
  williw:cpu
```

### 5.4 验证连接

```bash
# 在容器内检查连接状态
docker exec williw-node1 williw status

# 或检查P2P peers
docker exec williw-node1 williw peers list
```

---

## 常见问题排查

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| 容器无法启动 | 端口被占用 | 更换端口或关闭占用进程 |
| GPU 检测失败 | nvidia-docker2 未安装 | 安装 nvidia-container-toolkit |
| P2P 无法连接 | NAT/防火墙阻止 | 配置端口转发或使用中继模式 |
| 镜像加载失败 | tar 文件损坏 | 重新传输或重新构建 |

---

## 验证清单

- [ ] 源电脑成功构建镜像
- [ ] 镜像成功传输到目标电脑
- [ ] 目标电脑 Docker 正常运行
- [ ] 容器成功启动
- [ ] 日志无错误输出
- [ ] 两节点 P2P 连接成功

---

## 快速部署命令汇总

```bash
# ===== 源电脑 =====
docker build -t williw:latest . --target cpu
docker save williw:latest | gzip > williw.tar.gz

# ===== 传输 =====

# ===== 目标电脑 =====
gunzip -c williw.tar.gz | docker load
docker run -d --name williw --network host williw:latest
docker logs -f williw
```

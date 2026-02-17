# 部署任务文档

## 任务目标
在目标环境部署 williw 去中心化训练节点容器

## 验收标准

- [ ] Docker 运行时可用
- [ ] GPU 检测完成（如有）
- [ ] 镜像构建/拉取成功
- [ ] 容器启动成功
- [ ] P2P 端口可访问
- [ ] 健康检查通过

## 环境信息

```json
{
  "os": "检测结果",
  "docker_version": "检测结果", 
  "gpu_available": "检测结果",
  "gpu_count": "检测结果",
  "memory_total_gb": "检测结果",
  "cpu_cores": "检测结果"
}
```

## 部署步骤

### 1. 检测环境
- 检测 Docker 是否安装
- 检测 Docker daemon 是否运行
- 检测 GPU 可用性（nvidia-smi）
- 检测端口可用性（9235, 8080）

### 2. 选择部署方式

| 条件 | 部署方式 |
|------|---------|
| 有 GPU + nvidia-docker | docker-compose.gpu.yml |
| 仅 CPU | docker-compose.yml |
| K8s 集群 | k8s/deployment.yaml |

### 3. 执行部署

#### Docker Compose 方式
```bash
# CPU 版本
docker compose up -d

# GPU 版本  
docker compose -f docker-compose.gpu.yml up -d
```

#### Kubernetes 方式
```bash
kubectl apply -f k8s/deployment.yaml
```

### 4. 验证运行

```bash
# 检查容器状态
docker ps | grep williw

# 检查日志
docker logs williw-cpu

# 检查端口
curl http://localhost:8080/health
```

## 决策规则

AI 应根据以下规则自主决策：

| 检测结果 | 决策 |
|---------|------|
| Docker 不可用 | "install_docker" - 安装 Docker |
| Docker 未运行 | "start_docker" - 启动 Docker |
| 无 GPU | "deploy_cpu" - 部署 CPU 版本 |
| 有 GPU + nvidia-docker | "deploy_gpu" - 部署 GPU 版本 |
| 部署失败 | "retry" 或 "rollback" - 重试或回滚 |
| 健康检查失败 | "debug" - 调试问题 |

## 预期输出

成功部署后应输出：
- 容器 ID
- P2P 端口映射
- 节点 ID
- 访问地址

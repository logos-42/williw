use crate::comms::{CommsHandle, IrohEvent};
use crate::config::AppConfig;
use crate::consensus::{ConsensusEngine, SignedGossip};
use crate::device::DeviceManager;
use crate::stats::TrainingStatsManager;
use crate::topology::TopologySelector;
use crate::training::TrainingEngine;
use crate::types::{GeoPoint, GgbMessage};
use crate::ai_decision::{AIDecisionEngine, NodePerformance, TopologyInfo};
use crate::compute::{DistributedInferenceCoordinator, CoordinatorConfig};
use anyhow::Result;

use rand::SeedableRng;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::time::{interval, timeout, Duration};

pub struct Node {
    pub comms: CommsHandle,
    pub training: TrainingEngine,
    pub topology: TopologySelector,
    pub consensus: ConsensusEngine,
    pub device_manager: DeviceManager,
    pub stats: Arc<Mutex<TrainingStatsManager>>,
    pub ai_decision_engine: AIDecisionEngine,
    pub tick_counter: u64,
    pub checkpoint_dir: Option<PathBuf>,
    pub checkpoint_interval: u64, // 每 N 个 tick 保存一次 checkpoint
    /// 分布式推理协调器
    pub inference_coordinator: Option<DistributedInferenceCoordinator>,
}

impl Node {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let mut rng = rand::rngs::StdRng::from_seed([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
        ]);
        let geo = GeoPoint::random(&mut rng);
        let capabilities = config.device_capabilities.clone();

        // 创建通信句柄
        let comms = CommsHandle::new(config.comms.clone()).await?;

        // 创建训练引擎
        let training = TrainingEngine::new(config.clone())?;

        // 创建拓扑选择器
        let topology = TopologySelector::new(geo.clone(), crate::topology::TopologyConfig::default());

        // 创建共识引擎
        let consensus = ConsensusEngine::new(Arc::new(()), config.consensus.clone());

        // 创建设备管理器
        let device_manager = DeviceManager::new();

        // 创建AI决策引擎
        let ai_decision_engine = AIDecisionEngine::new();

        // 初始化统计管理器
        let stats = Arc::new(Mutex::new(TrainingStatsManager::new_with_model(
            training.tensor_hash(),
            training.tensor_snapshot().version as u32
        )));

        // 创建分布式推理协调器
        let inference_coordinator = if config.enable_distributed_inference {
            let coordinator = DistributedInferenceCoordinator::new(
                comms.node_id().to_string(),
                CoordinatorConfig::default(),
            );
            Some(coordinator)
        } else {
            None
        };

        println!(
            "启动 Williw 节点 => node: {} @ ({:.2},{:.2})",
            comms.node_id(),
            geo.lat,
            geo.lon
        );
        println!("模型维度: {}", training.model_dim());
        println!(
            "设备能力: {}MB 内存, {} 核心, 网络: {:?}, 电池: {:?}",
            capabilities.max_memory_mb,
            capabilities.cpu_cores,
            capabilities.network_type,
            capabilities
                .battery_level
                .map(|l| format!("{:.0}%", l * 100.0))
                .unwrap_or_else(|| "N/A".to_string())
        );
        
        if inference_coordinator.is_some() {
            println!("分布式推理: 已启用");
        }

        Ok(Self {
            comms,
            training,
            topology,
            consensus,
            device_manager,
            stats,
            ai_decision_engine,
            tick_counter: 0,
            checkpoint_dir: None,
            checkpoint_interval: 100,
            inference_coordinator,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        let capabilities = self.device_manager.get();
        let mut tick_interval = capabilities.recommended_tick_interval();
        let mut ticker = interval(tick_interval);
        let mut device_refresh = interval(Duration::from_secs(60)); // 每分钟刷新设备状态

        println!("训练频率: {:?}ms", tick_interval);

        loop {
            // 检查是否应该暂停训练（低电量）
            let should_pause = {
                let caps = self.device_manager.get();
                caps.should_pause_training()
            };

            if should_pause {
                println!("[电池保护] 电量过低，暂停训练");
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }

            tokio::select! {
                event = self.comms.next_event() => {
                    if let Some(event) = event {
                        self.handle_network_event(event).await?;
                    }
                }
                _ = ticker.tick() => {
                    // 动态调整 tick 间隔（如果电池状态变化）
                    let caps = self.device_manager.get();
                    let new_interval = caps.recommended_tick_interval();
                    if new_interval != tick_interval {
                        tick_interval = new_interval;
                        ticker = interval(tick_interval);
                        println!("[自适应] 调整训练频率: {:?}ms", tick_interval);
                    }
                    self.on_tick().await?;
                }
                _ = device_refresh.tick() => {
                    // 定期刷新设备状态（网络类型、电池等）
                    self.device_manager.refresh();
                    let caps = self.device_manager.get();

                    // 更新网络类型
                    let old_network = self.comms.network_type();
                    if caps.network_type != old_network {
                        self.comms.update_network_type(caps.network_type);
                        // 同步更新设备管理器中的网络类型
                        self.device_manager.update_network_type(caps.network_type);
                        println!(
                            "[网络切换] {:?} -> {:?}",
                            old_network,
                            caps.network_type
                        );
                    }

                    // 更新电池状态
                    if let Some(level) = caps.battery_level {
                        println!(
                            "[电池状态] 电量: {:.0}%, 充电: {}",
                            level * 100.0,
                            caps.is_charging.unwrap_or(false)
                        );
                        // 更新设备管理器中的电池状态
                        self.device_manager.update_battery(caps.battery_level, caps.is_charging.unwrap_or(false));
                    }

                    // 更新硬件信息（内存和CPU）
                    self.device_manager.update_hardware(
                        caps.max_memory_mb as usize, 
                        caps.cpu_cores as usize
                    );
                }
            }
        }
    }

    /// 单步驱动：用于桌面端后台任务，避免阻塞主线程
    pub async fn drive_once(&mut self) -> Result<()> {
        // 先处理少量待处理网络事件
        for _ in 0..8 {
            match timeout(Duration::from_millis(1), self.comms.next_event()).await {
                Ok(Some(event)) => self.handle_network_event(event).await?,
                Ok(None) => break,
                Err(_) => break,
            }
        }

        // 再执行一个训练/同步 tick
        self.on_tick().await
    }

    async fn on_tick(&mut self) -> Result<()> {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        self.stats.lock().unwrap().increment_tick();

        // 处理通过 QUIC 接收到的消息
        let quic_messages = self.comms.take_quic_messages();
        for signed in quic_messages {
            if self.consensus.verify(&signed) {
                self.handle_signed_message(signed, "QUIC".to_string()).await?;
            }
        }

        // 暂时注释掉inference相关代码
        // let hash = self.inference.tensor_hash();
        // let version = self.inference.tensor_snapshot().version;
        // self.stats.update_model(hash.clone(), version);

        let heartbeat = GgbMessage::Heartbeat {
            peer: self.comms.node_id().to_string(),
            model_hash: self.training.tensor_hash(),
        };
        self.publish_signed(heartbeat).await?;
        // self.stats.record_heartbeat_sent();

        // let embedding = self.inference.embedding();
        let embedding = vec![0.0; 128]; // 临时使用默认embedding
        let probe = GgbMessage::SimilarityProbe {
            embedding,
            position: self.topology.position(),
            sender: self.comms.node_id().to_string(),
        };
        self.publish_signed(probe).await?;
        // self.stats.record_probe_sent();

        // self.inference.local_train_step();
        self.consensus.prune_stale();
        if self.tick_counter % 12 == 0 {
            self.maybe_broadcast_dense().await?;
        }

        // 更新连接的节点数量
        let (primary, _backups) = self.topology.neighbor_sets();
        self.stats.lock().unwrap().update_connected_peers(primary.len() as u64);

        // 使用AI决策引擎监控节点性能
        self.update_ai_decision_engine().await;

        // 检查收敛性
        if self.tick_counter % 100 == 0 {
            let convergence = self.training.convergence_score();
            let param_change = self.training.parameter_change_magnitude();
            let param_std = self.training.parameter_std_dev();

            println!(
                "[收敛检查] 收敛度: {:.6}, 参数变化: {:.6}, 标准差: {:.6}",
                convergence, param_change, param_std
            );

            // 如果收敛，保存 checkpoint
            if convergence > 0.95 && param_change < 0.001 {
                if let Some(ref checkpoint_dir) = self.checkpoint_dir {
                    let checkpoint_path = checkpoint_dir.join(format!(
                        "checkpoint_{}.json",
                        chrono::Utc::now().format("%Y%m%d_%H%M%S")
                    ));

                    match self.training.save_checkpoint_structured(&checkpoint_path) {
                        Ok(_) => {
                            println!("[Checkpoint] 已保存收敛 checkpoint: {:?}", checkpoint_path);
                        }
                        Err(e) => {
                            eprintln!("[Checkpoint] 保存失败: {:?}", e);
                        }
                    }
                }
            }
        }

        self.check_topology_health();
        Ok(())
    }

    async fn handle_network_event(&mut self, event: IrohEvent) -> Result<()> {
        match event {
            IrohEvent::Gossip { source, data } => {
                if let Ok(signed) = serde_json::from_slice::<SignedGossip>(&data) {
                    if self.consensus.verify(&signed) {
                        self.handle_signed_message(signed, source.to_string())
                            .await?;
                    } else {
                        eprintln!("签名验证失败，来自 {:?}", source);
                    }
                }
            }
            IrohEvent::PeerDiscovered { peer, addr } => {
                println!("[Iroh] 发现节点 {} @ {}", peer, addr);
                // 将发现的节点添加到订阅列表
                self.comms.add_peer(peer);
                
                // 尝试测量到该节点的网络距离
                // if let Ok(node_addr) = addr.parse::<NodeAddr>() {
                //     let network_distance = self.comms.measure_network_distance(&node_addr).await;
                //     self.topology.update_peer_network_distance(&peer.to_string(), network_distance);
                // }
            }
            IrohEvent::PeerExpired { peer } => {
                println!("[Iroh] 节点离线 {}", peer);
                self.comms.remove_peer(&peer);
            }
            IrohEvent::ConnectionEstablished { peer } => {
                println!("[Iroh] 连接建立: {}", peer);
                // 连接建立后，可以添加到订阅列表
                self.comms.add_peer(peer);
                
                // 当连接建立时，尝试获取节点地址并测量网络距离
                // 注意：这里我们没有直接的地址，需要通过其他方式获取
                // 我们将在接收到相似性探测时更新网络距离
            }
            IrohEvent::ConnectionClosed { peer } => {
                println!("[Iroh] 连接断开: {}", peer);
                self.comms.remove_peer(&peer);
            }
        }
        Ok(())
    }

    async fn publish_signed(&mut self, payload: GgbMessage) -> Result<()> {
        let signed = self.consensus.sign(payload)?;
        self.comms.publish(&signed)?;
        if !self.comms.broadcast_realtime(&signed).await {
            println!("[FAILOVER] QUIC 广播失败，已回落到纯 Gossip");
        }
        Ok(())
    }

    async fn handle_signed_message(&mut self, signed: SignedGossip, source: String) -> Result<()> {
        match &signed.payload {
            GgbMessage::Heartbeat { peer, .. } => {
                self.consensus.update_stake(peer, 0.0, 0.0, 0.05);
                // self.stats.record_heartbeat_received(peer);
                println!("收到 {} 的心跳 (via {source})", peer);
            }
            GgbMessage::SimilarityProbe {
                embedding,
                position,
                sender,
            } => {
                // self.stats.record_probe_received(sender);
                let self_embedding = vec![0.0; 128]; // 临时使用默认embedding
                use crate::types::NetworkDistance;
                let network_distance = NetworkDistance::new();

                self.topology.update_peer(
                    sender,
                    embedding.clone(),
                    position.clone(),
                    &self_embedding,
                    network_distance,
                );

                // 如果可能，尝试获取并更新网络距离信息
                // 这里我们可以尝试测量到发送方的网络距离
                // NodeId API已改变，暂时注释掉
                // if let Ok(node_id) = sender.parse::<iroh::NodeId>() {
                //     // 创建一个临时的 NodeAddr，因为我们只有 NodeId
                //     // 实际上我们需要通过其他方式获取完整的 NodeAddr
                //     // 为了演示目的，我们暂时跳过此步骤
                // }

                if let Some(snapshot) = self.topology.peer_snapshot(sender) {
                    let stake = self.consensus.stake_weight(sender);
                    let network_affinity = self.topology.get_peer_network_affinity(sender).unwrap_or(0.0);
                    println!(
                        "拓扑更新：{} => sim {:.3}, geo {:.3}, net {:.3}, stake {:.3}, dim {}, pos ({:.1},{:.1})",
                        sender,
                        snapshot.similarity,
                        snapshot.geo_affinity,
                        network_affinity,
                        stake,
                        snapshot.embedding_dim,
                        snapshot.position.lat,
                        snapshot.position.lon
                    );
                }
                if self.should_send_sparse_update(sender) {
                    if self.comms.allow_sparse_update() {
                        // let update = self.inference.make_sparse_update(16);
                        let update = crate::types::SparseUpdate {
                            indices: (0..16).collect(),
                            values: vec![0.0; 16],
                            version: 1,
                        };
                        let msg = GgbMessage::SparseUpdate {
                            update,
                            sender: self.comms.node_id().to_string(),
                        };
                        self.publish_signed(msg).await?;
                        // self.stats.record_sparse_update_sent(sender);
                    } else {
                        println!("[带宽限制] 本轮跳过稀疏更新");
                    }
                }
            }
            GgbMessage::SparseUpdate { update, .. } => {
                // self.stats.record_sparse_update_received(sender);
                self.training.apply_sparse_update(update);
            }
            GgbMessage::DenseSnapshot { snapshot, .. } => {
                // self.stats.record_dense_snapshot_received(sender);
                self.training.apply_dense_snapshot(snapshot);
            }
        }
        Ok(())
    }

    fn should_send_sparse_update(&self, target: &str) -> bool {
        let primary = self.topology.select_neighbors();
        if primary.iter().any(|peer| peer == target) {
            return true;
        }
        self.topology.mark_unreachable(target);
        false
    }

    async fn update_ai_decision_engine(&mut self) {
        // 收集当前节点性能数据
        let capabilities = self.device_manager.get();
        let _stats = self.stats.lock().unwrap().clone();

        // 创建节点性能记录
        let node_performance = NodePerformance {
            node_id: self.comms.node_id().to_string(),
            cpu_utilization: (self.topology.neighbor_sets().0.len() as f64) / 10.0, // 示例计算
            memory_utilization: (capabilities.max_memory_mb as f64) / 10000.0, // 假设最大内存是10GB
            gpu_utilization: if capabilities.has_gpu { Some(0.5) } else { None }, // 示例值
            network_latency: 10.0, // 示例值
            throughput: 100.0, // 示例值
            success_rate: 0.95, // 示例值
            last_updated: chrono::Utc::now(),
        };

        // 更新AI决策引擎中的节点性能数据
        self.ai_decision_engine.update_node_performance(node_performance).await;

        // 更新拓扑信息
        let topology_info = TopologyInfo {
            node_id: self.comms.node_id().to_string(),
            neighbors: self.topology.neighbor_sets().0.iter().cloned().collect(),
            network_distance: 0.0, // 使用默认值
            connection_quality: 0.9, // 示例值
        };

        self.ai_decision_engine.update_topology_info(topology_info).await;

        // 每隔一定时间生成网络健康报告
        if self.tick_counter % 200 == 0 {
            let health_report = self.ai_decision_engine.get_network_health_report().await;
            println!("[AI决策] 网络健康报告: 平均成功率 {:.2}, 节点数 {}", 
                health_report.average_success_rate, health_report.total_nodes);
            
            for recommendation in &health_report.recommendations {
                println!("[AI决策] 建议: {}", recommendation);
            }
        }
    }

    fn check_topology_health(&self) {
        let (primary, backups) = self.topology.neighbor_sets();
        if primary.len() < self.topology.max_neighbors() && !backups.is_empty() {
            println!(
                "[拓扑 Failover] 主邻居 {}/{}，启用备份 {:?}",
                primary.len(),
                self.topology.max_neighbors(),
                backups
            );
        } else if backups.len() < self.topology.failover_pool() {
            println!(
                "[拓扑提示] 备份邻居不足 {}/{}",
                backups.len(),
                self.topology.failover_pool()
            );
        }
    }

    async fn maybe_broadcast_dense(&mut self) -> Result<()> {
        let network_type = self.comms.network_type();
        if !network_type.allows_dense_snapshot() {
            // 移动网络下跳过密集快照
            return Ok(());
        }

        let snapshot = self.training.tensor_snapshot();
        let msg = GgbMessage::DenseSnapshot {
            sender: self.comms.node_id().to_string(),
            snapshot,
        };
        self.publish_signed(msg).await?;
        Ok(())
    }
}

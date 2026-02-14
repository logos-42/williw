//! 分布式推理计算模块
//!
//! 提供多节点协同推理的能力，包括：
//! - 分布式推理协调器
//! - 消息协议
//! - 中间结果缓存
//! - 分片管理
//! - 网络通信层

pub mod protocol;
pub mod cache;
pub mod coordinator;
pub mod network;

// 重新导出常用类型
pub use protocol::{InferenceMessage, ShardInfo, ExecutionMetrics};
pub use cache::{IntermediateCache, CachedResult};
pub use coordinator::{DistributedInferenceCoordinator, InferenceTaskState, InferenceStatus, CoordinatorConfig};
pub use network::{InferenceNetwork, IrohInferenceNetwork, MockInferenceNetwork, InferenceNetworkConfig};

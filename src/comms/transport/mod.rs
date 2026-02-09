/**
 * 传输层模块
 * 包含iroh集成、传输协议等底层传输功能
 */

pub mod iroh;
pub mod protocol;

// 重新导出常用类型
pub use iroh::{
    IrohConnectionManager, IrohConnectionConfig, ConnectionStats, WrappedMessage,
    QuicGateway, FILE_TRANSFER_MESSAGE_TYPE, GOSSIP_MESSAGE_TYPE
};
pub use protocol::{FileTransferProtocol, TransferProtocolConfig, FileIntegrity, ChecksumAlgorithm};

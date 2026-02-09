/**
 * P2P文件分发模块
 * 包含分发器、发送端、接收端等P2P传输功能
 */

pub mod distributor;
pub mod sender;
pub mod receiver;
pub mod events;

// 重新导出常用类型
pub use distributor::{P2PModelDistributor, TransferSession, TransferStatus, FileTransferMessage};
pub use sender::{P2PModelSender, P2PSenderArgs, run_sender};
pub use receiver::{P2PModelReceiver, P2PReceiverArgs, run_receiver};
pub use events::{TransferEvent, EventManager, get_global_event_manager, send_global_event, get_global_receiver};

// 为了向后兼容，重新导出p2p_distributor模块
pub mod p2p_distributor {
    pub use super::distributor::*;
}

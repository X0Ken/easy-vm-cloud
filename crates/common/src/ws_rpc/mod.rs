/// WebSocket RPC 模块
/// 
/// 提供基于 WebSocket 的双向 RPC 通信框架

pub mod message;
pub mod error;
pub mod types;
pub mod client;
pub mod server;

pub use message::{RpcMessage, MessageType};
pub use error::{RpcError, RpcErrorCode};
pub use types::*;


/// Easy VM Cloud - 公共库
/// 
/// 提供 Server 和 Agent 共享的类型、错误处理、工具函数等

pub mod errors;
pub mod models;
pub mod utils;
pub mod ws_rpc;

// 重新导出常用类型
pub use errors::{Error, Result};
pub use ws_rpc::{RpcMessage, RpcError, RpcErrorCode, MessageType};


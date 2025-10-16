/// WebSocket 客户端模块
/// 
/// Agent 通过 WebSocket 连接到 Server

pub mod client;
pub mod handler;

pub use client::WsClient;
pub use handler::RpcHandlerRegistry;


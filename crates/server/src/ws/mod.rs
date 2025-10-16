/// WebSocket 模块
/// 
/// 管理与 Agent 和前端客户端的 WebSocket 连接

pub mod agent_manager;
pub mod handler;
pub mod frontend_handler;

pub use agent_manager::AgentConnectionManager;
pub use handler::handle_agent_websocket;
pub use frontend_handler::{FrontendConnectionManager, handle_frontend_websocket, FrontendMessage};


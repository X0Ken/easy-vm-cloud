/// 应用全局状态

use sea_orm::DatabaseConnection;
use crate::ws::{AgentConnectionManager, FrontendConnectionManager};

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    /// SeaORM 数据库连接 - 用于所有数据库管理
    pub sea_db: DatabaseConnection,
    /// Agent WebSocket 连接管理器
    pub agent_manager: AgentConnectionManager,
    /// 前端 WebSocket 连接管理器
    pub frontend_manager: FrontendConnectionManager,
}

impl AppState {
    pub fn new(
        sea_db: DatabaseConnection,
        agent_manager: AgentConnectionManager,
    ) -> Self {
        Self {
            sea_db,
            agent_manager,
            frontend_manager: FrontendConnectionManager::new(),
        }
    }

    /// 获取 SeaORM 数据库连接（克隆）
    pub fn sea_db(&self) -> DatabaseConnection {
        self.sea_db.clone()
    }

    /// 获取 Agent 连接管理器
    pub fn agent_manager(&self) -> AgentConnectionManager {
        self.agent_manager.clone()
    }

    /// 获取前端连接管理器
    pub fn frontend_manager(&self) -> FrontendConnectionManager {
        self.frontend_manager.clone()
    }
}


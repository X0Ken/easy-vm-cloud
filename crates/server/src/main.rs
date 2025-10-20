/// Easy VM Cloud - Server
/// 
/// 后端服务器主程序，提供 REST API 服务

mod api;
mod app_state;
mod auth;
mod config;
mod db;
mod extractors;
mod middleware;
mod services;
mod utils;
mod ws;

use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use crate::{
    app_state::AppState,
    db::establish_connection,
    ws::AgentConnectionManager,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"))
        )
        .init();

    info!("🚀 启动 Easy VM Cloud Server...");

    // 加载环境变量
    dotenvy::dotenv().ok();
    
    // 加载配置
    let cfg = config::Config::from_env()?;
    info!("✅ 配置加载成功");

    // 建立数据库连接 (SeaORM) - 用于用户、角色、权限等管理
    let sea_db = establish_connection(&cfg.database_url)
        .await
        .expect("SeaORM 数据库连接失败");
    info!("✅ SeaORM 数据库连接成功");

    // 初始化 Agent 连接管理器
    let agent_manager = AgentConnectionManager::new();
    info!("✅ Agent 连接管理器初始化成功");

    // 创建应用状态
    let app_state = AppState::new(sea_db, agent_manager.clone());

    // 启动心跳监控（3分钟超时，每30秒检查一次）
    agent_manager.start_heartbeat_monitor_with_db_update(180, 30, app_state.clone());
    info!("✅ 心跳监控任务已启动（3分钟超时检测）");

    // 设置CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 构建应用路由
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/ws/agent", get(ws::handle_agent_websocket))
        .route("/ws/frontend", get(ws::handle_frontend_websocket))
        .nest("/api", api::api_routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state.clone());

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.server_port));
    info!("🎯 服务器监听在 http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root_handler() -> &'static str {
    "Easy VM Cloud Server API v1"
}

async fn health_handler() -> &'static str {
    "OK"
}

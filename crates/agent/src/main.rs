/// Easy VM Cloud - Agent
/// 
/// 节点代理程序，运行在宿主机上，负责执行虚拟化操作

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

mod config;
mod hypervisor;
mod metrics;
mod network;
mod node;
mod storage;
mod ws;

use ws::{WsClient, RpcHandlerRegistry};
use node::NodeManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    // 可以通过环境变量 RUST_LOG 设置日志级别，例如：
    // RUST_LOG=debug cargo run
    // RUST_LOG=agent=debug cargo run
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"))
        )
        .init();

    info!("🚀 启动 Easy VM Cloud Agent...");

    // 加载配置
    dotenvy::dotenv().ok();
    let cfg = config::Config::from_env()?;
    info!("✅ 配置加载成功");

    // 初始化组件
    info!("📊 初始化指标收集器...");
    let _metrics_collector = metrics::MetricsCollector::new();

    // 初始化管理器
    info!("🔧 初始化 hypervisor 管理器...");
    let hypervisor = Arc::new(hypervisor::HypervisorManager::new()?);
    
    info!("💾 初始化存储管理器...");
    let storage = Arc::new(storage::StorageManager::new());
    
    // 从环境变量获取网络 provider 接口
    let provider_interface = std::env::var("NETWORK_PROVIDER_INTERFACE")
        .unwrap_or_else(|_| "eth0".to_string());
    info!("🌐 初始化网络管理器 (provider: {})...", provider_interface);
    let network = Arc::new(network::NetworkManager::new(provider_interface));

    // 创建 RPC 处理器注册表
    let handler_registry = Arc::new(RwLock::new(RpcHandlerRegistry::new(
        hypervisor.clone(),
        storage.clone(),
        network.clone(),
    )));
    info!("✅ RPC 处理器已初始化");

    // 创建节点管理器
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());
    
    // 获取本机 IP 地址（简化处理，实际应该更智能）
    let ip_address = std::env::var("NODE_IP")
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let node_manager = NodeManager::new(
        cfg.node_id.clone(),
        hostname,
        ip_address,
    );

    // 创建 WebSocket 客户端
    let ws_client = WsClient::new(
        cfg.server_ws_url.clone(),
        node_manager,
        handler_registry,
    );

    info!("🎯 连接到 Server: {}", cfg.server_ws_url);
    info!("📌 节点 ID: {}", cfg.node_id);

    // 运行 WebSocket 客户端（会自动重连）
    ws_client.run().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}


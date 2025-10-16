/// 配置管理

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub node_id: String,
    pub node_name: String,
    pub server_ws_url: String,
    pub heartbeat_interval: u64,
    pub log_level: String,
    pub network_provider_interface: String,
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> anyhow::Result<Self> {
        let node_id = std::env::var("NODE_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

        let node_name = std::env::var("NODE_NAME")
            .unwrap_or_else(|_| hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()));

        let server_ws_url = std::env::var("SERVER_WS_URL")
            .unwrap_or_else(|_| "ws://localhost:3000/ws/agent".to_string());

        let heartbeat_interval = std::env::var("HEARTBEAT_INTERVAL")
            .unwrap_or_else(|_| "30".to_string())
            .parse()?;

        let log_level = std::env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "debug".to_string());

        let network_provider_interface = std::env::var("NETWORK_PROVIDER_INTERFACE")
            .unwrap_or_else(|_| "eth0".to_string());

        Ok(Self {
            node_id,
            node_name,
            server_ws_url,
            heartbeat_interval,
            log_level,
            network_provider_interface,
        })
    }
}


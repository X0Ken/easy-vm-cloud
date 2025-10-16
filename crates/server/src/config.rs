/// 配置管理

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server_port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub log_level: String,
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> anyhow::Result<Self> {
        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/vmcloud".to_string());

        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change-me-in-production".to_string());

        let log_level = std::env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "debug".to_string());

        Ok(Self {
            server_port,
            database_url,
            jwt_secret,
            log_level,
        })
    }
}


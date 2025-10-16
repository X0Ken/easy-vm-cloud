use thiserror::Error;

/// 统一错误类型
#[derive(Error, Debug)]
pub enum Error {
    #[error("数据库错误: {0}")]
    Database(String),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("认证错误: {0}")]
    Authentication(String),

    #[error("授权错误: {0}")]
    Authorization(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("资源已存在: {0}")]
    AlreadyExists(String),

    #[error("无效参数: {0}")]
    InvalidArgument(String),

    #[error("虚拟化错误: {0}")]
    Hypervisor(String),

    #[error("存储错误: {0}")]
    Storage(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("内部错误: {0}")]
    Internal(String),

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

/// 统一结果类型
pub type Result<T> = std::result::Result<T, Error>;

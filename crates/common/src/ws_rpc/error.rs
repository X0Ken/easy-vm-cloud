/// WebSocket RPC 错误定义

use serde::{Deserialize, Serialize};
use std::fmt;

/// RPC 错误码
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcErrorCode {
    // 通用错误
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    Timeout,
    ConnectionClosed,
    SerializationError,
    
    // 业务错误
    VmNotFound,
    VmAlreadyExists,
    VmOperationFailed,
    VmCreateFailed,
    VmStartFailed,
    VmStopFailed,
    VmDeleteFailed,
    
    StorageError,
    VolumeNotFound,
    VolumeCreateFailed,
    VolumeDeleteFailed,
    
    NetworkError,
    NetworkCreateFailed,
    NetworkDeleteFailed,
    
    NodeNotFound,
    NodeOffline,
}

impl RpcErrorCode {
    /// 转换为字符串码
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::MethodNotFound => "METHOD_NOT_FOUND",
            Self::InvalidParams => "INVALID_PARAMS",
            Self::InternalError => "INTERNAL_ERROR",
            Self::Timeout => "TIMEOUT",
            Self::ConnectionClosed => "CONNECTION_CLOSED",
            Self::SerializationError => "SERIALIZATION_ERROR",
            
            Self::VmNotFound => "VM_NOT_FOUND",
            Self::VmAlreadyExists => "VM_ALREADY_EXISTS",
            Self::VmOperationFailed => "VM_OPERATION_FAILED",
            Self::VmCreateFailed => "VM_CREATE_FAILED",
            Self::VmStartFailed => "VM_START_FAILED",
            Self::VmStopFailed => "VM_STOP_FAILED",
            Self::VmDeleteFailed => "VM_DELETE_FAILED",
            
            Self::StorageError => "STORAGE_ERROR",
            Self::VolumeNotFound => "VOLUME_NOT_FOUND",
            Self::VolumeCreateFailed => "VOLUME_CREATE_FAILED",
            Self::VolumeDeleteFailed => "VOLUME_DELETE_FAILED",
            
            Self::NetworkError => "NETWORK_ERROR",
            Self::NetworkCreateFailed => "NETWORK_CREATE_FAILED",
            Self::NetworkDeleteFailed => "NETWORK_DELETE_FAILED",
            
            Self::NodeNotFound => "NODE_NOT_FOUND",
            Self::NodeOffline => "NODE_OFFLINE",
        }
    }
}

impl fmt::Display for RpcErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// RPC 错误
#[derive(Debug, Clone)]
pub struct RpcError {
    pub code: RpcErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl RpcError {
    /// 创建新的 RPC 错误
    pub fn new(code: RpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// 创建带详情的 RPC 错误
    pub fn with_details(
        code: RpcErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    /// 无效请求错误
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::InvalidRequest, message)
    }

    /// 方法不存在错误
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(
            RpcErrorCode::MethodNotFound,
            format!("方法不存在: {}", method.into()),
        )
    }

    /// 参数错误
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::InvalidParams, message)
    }

    /// 内部错误
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::InternalError, message)
    }

    /// 超时错误
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(RpcErrorCode::Timeout, message)
    }

    /// 连接关闭错误
    pub fn connection_closed() -> Self {
        Self::new(RpcErrorCode::ConnectionClosed, "连接已关闭")
    }

    /// 序列化错误
    pub fn serialization_error(err: impl fmt::Display) -> Self {
        Self::new(
            RpcErrorCode::SerializationError,
            format!("序列化错误: {}", err),
        )
    }

    /// 虚拟机不存在
    pub fn vm_not_found(vm_id: impl Into<String>) -> Self {
        Self::new(
            RpcErrorCode::VmNotFound,
            format!("虚拟机不存在: {}", vm_id.into()),
        )
    }

    /// 节点不存在
    pub fn node_not_found(node_id: impl Into<String>) -> Self {
        Self::new(
            RpcErrorCode::NodeNotFound,
            format!("节点不存在: {}", node_id.into()),
        )
    }

    /// 节点离线
    pub fn node_offline(node_id: impl Into<String>) -> Self {
        Self::new(
            RpcErrorCode::NodeOffline,
            format!("节点离线: {}", node_id.into()),
        )
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

impl From<serde_json::Error> for RpcError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization_error(err)
    }
}

impl From<RpcError> for crate::Error {
    fn from(err: RpcError) -> Self {
        crate::Error::Internal(err.to_string())
    }
}


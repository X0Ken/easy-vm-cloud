/// WebSocket RPC 消息定义

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RPC 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// 请求消息（期望响应）
    Request,
    /// 响应消息
    Response,
    /// 通知消息（不需要响应）
    Notification,
    /// 流式数据消息
    Stream,
}

/// RPC 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMessage {
    /// 消息唯一ID
    pub id: String,
    
    /// 消息类型
    #[serde(rename = "type")]
    pub message_type: MessageType,
    
    /// RPC 方法名（request/notification 时必需）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    
    /// 消息负载
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    
    /// 错误信息（仅 response 时可能有值）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorInfo>,
}

/// RPC 错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcErrorInfo {
    /// 错误码
    pub code: String,
    
    /// 错误消息
    pub message: String,
    
    /// 错误详情（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl RpcMessage {
    /// 创建请求消息
    pub fn request(method: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: format!("req-{}", Uuid::new_v4()),
            message_type: MessageType::Request,
            method: Some(method.into()),
            payload: Some(payload),
            error: None,
        }
    }

    /// 创建响应消息
    pub fn response(id: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            message_type: MessageType::Response,
            method: None,
            payload: Some(payload),
            error: None,
        }
    }

    /// 创建错误响应消息
    pub fn error_response(
        id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            message_type: MessageType::Response,
            method: None,
            payload: None,
            error: Some(RpcErrorInfo {
                code: code.into(),
                message: message.into(),
                details,
            }),
        }
    }

    /// 创建通知消息
    pub fn notification(method: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: format!("notif-{}", Uuid::new_v4()),
            message_type: MessageType::Notification,
            method: Some(method.into()),
            payload: Some(payload),
            error: None,
        }
    }

    /// 创建流式消息
    pub fn stream(id: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            message_type: MessageType::Stream,
            method: None,
            payload: Some(payload),
            error: None,
        }
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 判断是否是成功响应
    pub fn is_success(&self) -> bool {
        self.message_type == MessageType::Response && self.error.is_none()
    }

    /// 判断是否是错误响应
    pub fn is_error(&self) -> bool {
        self.message_type == MessageType::Response && self.error.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_message() {
        let msg = RpcMessage::request("test_method", json!({"key": "value"}));
        assert_eq!(msg.message_type, MessageType::Request);
        assert_eq!(msg.method, Some("test_method".to_string()));
        assert!(msg.id.starts_with("req-"));
    }

    #[test]
    fn test_response_message() {
        let msg = RpcMessage::response("req-123", json!({"result": "ok"}));
        assert_eq!(msg.message_type, MessageType::Response);
        assert_eq!(msg.id, "req-123");
        assert!(msg.is_success());
    }

    #[test]
    fn test_error_response() {
        let msg = RpcMessage::error_response(
            "req-123",
            "TEST_ERROR",
            "Test error message",
            None,
        );
        assert!(msg.is_error());
        assert_eq!(msg.error.as_ref().unwrap().code, "TEST_ERROR");
    }

    #[test]
    fn test_serialization() {
        let msg = RpcMessage::request("test", json!({"x": 1}));
        let json = msg.to_json().unwrap();
        let parsed = RpcMessage::from_json(&json).unwrap();
        assert_eq!(msg.id, parsed.id);
        assert_eq!(msg.message_type, parsed.message_type);
    }
}


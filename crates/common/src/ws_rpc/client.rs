/// WebSocket RPC 客户端辅助工具

use super::{RpcMessage, RpcError, RpcErrorCode};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, warn};

/// RPC 响应等待器
type ResponseWaiter = oneshot::Sender<Result<RpcMessage, RpcError>>;

/// WebSocket RPC 客户端连接
pub struct WsRpcConnection {
    /// 待响应的请求映射（request_id -> response_sender）
    pending_requests: Arc<RwLock<HashMap<String, ResponseWaiter>>>,
    
    /// 发送消息的通道
    sender: mpsc::UnboundedSender<RpcMessage>,
}

impl WsRpcConnection {
    /// 创建新的 RPC 连接
    pub fn new() -> (Self, mpsc::UnboundedReceiver<RpcMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let connection = Self {
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            sender: tx,
        };
        
        (connection, rx)
    }

    /// 发送请求并等待响应
    pub async fn call(
        &self,
        method: impl Into<String>,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<RpcMessage, RpcError> {
        let msg = RpcMessage::request(method, payload);
        let request_id = msg.id.clone();
        
        // 创建响应接收器
        let (tx, rx) = oneshot::channel();
        
        // 注册等待响应
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }
        
        // 发送请求
        self.sender.send(msg).map_err(|_| {
            RpcError::new(RpcErrorCode::ConnectionClosed, "连接已关闭")
        })?;
        
        // 等待响应（带超时）
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(RpcError::new(
                RpcErrorCode::InternalError,
                "响应通道被关闭"
            )),
            Err(_) => {
                // 超时，清理等待器
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(RpcError::timeout(format!("请求超时: {}", request_id)))
            }
        }
    }

    /// 发送通知（不等待响应）
    pub async fn notify(
        &self,
        method: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<(), RpcError> {
        let msg = RpcMessage::notification(method, payload);
        self.sender.send(msg).map_err(|_| {
            RpcError::new(RpcErrorCode::ConnectionClosed, "连接已关闭")
        })?;
        Ok(())
    }

    /// 发送响应消息
    pub async fn send_response(&self, msg: RpcMessage) -> Result<(), RpcError> {
        self.sender.send(msg).map_err(|_| {
            RpcError::new(RpcErrorCode::ConnectionClosed, "连接已关闭")
        })?;
        Ok(())
    }

    /// 处理收到的消息
    pub async fn handle_message(&self, msg: RpcMessage) -> Result<(), RpcError> {
        match msg.message_type {
            super::MessageType::Response => {
                // 响应消息，唤醒对应的等待器
                let mut pending = self.pending_requests.write().await;
                if let Some(waiter) = pending.remove(&msg.id) {
                    let result = if msg.is_success() {
                        Ok(msg)
                    } else {
                        Err(RpcError::new(
                            RpcErrorCode::InternalError,
                            msg.error.as_ref()
                                .map(|e| e.message.clone())
                                .unwrap_or_else(|| "未知错误".to_string())
                        ))
                    };
                    let _ = waiter.send(result);
                } else {
                    warn!("收到未预期的响应消息: {}", msg.id);
                }
            }
            _ => {
                debug!("收到其他类型消息，由外部处理器处理");
            }
        }
        Ok(())
    }

    /// 获取待处理请求数量
    pub async fn pending_count(&self) -> usize {
        let pending = self.pending_requests.read().await;
        pending.len()
    }

    /// 清理所有待处理的请求
    pub async fn clear_pending(&self) {
        let mut pending = self.pending_requests.write().await;
        for (id, waiter) in pending.drain() {
            debug!("清理待处理请求: {}", id);
            let _ = waiter.send(Err(RpcError::connection_closed()));
        }
    }
}

impl Clone for WsRpcConnection {
    fn clone(&self) -> Self {
        Self {
            pending_requests: self.pending_requests.clone(),
            sender: self.sender.clone(),
        }
    }
}

/// 消息编解码辅助函数
pub mod codec {
    use super::*;
    
    /// 编码 RPC 消息为 WebSocket 消息
    pub fn encode(msg: &RpcMessage) -> Result<WsMessage, RpcError> {
        let json = msg.to_json()?;
        Ok(WsMessage::Text(json))
    }
    
    /// 解码 WebSocket 消息为 RPC 消息
    pub fn decode(ws_msg: WsMessage) -> Result<RpcMessage, RpcError> {
        match ws_msg {
            WsMessage::Text(text) => {
                RpcMessage::from_json(&text).map_err(|e| {
                    RpcError::serialization_error(e)
                })
            }
            WsMessage::Binary(data) => {
                let text = String::from_utf8(data).map_err(|e| {
                    RpcError::serialization_error(e)
                })?;
                RpcMessage::from_json(&text).map_err(|e| {
                    RpcError::serialization_error(e)
                })
            }
            WsMessage::Close(_) => {
                Err(RpcError::connection_closed())
            }
            _ => {
                Err(RpcError::invalid_request("不支持的 WebSocket 消息类型"))
            }
        }
    }
}


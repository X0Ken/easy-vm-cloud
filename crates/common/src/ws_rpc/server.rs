/// WebSocket RPC 服务端辅助工具

use super::{RpcMessage, RpcError, RpcErrorCode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// RPC 方法处理器类型
pub type RpcHandler = Arc<
    dyn Fn(serde_json::Value) -> Result<serde_json::Value, RpcError> + Send + Sync
>;

/// 异步 RPC 方法处理器类型
pub type AsyncRpcHandler = Arc<
    dyn Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, RpcError>> + Send>> + Send + Sync
>;

/// RPC 方法路由器
pub struct RpcRouter {
    /// 同步方法处理器
    handlers: Arc<RwLock<HashMap<String, RpcHandler>>>,
    
    /// 异步方法处理器
    async_handlers: Arc<RwLock<HashMap<String, AsyncRpcHandler>>>,
}

impl RpcRouter {
    /// 创建新的路由器
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            async_handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册同步方法处理器
    pub async fn register<F>(&self, method: impl Into<String>, handler: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, RpcError> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.into(), Arc::new(handler));
    }

    /// 注册异步方法处理器
    pub async fn register_async<F, Fut>(&self, method: impl Into<String>, handler: F)
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, RpcError>> + Send + 'static,
    {
        let mut handlers = self.async_handlers.write().await;
        let handler = Arc::new(move |payload: serde_json::Value| {
            Box::pin(handler(payload)) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, RpcError>> + Send>>
        });
        handlers.insert(method.into(), handler);
    }

    /// 处理 RPC 请求
    pub async fn handle_request(&self, msg: RpcMessage) -> RpcMessage {
        let method = match &msg.method {
            Some(m) => m,
            None => {
                return RpcMessage::error_response(
                    msg.id,
                    RpcErrorCode::InvalidRequest.as_str(),
                    "缺少方法名",
                    None,
                );
            }
        };

        let payload = msg.payload.clone().unwrap_or(serde_json::Value::Null);
        
        // 📨 打印收到的请求
        info!("📨 [收到RPC请求] method={}, id={}", method, msg.id);
        debug!("📨 请求内容: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

        // 先尝试异步处理器
        {
            let async_handlers = self.async_handlers.read().await;
            if let Some(handler) = async_handlers.get(method) {
                match handler(payload).await {
                    Ok(result) => {
                        info!("✅ [RPC处理成功] method={}, id={}", method, msg.id);
                        debug!("✅ 响应内容: {}", serde_json::to_string_pretty(&result).unwrap_or_default());
                        return RpcMessage::response(msg.id, result);
                    }
                    Err(err) => {
                        warn!("❌ [RPC处理失败] method={}, id={}, code={}, error={}", 
                              method, msg.id, err.code.as_str(), err.message);
                        return RpcMessage::error_response(
                            msg.id,
                            err.code.as_str(),
                            err.message,
                            err.details,
                        );
                    }
                }
            }
        }

        // 再尝试同步处理器
        {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(method) {
                match handler(payload) {
                    Ok(result) => {
                        return RpcMessage::response(msg.id, result);
                    }
                    Err(err) => {
                        return RpcMessage::error_response(
                            msg.id,
                            err.code.as_str(),
                            err.message,
                            err.details,
                        );
                    }
                }
            }
        }

        // 方法不存在
        RpcMessage::error_response(
            msg.id,
            RpcErrorCode::MethodNotFound.as_str(),
            format!("方法不存在: {}", method),
            None,
        )
    }

    /// 获取已注册的方法列表
    pub async fn list_methods(&self) -> Vec<String> {
        let mut methods = Vec::new();
        
        let handlers = self.handlers.read().await;
        methods.extend(handlers.keys().cloned());
        
        let async_handlers = self.async_handlers.read().await;
        methods.extend(async_handlers.keys().cloned());
        
        methods.sort();
        methods
    }
}

impl Default for RpcRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RpcRouter {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
            async_handlers: self.async_handlers.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_sync_handler() {
        let router = RpcRouter::new();
        
        router.register("test", |payload| {
            Ok(json!({"echo": payload}))
        }).await;

        let req = RpcMessage::request("test", json!({"hello": "world"}));
        let resp = router.handle_request(req).await;
        
        assert!(resp.is_success());
        assert_eq!(resp.payload.unwrap()["echo"]["hello"], "world");
    }

    #[tokio::test]
    async fn test_async_handler() {
        let router = RpcRouter::new();
        
        router.register_async("async_test", |payload| async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok(json!({"processed": payload}))
        }).await;

        let req = RpcMessage::request("async_test", json!({"data": 123}));
        let resp = router.handle_request(req).await;
        
        assert!(resp.is_success());
    }

    #[tokio::test]
    async fn test_method_not_found() {
        let router = RpcRouter::new();
        let req = RpcMessage::request("unknown", json!({}));
        let resp = router.handle_request(req).await;
        
        assert!(resp.is_error());
        assert_eq!(resp.error.unwrap().code, RpcErrorCode::MethodNotFound.as_str());
    }
}


/// Agent 连接管理器
/// 
/// 负责管理所有 Agent 的 WebSocket 连接

use common::ws_rpc::{RpcMessage, RpcError, RpcErrorCode};
use futures_util::stream::SplitSink;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock, oneshot};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{debug, info, warn, error};

/// 等待响应的请求信息
type PendingRequest = oneshot::Sender<Result<RpcMessage, RpcError>>;

/// Agent 连接信息
pub struct AgentConnection {
    /// 节点 ID
    pub node_id: String,
    
    /// 节点主机名
    pub hostname: String,
    
    /// 节点 IP 地址
    pub ip_address: String,
    
    /// 发送消息的通道
    pub sender: mpsc::UnboundedSender<RpcMessage>,
    
    /// 最后心跳时间
    pub last_heartbeat: Arc<RwLock<std::time::Instant>>,
    
    /// 等待响应的请求 Map: request_id -> response_sender
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
}

impl AgentConnection {
    /// 发送 RPC 请求并等待响应
    pub async fn call(
        &self,
        method: impl Into<String>,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<RpcMessage, RpcError> {
        let method_str = method.into();
        let msg = RpcMessage::request(&method_str, payload.clone());
        let request_id = msg.id.clone();
        
        // 📤 打印发送的请求
        info!("📤 [Server -> Agent] 发送请求: node={}, method={}, id={}", 
              self.node_id, method_str, request_id);
        debug!("📤 请求内容: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());
        
        // 创建响应接收器
        let (tx, rx) = oneshot::channel();
        
        // 注册等待响应的请求
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }
        
        // 发送请求
        if let Err(_) = self.sender.send(msg) {
            // 发送失败，移除待处理请求
            let mut pending = self.pending_requests.write().await;
            pending.remove(&request_id);
            return Err(RpcError::new(RpcErrorCode::ConnectionClosed, "连接已关闭"));
        }
        
        // 等待响应（带超时）
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                // 响应通道被关闭（这不应该发生）
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(RpcError::new(RpcErrorCode::InternalError, "响应通道被关闭"))
            }
            Err(_) => {
                // 超时，移除待处理请求
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(RpcError::timeout(format!("请求超时: {}", request_id)))
            }
        }
    }
    
    /// 处理收到的响应消息（由 WebSocket handler 调用）
    pub async fn handle_response(&self, response: RpcMessage) {
        let request_id = response.id.clone();
        
        // 📥 打印收到的响应
        if let Some(ref error_info) = response.error {
            warn!("📥 [Agent -> Server] 收到错误响应: node={}, id={}, code={}, message={}", 
                  self.node_id, request_id, error_info.code, error_info.message);
        } else {
            info!("📥 [Agent -> Server] 收到成功响应: node={}, id={}", 
                  self.node_id, request_id);
            if let Some(ref payload) = response.payload {
                debug!("📥 响应内容: {}", serde_json::to_string_pretty(payload).unwrap_or_default());
            }
        }
        
        // 查找并移除待处理的请求
        let sender = {
            let mut pending = self.pending_requests.write().await;
            pending.remove(&request_id)
        };
        
        if let Some(sender) = sender {
            // 检查响应是否包含错误
            let result = if let Some(error_info) = response.error {
                // 将错误代码字符串转换回 RpcErrorCode
                let error_code = match error_info.code.as_str() {
                    code if code.starts_with("VM_") => RpcErrorCode::VmOperationFailed,
                    code if code.starts_with("VOLUME_") => RpcErrorCode::StorageError,
                    code if code.starts_with("NETWORK_") => RpcErrorCode::NetworkError,
                    _ => RpcErrorCode::InternalError,
                };
                Err(RpcError::new(error_code, error_info.message))
            } else {
                Ok(response)
            };
            
            // 发送响应到等待的请求
            if let Err(_) = sender.send(result) {
                warn!("无法发送响应，等待者已关闭: {}", request_id);
            }
        } else {
            debug!("收到未预期的响应: {}", request_id);
        }
    }

    /// 发送通知
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

    /// 更新最后心跳时间
    pub async fn update_heartbeat(&self) {
        let mut last_heartbeat = self.last_heartbeat.write().await;
        *last_heartbeat = std::time::Instant::now();
    }

    /// 获取距离上次心跳的时间（秒）
    pub async fn heartbeat_elapsed(&self) -> u64 {
        let last_heartbeat = self.last_heartbeat.read().await;
        last_heartbeat.elapsed().as_secs()
    }
}

/// Agent 连接管理器
#[derive(Clone)]
pub struct AgentConnectionManager {
    /// 所有连接的映射：node_id -> AgentConnection
    connections: Arc<RwLock<HashMap<String, Arc<AgentConnection>>>>,
}

impl AgentConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册新的 Agent 连接
    pub async fn register(
        &self,
        node_id: String,
        hostname: String,
        ip_address: String,
        sender: mpsc::UnboundedSender<RpcMessage>,
    ) -> Arc<AgentConnection> {
        let connection = Arc::new(AgentConnection {
            node_id: node_id.clone(),
            hostname,
            ip_address,
            sender,
            last_heartbeat: Arc::new(RwLock::new(std::time::Instant::now())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        });

        let mut connections = self.connections.write().await;
        connections.insert(node_id.clone(), connection.clone());
        
        info!("Agent 已注册: {}", node_id);
        connection
    }

    /// 注销 Agent 连接
    pub async fn unregister(&self, node_id: &str) {
        let mut connections = self.connections.write().await;
        if connections.remove(node_id).is_some() {
            info!("Agent 已注销: {}", node_id);
        }
    }

    /// 获取指定节点的连接
    pub async fn get(&self, node_id: &str) -> Option<Arc<AgentConnection>> {
        let connections = self.connections.read().await;
        connections.get(node_id).cloned()
    }

    /// 获取所有在线的节点 ID 列表
    pub async fn list_nodes(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// 获取在线节点数量
    pub async fn count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 检查节点是否在线
    pub async fn is_online(&self, node_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(node_id)
    }

    /// 清理超时的连接
    /// 返回被清理的节点 ID 列表
    pub async fn cleanup_timeout_connections(&self, timeout_secs: u64) -> Vec<String> {
        let mut to_remove = Vec::new();
        
        {
            let connections = self.connections.read().await;
            for (node_id, conn) in connections.iter() {
                if conn.heartbeat_elapsed().await > timeout_secs {
                    warn!("节点心跳超时: {} ({}秒)", node_id, conn.heartbeat_elapsed().await);
                    to_remove.push(node_id.clone());
                }
            }
        }

        // 移除超时的连接
        if !to_remove.is_empty() {
            let mut connections = self.connections.write().await;
            for node_id in &to_remove {
                connections.remove(node_id);
                info!("已清理超时节点: {}", node_id);
            }
        }

        to_remove
    }

    /// 向指定节点发送 RPC 请求
    pub async fn call(
        &self,
        node_id: &str,
        method: impl Into<String>,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<RpcMessage, RpcError> {
        let connection = self.get(node_id).await
            .ok_or_else(|| RpcError::node_not_found(node_id))?;
        
        connection.call(method, payload, timeout).await
    }

    /// 向指定节点发送通知
    pub async fn notify(
        &self,
        node_id: &str,
        method: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<(), RpcError> {
        let method_str = method.into();
        info!("📤 [Server -> Agent] 发送通知: node={}, method={}, payload={}", node_id, method_str, payload);
        let connection = self.get(node_id).await
            .ok_or_else(|| RpcError::node_not_found(node_id))?;
        
        connection.notify(method_str, payload).await
    }

    /// 向所有节点广播通知
    pub async fn broadcast(
        &self,
        method: impl Into<String> + Clone,
        payload: serde_json::Value,
    ) -> usize {
        let connections = self.connections.read().await;
        let mut count = 0;

        for (node_id, conn) in connections.iter() {
            if let Err(e) = conn.notify(method.clone(), payload.clone()).await {
                warn!("向节点 {} 发送广播失败: {}", node_id, e);
            } else {
                count += 1;
            }
        }

        debug!("广播消息已发送到 {} 个节点", count);
        count
    }

    /// 启动心跳超时检查任务
    pub fn start_heartbeat_monitor(self, timeout_secs: u64, check_interval_secs: u64) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(check_interval_secs));
            
            loop {
                interval.tick().await;
                
                let removed = self.cleanup_timeout_connections(timeout_secs).await;
                if !removed.is_empty() {
                    warn!("心跳监控: 清理了 {} 个超时节点", removed.len());
                }
            }
        });
    }

    /// 启动心跳超时检查任务（带数据库状态更新）
    pub fn start_heartbeat_monitor_with_db_update(
        self, 
        timeout_secs: u64, 
        check_interval_secs: u64,
        app_state: crate::app_state::AppState,
    ) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(check_interval_secs));
            
            loop {
                interval.tick().await;
                
                // 清理超时的连接
                let removed = self.cleanup_timeout_connections(timeout_secs).await;
                if !removed.is_empty() {
                    warn!("心跳监控: 清理了 {} 个超时节点", removed.len());
                }

                // 检查并更新数据库中的超时节点状态
                let node_service = crate::services::node_service::NodeService::new(app_state.clone());
                match node_service.check_and_update_timeout_nodes(timeout_secs).await {
                    Ok(updated_nodes) => {
                        if !updated_nodes.is_empty() {
                            info!("心跳监控: 已更新 {} 个超时节点状态为离线", updated_nodes.len());
                        }
                    }
                    Err(e) => {
                        error!("心跳监控: 检查超时节点失败: {}", e);
                    }
                }
            }
        });
    }
}

impl Default for AgentConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}


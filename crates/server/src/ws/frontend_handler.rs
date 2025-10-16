/// 前端 WebSocket 连接处理器
/// 
/// 处理与前端客户端的 WebSocket 连接和消息

use axum::extract::ws::{Message as AxumWsMessage, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// 前端连接信息
#[derive(Debug, Clone)]
pub struct FrontendConnection {
    /// 连接 ID
    pub connection_id: String,
    
    /// 用户 ID（如果有认证）
    pub user_id: Option<String>,
    
    /// 发送消息的通道
    pub sender: mpsc::UnboundedSender<FrontendMessage>,
    
    /// 连接时间
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// 前端消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FrontendMessage {
    /// VM 状态更新
    VmStatusUpdate {
        vm_id: String,
        status: String,
        message: Option<String>,
    },
    /// 节点状态更新
    NodeStatusUpdate {
        node_id: String,
        status: String,
        message: Option<String>,
    },
    /// 任务状态更新
    TaskStatusUpdate {
        task_id: String,
        status: String,
        progress: Option<i32>,
        message: Option<String>,
    },
    /// 系统通知
    SystemNotification {
        title: String,
        message: String,
        level: String, // info, warning, error
    },
    /// 心跳响应
    Pong {
        timestamp: i64,
    },
}

/// 前端连接管理器
#[derive(Clone)]
pub struct FrontendConnectionManager {
    /// 所有连接的映射：connection_id -> FrontendConnection
    connections: Arc<RwLock<HashMap<String, Arc<FrontendConnection>>>>,
}

impl FrontendConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册新的前端连接
    pub async fn register(
        &self,
        connection_id: String,
        user_id: Option<String>,
        sender: mpsc::UnboundedSender<FrontendMessage>,
    ) -> Arc<FrontendConnection> {
        let connection = Arc::new(FrontendConnection {
            connection_id: connection_id.clone(),
            user_id,
            sender,
            connected_at: chrono::Utc::now(),
        });

        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), connection.clone());
        
        info!("前端连接已注册: {}", connection_id);
        connection
    }

    /// 注销前端连接
    pub async fn unregister(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if connections.remove(connection_id).is_some() {
            info!("前端连接已注销: {}", connection_id);
        }
    }

    /// 获取所有连接
    pub async fn get_all_connections(&self) -> Vec<Arc<FrontendConnection>> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// 获取连接数量
    pub async fn count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 向所有连接广播消息
    pub async fn broadcast(&self, message: FrontendMessage) -> usize {
        let connections = self.connections.read().await;
        let mut count = 0;

        for (connection_id, conn) in connections.iter() {
            if let Err(e) = conn.sender.send(message.clone()) {
                warn!("向前端连接 {} 发送消息失败: {}", connection_id, e);
            } else {
                count += 1;
            }
        }

        debug!("广播消息已发送到 {} 个前端连接", count);
        count
    }

    /// 向指定用户的所有连接发送消息
    pub async fn send_to_user(&self, user_id: &str, message: FrontendMessage) -> usize {
        let connections = self.connections.read().await;
        let mut count = 0;

        for (connection_id, conn) in connections.iter() {
            if let Some(ref conn_user_id) = conn.user_id {
                if conn_user_id == user_id {
                    if let Err(e) = conn.sender.send(message.clone()) {
                        warn!("向用户 {} 的连接 {} 发送消息失败: {}", user_id, connection_id, e);
                    } else {
                        count += 1;
                    }
                }
            }
        }

        debug!("向用户 {} 发送消息到 {} 个连接", user_id, count);
        count
    }
}

impl Default for FrontendConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket 升级处理器
pub async fn handle_frontend_websocket(
    ws: WebSocketUpgrade,
    State(state): State<crate::app_state::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_frontend_connection(socket, state))
}

/// 处理前端 WebSocket 连接
async fn handle_frontend_connection(socket: WebSocket, state: crate::app_state::AppState) {
    let connection_id = Uuid::new_v4().to_string();
    info!("新的前端 WebSocket 连接: {}", connection_id);

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建消息发送通道
    let (tx, mut rx) = mpsc::unbounded_channel::<FrontendMessage>();

    // 注册到管理器
    let connection = state.frontend_manager()
        .register(connection_id.clone(), None, tx.clone())
        .await;

    // 创建消息发送任务
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = send_frontend_message(&mut ws_sender, msg).await {
                error!("发送前端消息失败: {}", e);
                break;
            }
        }
        debug!("前端消息发送任务结束");
    });

    // 创建消息接收任务
    let connection_clone = connection.clone();
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => {
                    if let Err(e) = handle_frontend_incoming_message(msg, &connection_clone, &state_clone).await {
                        warn!("处理前端消息失败: {}", e);
                    }
                }
                Err(e) => {
                    error!("接收前端消息错误: {}", e);
                    break;
                }
            }
        }
        debug!("前端消息接收任务结束");
    });

    // 等待任一任务完成
    tokio::select! {
        _ = &mut send_task => {
            debug!("前端发送任务已结束");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug!("前端接收任务已结束");
            send_task.abort();
        }
    }

    // 清理：从管理器中注销
    state.frontend_manager().unregister(&connection_id).await;
    info!("前端连接已关闭: {}", connection_id);
}

/// 处理收到的前端消息
async fn handle_frontend_incoming_message(
    ws_msg: AxumWsMessage,
    connection: &FrontendConnection,
    state: &crate::app_state::AppState,
) -> Result<(), String> {
    match ws_msg {
        AxumWsMessage::Text(text) => {
            debug!("收到前端文本消息: {}", text);
            
            // 解析 JSON 消息
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(msg_type) = msg.get("type").and_then(|v| v.as_str()) {
                    match msg_type {
                        "ping" => {
                            // 处理心跳
                            let timestamp = chrono::Utc::now().timestamp();
                            let pong = FrontendMessage::Pong { timestamp };
                            
                            if let Err(e) = connection.sender.send(pong) {
                                warn!("发送心跳响应失败: {}", e);
                            }
                        }
                        _ => {
                            debug!("收到未知的前端消息类型: {}", msg_type);
                        }
                    }
                }
            }
        }
        AxumWsMessage::Binary(data) => {
            debug!("收到前端二进制消息: {} bytes", data.len());
        }
        AxumWsMessage::Close(_) => {
            debug!("前端连接关闭");
        }
        _ => {
            debug!("收到其他类型的前端消息");
        }
    }
    
    Ok(())
}

/// 发送前端消息
async fn send_frontend_message(
    sender: &mut futures_util::stream::SplitSink<WebSocket, AxumWsMessage>,
    msg: FrontendMessage,
) -> Result<(), String> {
    let json = serde_json::to_string(&msg)
        .map_err(|e| format!("序列化前端消息失败: {}", e))?;
    
    sender.send(AxumWsMessage::Text(json))
        .await
        .map_err(|e| format!("发送前端 WebSocket 消息失败: {}", e))?;
    
    Ok(())
}

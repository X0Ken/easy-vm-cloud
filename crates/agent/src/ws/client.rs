/// WebSocket 客户端
/// 
/// Agent 连接到 Server 的 WebSocket 客户端

use common::ws_rpc::{RpcMessage, RegisterRequest, NodeResourceInfo};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use super::handler::RpcHandlerRegistry;

/// WebSocket 客户端状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
    Registered,
}

/// WebSocket 客户端
#[derive(Clone)]
pub struct WsClient {
    /// Server 地址
    server_url: String,
    
    /// 节点配置
    node_id: String,
    hostname: String,
    ip_address: String,
    
    /// 客户端状态
    state: Arc<RwLock<ClientState>>,
    
    /// RPC 处理器注册表
    handler_registry: Arc<RwLock<RpcHandlerRegistry>>,
    
    /// 重连间隔（秒）
    reconnect_interval: u64,
    
    /// 心跳间隔（秒）
    heartbeat_interval: u64,
    
    /// 消息发送通道（用于主动RPC调用）
    message_sender: Arc<RwLock<Option<mpsc::UnboundedSender<RpcMessage>>>>,
    
    /// 待响应的RPC请求（用于主动RPC调用）
    pending_requests: Arc<RwLock<std::collections::HashMap<String, mpsc::UnboundedSender<RpcMessage>>>>,
}

impl WsClient {
    /// 创建新的 WebSocket 客户端
    pub fn new(
        server_url: impl Into<String>,
        node_id: impl Into<String>,
        hostname: impl Into<String>,
        ip_address: impl Into<String>,
        handler_registry: Arc<RwLock<RpcHandlerRegistry>>,
    ) -> Self {
        Self {
            server_url: server_url.into(),
            node_id: node_id.into(),
            hostname: hostname.into(),
            ip_address: ip_address.into(),
            state: Arc::new(RwLock::new(ClientState::Disconnected)),
            handler_registry,
            reconnect_interval: 5,
            heartbeat_interval: 30,
            message_sender: Arc::new(RwLock::new(None)),
            pending_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 启动客户端（连接并保持）
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            info!("尝试连接到 Server: {}", self.server_url);
            
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("连接正常关闭");
                }
                Err(e) => {
                    error!("连接错误: {}", e);
                }
            }
            
            // 更新状态为断开
            {
                let mut state = self.state.write().await;
                *state = ClientState::Disconnected;
            }
            
            // 等待后重连
            warn!("{}秒后重新连接...", self.reconnect_interval);
            tokio::time::sleep(Duration::from_secs(self.reconnect_interval)).await;
        }
    }

    /// 连接并运行
    async fn connect_and_run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 更新状态
        {
            let mut state = self.state.write().await;
            *state = ClientState::Connecting;
        }

        // 连接到 Server
        let (ws_stream, _) = connect_async(&self.server_url).await?;
        info!("✅ WebSocket 连接成功");

        // 更新状态
        {
            let mut state = self.state.write().await;
            *state = ClientState::Connected;
        }

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // 创建消息发送通道
        let (tx, mut rx) = mpsc::unbounded_channel::<RpcMessage>();

        // 发送注册请求
        let register_req = RegisterRequest {
            node_id: self.node_id.clone(),
            hostname: self.hostname.clone(),
            ip_address: self.ip_address.clone(),
        };
        
        let register_msg = RpcMessage::request(
            "register",
            serde_json::to_value(&register_req)?,
        );
        
        self.send_message(&mut ws_sender, register_msg).await?;
        debug!("已发送注册请求");

        // 等待注册响应
        if let Some(msg) = ws_receiver.next().await {
            let rpc_msg = self.parse_message(msg?)?;
            if rpc_msg.is_success() {
                info!("✅ 注册成功");
                let mut state = self.state.write().await;
                *state = ClientState::Registered;
                
                // 注册成功后，立即发送节点资源信息
                if let Err(e) = self.send_node_resource_info(&tx).await {
                    warn!("发送节点资源信息失败: {}", e);
                }
            } else {
                return Err("注册失败".into());
            }
        }

        // 启动心跳任务
        let tx_heartbeat = tx.clone();
        let heartbeat_interval = self.heartbeat_interval;
        let heartbeat_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(heartbeat_interval));
            loop {
                interval.tick().await;
                
                let heartbeat_msg = RpcMessage::notification(
                    "heartbeat",
                    serde_json::json!({
                        "timestamp": chrono::Utc::now().timestamp()
                    }),
                );
                
                if tx_heartbeat.send(heartbeat_msg).is_err() {
                    break;
                }
                debug!("发送心跳");
            }
        });

        // 启动发送任务
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = match msg.to_json() {
                    Ok(j) => j,
                    Err(e) => {
                        error!("序列化消息失败: {}", e);
                        continue;
                    }
                };
                
                if let Err(e) = ws_sender.send(Message::Text(json)).await {
                    error!("发送消息失败: {}", e);
                    break;
                }
            }
            debug!("发送任务结束");
        });

        // 设置通知发送器和WebSocket客户端引用到处理器注册表
        {
            let mut registry = self.handler_registry.write().await;
            registry.set_notification_sender(tx.clone());
            registry.set_ws_client(Arc::new(self.clone()));
        }

        // 设置消息发送通道（用于主动RPC调用）
        {
            let mut sender = self.message_sender.write().await;
            *sender = Some(tx.clone());
        }

        // 启动接收任务
        let handler_registry = self.handler_registry.clone();
        let tx_clone = tx.clone();
        let pending_requests = self.pending_requests.clone();
        let recv_task = tokio::spawn(async move {
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(msg) => {
                        // 为每个消息创建独立的异步任务进行并发处理
                        let handler_registry = handler_registry.clone();
                        let tx_clone = tx_clone.clone();
                        let pending_requests = pending_requests.clone();
                        tokio::spawn(async move {
                            Self::handle_message_static(
                                msg,
                                &handler_registry,
                                &tx_clone,
                                &pending_requests,
                            ).await;
                        });
                    }
                    Err(e) => {
                        error!("接收消息错误: {}", e);
                        break;
                    }
                }
            }
            debug!("接收任务结束");
        });

        // 等待任一任务完成
        tokio::select! {
            _ = send_task => {
                debug!("发送任务已结束");
            }
            _ = recv_task => {
                debug!("接收任务已结束");
            }
        }

        // 清理心跳任务
        heartbeat_task.abort();

        Ok(())
    }

    /// 发送消息（辅助方法）
    async fn send_message(
        &self,
        sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
            >,
            Message
        >,
        msg: RpcMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = msg.to_json()?;
        sender.send(Message::Text(json)).await?;
        Ok(())
    }

    /// 解析消息（辅助方法）
    fn parse_message(&self, msg: Message) -> Result<RpcMessage, Box<dyn std::error::Error + Send + Sync>> {
        match msg {
            Message::Text(text) => {
                Ok(RpcMessage::from_json(&text)?)
            }
            Message::Binary(data) => {
                let text = String::from_utf8(data)?;
                Ok(RpcMessage::from_json(&text)?)
            }
            _ => Err("不支持的消息类型".into()),
        }
    }

    /// 处理收到的消息（静态方法）
    /// 无返回值，便于异步并发调用
    async fn handle_message_static(
        msg: Message,
        handler_registry: &Arc<RwLock<RpcHandlerRegistry>>,
        tx: &mpsc::UnboundedSender<RpcMessage>,
        pending_requests: &Arc<RwLock<std::collections::HashMap<String, mpsc::UnboundedSender<RpcMessage>>>>,
    ) {
        let rpc_msg = match msg {
            Message::Text(text) => {
                match RpcMessage::from_json(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("解析文本消息失败: {}", e);
                        return;
                    }
                }
            }
            Message::Binary(data) => {
                let text = match String::from_utf8(data) {
                    Ok(text) => text,
                    Err(e) => {
                        error!("二进制转字符串失败: {}", e);
                        return;
                    }
                };
                match RpcMessage::from_json(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("解析二进制消息失败: {}", e);
                        return;
                    }
                }
            }
            Message::Close(_) => {
                warn!("收到连接关闭消息");
                return;
            }
            _ => {
                debug!("收到其他类型消息，忽略");
                return;
            }
        };

        debug!("收到消息: type={:?}, method={:?}", 
               rpc_msg.message_type, rpc_msg.method);

        match rpc_msg.message_type {
            common::MessageType::Request => {
                // 处理请求并发送响应
                let registry = handler_registry.read().await;
                let response = registry.handle_request(rpc_msg).await;
                if let Err(e) = tx.send(response) {
                    error!("发送响应失败: {}", e);
                }
            }
            common::MessageType::Response => {
                // 处理来自Server的响应（用于主动RPC调用）
                debug!("收到Server响应: id={}", rpc_msg.id);
                
                // 查找对应的待响应请求
                let mut pending = pending_requests.write().await;
                if let Some(response_tx) = pending.remove(&rpc_msg.id) {
                    if let Err(e) = response_tx.send(rpc_msg) {
                        error!("发送响应到等待通道失败: {}", e);
                    }
                } else {
                    debug!("未找到对应的待响应请求: {}", rpc_msg.id);
                }
            }
            common::MessageType::Notification => {
                // 处理通知（不需要响应）
                debug!("收到通知: {:?}", rpc_msg.method);
                
                // 处理异步操作通知
                if let Some(method) = &rpc_msg.method {
                    match method.as_str() {
                        "stop_vm_async" => {
                            // 处理异步停止虚拟机通知
                            Self::handle_async_notification(rpc_msg, handler_registry, tx).await;
                        }
                        _ => {
                            debug!("收到其他通知: {}", method);
                        }
                    }
                }
            }
            _ => {
                debug!("收到其他类型消息");
            }
        }
    }

    /// 获取当前状态
    pub async fn state(&self) -> ClientState {
        let state = self.state.read().await;
        state.clone()
    }

    /// 是否已注册
    pub async fn is_registered(&self) -> bool {
        let state = self.state.read().await;
        *state == ClientState::Registered
    }

    /// 获取通知发送器
    pub fn get_notification_sender(&self) -> Option<mpsc::UnboundedSender<RpcMessage>> {
        // 这个方法需要在连接建立后调用
        // 暂时返回 None，因为需要访问连接状态
        None
    }

    /// 主动调用 Server 的 RPC 方法
    /// 
    /// 这个方法允许 Agent 主动向 Server 发起 RPC 调用
    /// 例如：获取存储池信息、查询任务状态等
    pub async fn call_server_rpc(
        &self,
        method: &str,
        payload: serde_json::Value,
        timeout: Option<Duration>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // 检查连接状态
        let state = self.state.read().await;
        if *state != ClientState::Registered {
            return Err("Agent 未注册或连接未建立".into());
        }
        drop(state);

        // 创建请求消息
        let request = RpcMessage::request(method, payload);
        let request_id = request.id.clone();

        // 创建响应通道
        let (tx, mut rx) = mpsc::unbounded_channel::<RpcMessage>();

        // 注册待响应请求
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx.clone());
        }

        // 获取消息发送通道
        let sender = {
            let sender_guard = self.message_sender.read().await;
            sender_guard.clone()
        };

        let sender = match sender {
            Some(s) => s,
            None => return Err("消息发送通道未初始化".into()),
        };

        // 发送请求
        if let Err(e) = sender.send(request) {
            // 清理待响应请求
            let mut pending = self.pending_requests.write().await;
            pending.remove(&request_id);
            return Err(format!("发送请求失败: {}", e).into());
        }

        // 等待响应
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(30));
        let response = tokio::time::timeout(timeout_duration, rx.recv()).await;

        // 清理待响应请求
        {
            let mut pending = self.pending_requests.write().await;
            pending.remove(&request_id);
        }

        match response {
            Ok(Some(rpc_response)) => {
                if rpc_response.is_success() {
                    Ok(rpc_response.payload.unwrap_or(serde_json::Value::Null))
                } else {
                    let error_msg = rpc_response.error
                        .map(|e| format!("RPC错误: {}", e.message))
                        .unwrap_or_else(|| "未知RPC错误".to_string());
                    Err(error_msg.into())
                }
            }
            Ok(None) => Err("响应通道已关闭".into()),
            Err(_) => Err("RPC调用超时".into()),
        }
    }

    /// 获取存储池信息
    /// 
    /// 这是一个便捷方法，用于从Server获取存储池信息
    pub async fn get_storage_pool_info(
        &self,
        pool_id: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "pool_id": pool_id
        });
        
        self.call_server_rpc("get_storage_pool_info", payload, Some(Duration::from_secs(10))).await
    }

    /// 处理异步通知
    async fn handle_async_notification(
        msg: RpcMessage,
        handler_registry: &Arc<RwLock<RpcHandlerRegistry>>,
        tx: &mpsc::UnboundedSender<RpcMessage>,
    ) {
        let method = match &msg.method {
            Some(m) => m,
            None => {
                error!("异步通知缺少方法名");
                return;
            }
        };

        let payload = msg.payload.clone().unwrap_or(serde_json::Value::Null);

        // 根据方法名处理不同的异步通知
        match method.as_str() {
            "stop_vm_async" => {
                debug!("处理异步停止虚拟机通知");
                let registry = handler_registry.read().await;
                
                // 调用处理器中的异步停止方法
                match registry.handle_stop_vm_async_internal(payload).await {
                    Ok(_) => {
                        debug!("异步停止虚拟机通知处理完成");
                    }
                    Err(e) => {
                        error!("处理异步停止虚拟机失败: {}", e);
                    }
                }
            }
            _ => {
                debug!("未知的异步通知方法: {}", method);
            }
        }
    }

    /// 获取系统资源信息
    fn get_system_resource_info(&self) -> Result<NodeResourceInfo, Box<dyn std::error::Error + Send + Sync>> {
        use sysinfo::{System, Disks};
        
        let mut sys = System::new_all();
        sys.refresh_all();
        
        // 获取 CPU 信息
        let cpu_cores = sys.cpus().len() as u32;
        let cpu_threads = sys.cpus().len() as u32;
        
        // 获取内存信息
        let memory_total = sys.total_memory() * 1024; // 转换为字节
        
        // 获取磁盘信息
        let disks = Disks::new_with_refreshed_list();
        let disk_total = disks.list().iter()
            .map(|disk| disk.total_space())
            .sum();
        
        // 获取虚拟化信息（简化实现）
        let hypervisor_type = self.detect_hypervisor_type();
        let hypervisor_version = self.detect_hypervisor_version();
        
        Ok(NodeResourceInfo {
            node_id: self.node_id.clone(),
            cpu_cores,
            cpu_threads,
            memory_total,
            disk_total,
            hypervisor_type: Some(hypervisor_type),
            hypervisor_version: Some(hypervisor_version),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// 检测虚拟化类型
    fn detect_hypervisor_type(&self) -> String {
        // 简化实现，实际应该检测系统虚拟化能力
        if std::path::Path::new("/dev/kvm").exists() {
            "kvm".to_string()
        } else if std::path::Path::new("/usr/bin/qemu-system-x86_64").exists() {
            "qemu".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// 检测虚拟化版本
    fn detect_hypervisor_version(&self) -> String {
        // 简化实现，实际应该执行命令获取版本
        "unknown".to_string()
    }

    /// 发送节点资源信息
    async fn send_node_resource_info(
        &self,
        tx: &mpsc::UnboundedSender<RpcMessage>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resource_info = self.get_system_resource_info()?;
        
        let resource_msg = RpcMessage::notification(
            "node_resource_info",
            serde_json::to_value(&resource_info)?,
        );
        
        tx.send(resource_msg)
            .map_err(|_| "发送节点资源信息失败".to_string())?;
        
        info!("✅ 已发送节点资源信息: cpu_cores={}, cpu_threads={}, memory_total={}, disk_total={}", 
              resource_info.cpu_cores, resource_info.cpu_threads, 
              resource_info.memory_total, resource_info.disk_total);
        
        Ok(())
    }
}


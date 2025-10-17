/// WebSocket 连接处理器
/// 
/// 处理与 Agent 的 WebSocket 连接和消息

use super::AgentConnectionManager;
use axum::extract::ws::{Message as AxumWsMessage, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use common::ws_rpc::{RpcMessage, MessageType, RegisterRequest, RegisterResponse, NodeResourceInfo, NodeResourceInfoResponse};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use crate::services::node_service::NodeService;

/// WebSocket 升级处理器
pub async fn handle_agent_websocket(
    ws: WebSocketUpgrade,
    State(state): State<crate::app_state::AppState>,
) -> impl IntoResponse {
    let manager = state.agent_manager();
    ws.on_upgrade(move |socket| handle_agent_connection(socket, manager, state))
}

/// 处理 Agent WebSocket 连接
async fn handle_agent_connection(socket: WebSocket, manager: AgentConnectionManager, state: crate::app_state::AppState) {
    info!("新的 Agent WebSocket 连接");

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建消息发送通道
    let (tx, mut rx) = mpsc::unbounded_channel::<RpcMessage>();

    // 等待注册消息
    let (node_id, hostname, ip_address) = match wait_for_registration(&mut ws_receiver, &state).await {
        Ok(info) => info,
        Err(e) => {
            error!("Agent 注册失败: {}", e);
            let _ = ws_sender.close().await;
            return;
        }
    };

    // 发送注册成功响应
    let register_response = RegisterResponse {
        success: true,
        message: "注册成功".to_string(),
    };
    
    let response_msg = RpcMessage::response(
        "register",
        serde_json::to_value(&register_response).unwrap(),
    );
    
    if let Err(e) = send_message(&mut ws_sender, response_msg).await {
        error!("发送注册响应失败: {}", e);
        return;
    }

    // 注册到管理器
    let connection = manager.register(
        node_id.clone(),
        hostname.clone(),
        ip_address.clone(),
        tx.clone(),
    ).await;

    info!("Agent 已连接并注册: node_id={}, hostname={}, ip={}", 
          node_id, hostname, ip_address);

    // 创建消息发送任务
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = send_message(&mut ws_sender, msg).await {
                error!("发送消息失败: {}", e);
                break;
            }
        }
        debug!("消息发送任务结束");
    });

    // 创建消息接收任务
    let connection_clone = connection.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => {
                    if let Err(e) = handle_incoming_message(msg, &connection_clone, &state).await {
                        warn!("处理消息失败: {}", e);
                    }
                }
                Err(e) => {
                    error!("接收消息错误: {}", e);
                    break;
                }
            }
        }
        debug!("消息接收任务结束");
    });

    // 等待任一任务完成
    tokio::select! {
        _ = &mut send_task => {
            debug!("发送任务已结束");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug!("接收任务已结束");
            send_task.abort();
        }
    }

    // 清理：从管理器中注销
    manager.unregister(&node_id).await;
    info!("Agent 连接已关闭: {}", node_id);
}

/// 等待并处理注册消息
async fn wait_for_registration(
    receiver: &mut futures_util::stream::SplitStream<WebSocket>,
    state: &crate::app_state::AppState,
) -> Result<(String, String, String), String> {
    // 等待第一条消息（应该是注册请求）
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        receiver.next()
    ).await {
        Ok(Some(Ok(msg))) => {
            let rpc_msg = parse_websocket_message(msg)
                .map_err(|e| format!("解析注册消息失败: {}", e))?;

            // 验证是否是注册请求
            if rpc_msg.message_type != MessageType::Request {
                return Err("期望收到注册请求".to_string());
            }

            if rpc_msg.method.as_deref() != Some("register") {
                return Err(format!("期望 register 方法，收到: {:?}", rpc_msg.method));
            }

            // 解析注册信息
            let payload = rpc_msg.payload.ok_or("缺少注册信息")?;
            let register_req: RegisterRequest = serde_json::from_value(payload)
                .map_err(|e| format!("解析注册信息失败: {}", e))?;

            // 检查并创建节点
            let node_service = NodeService::new(state.clone());
            
            // 检查节点是否已存在
            match node_service.node_exists(&register_req.node_id).await {
                Ok(exists) => {
                    if !exists {
                        // 节点不存在，创建新节点
                        let create_dto = crate::db::models::node::CreateNodeDto {
                            hostname: register_req.hostname.clone(),
                            ip_address: register_req.ip_address.clone(),
                            hypervisor_type: None,
                            hypervisor_version: None,
                            metadata: None,
                        };
                        
                        match node_service.create_node_with_id(
                            register_req.node_id.clone(),
                            create_dto,
                        ).await {
                            Ok(_) => {
                                info!("成功创建新节点: node_id={}, hostname={}, ip={}", 
                                      register_req.node_id, register_req.hostname, register_req.ip_address);
                            }
                            Err(e) => {
                                error!("创建节点失败: node_id={}, error={}", register_req.node_id, e);
                                return Err(format!("创建节点失败: {}", e));
                            }
                        }
                    } else {
                        info!("节点已存在，更新连接: node_id={}", register_req.node_id);
                    }
                }
                Err(e) => {
                    error!("检查节点存在性失败: node_id={}, error={}", register_req.node_id, e);
                    return Err(format!("检查节点失败: {}", e));
                }
            }

            Ok((register_req.node_id, register_req.hostname, register_req.ip_address))
        }
        Ok(Some(Err(e))) => Err(format!("接收注册消息错误: {}", e)),
        Ok(None) => Err("连接已关闭".to_string()),
        Err(_) => Err("等待注册消息超时".to_string()),
    }
}

/// 处理收到的消息
async fn handle_incoming_message(
    ws_msg: AxumWsMessage,
    connection: &super::agent_manager::AgentConnection,
    state: &crate::app_state::AppState,
) -> Result<(), String> {
    let rpc_msg = parse_websocket_message(ws_msg)?;

    debug!("收到消息: type={:?}, method={:?}, id={}", 
           rpc_msg.message_type, rpc_msg.method, rpc_msg.id);

    match rpc_msg.message_type {
        MessageType::Notification => {
            handle_notification(rpc_msg, connection, &state).await
        }
        MessageType::Request => {
            // Agent 发起的请求（目前主要是心跳等）
            handle_agent_request(rpc_msg, connection, &state).await
        }
        MessageType::Response => {
            // 对 Server 请求的响应 - 唤醒等待的请求
            debug!("收到响应消息: {}", rpc_msg.id);
            connection.handle_response(rpc_msg).await;
            Ok(())
        }
        MessageType::Stream => {
            // 流式数据
            debug!("收到流式消息: {}", rpc_msg.id);
            Ok(())
        }
    }
}

/// 处理通知消息
async fn handle_notification(
    msg: RpcMessage,
    connection: &super::agent_manager::AgentConnection,
    state: &crate::app_state::AppState,
) -> Result<(), String> {
    let method = msg.method.as_deref().ok_or("通知消息缺少方法名")?;

    match method {
        "heartbeat" => {
            // 更新心跳时间
            connection.update_heartbeat().await;
            debug!("收到心跳: node_id={}", connection.node_id);
            
            // 调用NodeService更新节点最后心跳时间
            let node_service = NodeService::new(state.clone());
            
            if let Err(e) = node_service.update_heartbeat(&connection.node_id).await {
                error!("更新节点心跳失败: node_id={}, error={}", connection.node_id, e);
            } else {
                debug!("成功更新节点心跳: node_id={}", connection.node_id);
            }
            
            Ok(())
        }
        "node_status_update" => {
            debug!("收到节点状态更新: node_id={}", connection.node_id);
            // TODO: 更新节点状态到数据库
            Ok(())
        }
        "vm_status_change" => {
            debug!("收到虚拟机状态变更: node_id={}", connection.node_id);
            // TODO: 更新虚拟机状态到数据库
            Ok(())
        }
        "vm_operation_completed" => {
            debug!("收到虚拟机操作完成通知: node_id={}", connection.node_id);
            handle_vm_operation_completed(msg, connection, &state).await
        }
        "node_resource_info" => {
            debug!("收到节点资源信息上报: node_id={}", connection.node_id);
            handle_node_resource_info(msg, connection, &state).await
        }
        _ => {
            warn!("未知的通知方法: {}", method);
            Ok(())
        }
    }
}

/// 处理 Agent 发起的请求
async fn handle_agent_request(
    msg: RpcMessage,
    connection: &super::agent_manager::AgentConnection,
    state: &crate::app_state::AppState,
) -> Result<(), String> {
    let method = msg.method.as_deref().ok_or("请求消息缺少方法名")?;

    match method {
        "get_storage_pool_info" => {
            // 处理获取存储池信息请求
            handle_get_storage_pool_info(msg, connection, &state).await
        }
        _ => {
            warn!("未知的请求方法: {}", method);
            
            // 返回方法不存在错误
            let error_response = RpcMessage::error_response(
                msg.id,
                "METHOD_NOT_FOUND",
                format!("方法不存在: {}", method),
                None,
            );
            
            connection.sender.send(error_response)
                .map_err(|_| "发送错误响应失败".to_string())?;
            
            Ok(())
        }
    }
}

/// 解析 WebSocket 消息为 RPC 消息
fn parse_websocket_message(ws_msg: AxumWsMessage) -> Result<RpcMessage, String> {
    match ws_msg {
        AxumWsMessage::Text(text) => {
            RpcMessage::from_json(&text)
                .map_err(|e| format!("解析 JSON 失败: {}", e))
        }
        AxumWsMessage::Binary(data) => {
            let text = String::from_utf8(data)
                .map_err(|e| format!("二进制转字符串失败: {}", e))?;
            RpcMessage::from_json(&text)
                .map_err(|e| format!("解析 JSON 失败: {}", e))
        }
        AxumWsMessage::Close(_) => {
            Err("连接关闭".to_string())
        }
        _ => {
            Err("不支持的消息类型".to_string())
        }
    }
}

/// 发送 RPC 消息
async fn send_message(
    sender: &mut futures_util::stream::SplitSink<WebSocket, AxumWsMessage>,
    msg: RpcMessage,
) -> Result<(), String> {
    let json = msg.to_json()
        .map_err(|e| format!("序列化消息失败: {}", e))?;
    
    sender.send(AxumWsMessage::Text(json))
        .await
        .map_err(|e| format!("发送 WebSocket 消息失败: {}", e))?;
    
    Ok(())
}

/// 处理虚拟机操作完成通知
async fn handle_vm_operation_completed(
    msg: RpcMessage,
    connection: &super::agent_manager::AgentConnection,
    state: &crate::app_state::AppState,
) -> Result<(), String> {
    let payload = msg.payload.ok_or("通知消息缺少负载")?;
    
    // 解析通知数据
    let vm_id: String = payload.get("vm_id")
        .and_then(|v| v.as_str())
        .ok_or("缺少 vm_id")?
        .to_string();
    
    let task_id: String = payload.get("task_id")
        .and_then(|v| v.as_str())
        .ok_or("缺少 task_id")?
        .to_string();
    
    let operation: String = payload.get("operation")
        .and_then(|v| v.as_str())
        .ok_or("缺少 operation")?
        .to_string();
    
    let success: bool = payload.get("success")
        .and_then(|v| v.as_bool())
        .ok_or("缺少 success")?;
    
    let message: String = payload.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    info!("虚拟机操作完成: vm_id={}, task_id={}, operation={}, success={}, message={}", 
          vm_id, task_id, operation, success, message);

    // 使用任务服务更新状态
    let task_service = crate::services::task_service::TaskService::new(state.clone());
    
    match task_service.handle_vm_operation_completed_by_task_id(
        &task_id,
        &vm_id,
        &operation,
        success,
        &message,
    ).await {
        Ok(_) => {
            if success {
                info!("虚拟机 {} 操作成功 (task_id: {}): {}", vm_id, task_id, message);
            } else {
                error!("虚拟机 {} 操作失败 (task_id: {}): {}", vm_id, task_id, message);
            }
        }
        Err(e) => {
            error!("更新任务状态失败 (task_id: {}): {}", task_id, e);
        }
    }

    Ok(())
}

/// 处理获取存储池信息请求
async fn handle_get_storage_pool_info(
    msg: RpcMessage,
    connection: &super::agent_manager::AgentConnection,
    state: &crate::app_state::AppState,
) -> Result<(), String> {
    // 解析请求参数
    let payload = msg.payload.ok_or("请求缺少负载")?;
    let pool_id = payload.get("pool_id")
        .and_then(|v| v.as_str())
        .ok_or("缺少 pool_id 参数")?;

    debug!("Agent 请求获取存储池信息: pool_id={}", pool_id);

    // 使用存储服务获取存储池信息
    let storage_service = crate::services::storage_service::StorageService::new(state.clone());
    
    match storage_service.get_storage_pool(pool_id).await {
        Ok(pool) => {
            // 转换为 Agent 需要的格式
            let pool_info = serde_json::json!({
                "pool_id": pool.id,
                "pool_name": pool.name,
                "pool_type": pool.pool_type,
                "config": pool.config
            });

            let response = RpcMessage::response(msg.id, pool_info);
            
            connection.sender.send(response)
                .map_err(|_| "发送存储池信息响应失败".to_string())?;
        }
        Err(e) => {
            error!("获取存储池信息失败: {}", e);
            
            let error_response = RpcMessage::error_response(
                msg.id,
                "STORAGE_POOL_NOT_FOUND",
                format!("存储池不存在: {}", pool_id),
                None,
            );
            
            connection.sender.send(error_response)
                .map_err(|_| "发送错误响应失败".to_string())?;
        }
    }
    
    Ok(())
}

/// 处理节点资源信息上报
async fn handle_node_resource_info(
    msg: RpcMessage,
    connection: &super::agent_manager::AgentConnection,
    state: &crate::app_state::AppState,
) -> Result<(), String> {
    let payload = msg.payload.ok_or("通知消息缺少负载")?;
    
    // 解析节点资源信息
    let resource_info: NodeResourceInfo = serde_json::from_value(payload)
        .map_err(|e| format!("解析节点资源信息失败: {}", e))?;

    info!("收到节点资源信息: node_id={}, cpu_cores={}, cpu_threads={}, memory_total={}, disk_total={}", 
          resource_info.node_id, resource_info.cpu_cores, resource_info.cpu_threads, 
          resource_info.memory_total, resource_info.disk_total);

    // 使用节点服务更新节点资源信息
    let node_service = NodeService::new(state.clone());
    
    match node_service.update_node_resource_info(
        &resource_info.node_id,
        resource_info.cpu_cores,
        resource_info.cpu_threads,
        resource_info.memory_total,
        resource_info.disk_total,
        resource_info.hypervisor_type,
        resource_info.hypervisor_version,
    ).await {
        Ok(_) => {
            info!("成功更新节点资源信息: node_id={}", resource_info.node_id);
        }
        Err(e) => {
            error!("更新节点资源信息失败: node_id={}, error={}", resource_info.node_id, e);
        }
    }

    Ok(())
}



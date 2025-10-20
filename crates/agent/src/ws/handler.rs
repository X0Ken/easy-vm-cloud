/// RPC 请求处理器
/// 
/// 注册和调度 Agent 端的 RPC 方法处理器

use common::ws_rpc::{RpcMessage, RpcError, RpcErrorCode};
use common::ws_rpc::types::*;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use tokio::sync::mpsc;

use crate::hypervisor::{HypervisorManager, DiskBusType, DiskDeviceType};
use crate::storage::StorageManager;
use crate::network::NetworkManager;
use crate::ws::client::WsClient;

/// RPC 处理器注册表
pub struct RpcHandlerRegistry {
    hypervisor: Arc<HypervisorManager>,
    storage: Arc<StorageManager>,
    network: Arc<NetworkManager>,
    /// 通知发送器，用于向 Server 发送通知
    notification_sender: Option<mpsc::UnboundedSender<RpcMessage>>,
    /// WebSocket 客户端引用，用于主动调用 Server RPC
    ws_client: Option<Arc<WsClient>>,
}

impl RpcHandlerRegistry {
    /// 创建新的处理器注册表
    pub fn new(
        hypervisor: Arc<HypervisorManager>,
        storage: Arc<StorageManager>,
        network: Arc<NetworkManager>,
    ) -> Self {
        Self {
            hypervisor,
            storage,
            network,
            notification_sender: None,
            ws_client: None,
        }
    }

    /// 设置通知发送器
    pub fn set_notification_sender(&mut self, sender: mpsc::UnboundedSender<RpcMessage>) {
        self.notification_sender = Some(sender);
    }

    /// 设置 WebSocket 客户端引用
    pub fn set_ws_client(&mut self, client: Arc<WsClient>) {
        self.ws_client = Some(client);
    }

    /// 确保存储池已注册，如果未注册则从 Server 获取信息并注册
    async fn ensure_storage_pool_registered(&self, pool_id: &str) -> Result<(), RpcError> {
        // 检查存储池是否已注册
        if self.storage.is_pool_registered(pool_id).await {
            debug!("存储池 {} 已注册", pool_id);
            return Ok(());
        }

        info!("存储池 {} 未注册，尝试从 Server 获取信息", pool_id);

        // 从 Server 获取存储池信息
        let ws_client = match &self.ws_client {
            Some(client) => client,
            None => {
                return Err(RpcError::new(
                    RpcErrorCode::InternalError,
                    "WebSocket 客户端未初始化".to_string(),
                ));
            }
        };

        // 调用 Server RPC 获取存储池信息
        let pool_info = match ws_client.get_storage_pool_info(pool_id).await {
            Ok(info) => info,
            Err(e) => {
                error!("从 Server 获取存储池信息失败: {}", e);
                return Err(RpcError::new(
                    RpcErrorCode::StorageError,
                    format!("获取存储池信息失败: {}", e),
                ));
            }
        };

        // 解析存储池配置
        let pool_config = match serde_json::from_value::<StoragePoolConfig>(pool_info) {
            Ok(config) => config,
            Err(e) => {
                error!("解析存储池配置失败: {}", e);
                return Err(RpcError::new(
                    RpcErrorCode::StorageError,
                    format!("解析存储池配置失败: {}", e),
                ));
            }
        };

        // 转换为 Agent 的存储池配置
        let agent_pool_config = crate::storage::driver::StoragePoolConfig {
            pool_id: pool_config.pool_id.clone(),
            pool_name: pool_config.pool_name.clone(),
            storage_type: pool_config.pool_type.clone(),
            config: pool_config.config.clone(),
        };

        // 注册存储池
        match self.storage.register_pool(agent_pool_config).await {
            Ok(_) => {
                info!("成功注册存储池: {}", pool_id);
                Ok(())
            }
            Err(e) => {
                error!("注册存储池失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::StorageError,
                    format!("注册存储池失败: {}", e),
                ))
            }
        }
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

        debug!("处理 RPC 请求: method={}, id={}", method, msg.id);

        // 路由到对应的处理方法
        let result = match method.as_str() {
            // 节点信息
            "get_node_info" => self.handle_get_node_info(payload).await,
            
            // 存储管理
            "create_volume" => self.handle_create_volume(payload).await,
            "delete_volume" => self.handle_delete_volume(payload).await,
            "resize_volume" => self.handle_resize_volume(payload).await,
            "snapshot_volume" => self.handle_snapshot_volume(payload).await,
            "clone_volume" => self.handle_clone_volume(payload).await,
            "get_volume_info" => self.handle_get_volume_info(payload).await,
            "list_volumes" => self.handle_list_volumes(payload).await,
            
            // 网络管理
            "create_network" => self.handle_create_network(payload).await,
            "delete_network" => self.handle_delete_network(payload).await,
            "attach_interface" => self.handle_attach_interface(payload).await,
            "detach_interface" => self.handle_detach_interface(payload).await,
            
            // 虚拟机存储卷管理
            "attach_volume" => self.handle_attach_volume(payload).await,
            "detach_volume" => self.handle_detach_volume(payload).await,
            // 异步卷操作通过通知
            
            _ => {
                return RpcMessage::error_response(
                    msg.id,
                    RpcErrorCode::MethodNotFound.as_str(),
                    format!("方法不存在: {}", method),
                    None,
                );
            }
        };

        match result {
            Ok(response_payload) => RpcMessage::response(msg.id, response_payload),
            Err(err) => RpcMessage::error_response(
                msg.id,
                err.code.as_str(),
                err.message,
                err.details,
            ),
        }
    }

    /// 处理异步通知的统一入口
    /// 
    /// 根据通知的方法名路由到对应的处理逻辑
    pub async fn handle_notification(&self, method: &str, payload: serde_json::Value) -> Result<(), RpcError> {
        debug!("处理异步通知: method={}", method);
        
        match method {
            "stop_vm_async" => {
                self.handle_stop_vm_async_internal(payload).await
            }
            "start_vm_async" => {
                self.handle_start_vm_async_internal(payload).await
            }
            "restart_vm_async" => {
                self.handle_restart_vm_async_internal(payload).await
            }
            "attach_volume_async" => {
                self.handle_attach_volume_async_internal(payload).await
            }
            "detach_volume_async" => {
                self.handle_detach_volume_async_internal(payload).await
            }
            _ => {
                debug!("未知的异步通知方法: {}", method);
                Ok(())
            }
        }
    }

    // ========================================================================
    // 节点信息处理
    // ========================================================================

    async fn handle_get_node_info(&self, _payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        info!("获取节点信息");
        
        // TODO: 从 hypervisor 获取真实的节点信息
        let node_info = NodeInfo {
            node_id: std::env::var("NODE_ID").unwrap_or_else(|_| "unknown".to_string()),
            hostname: hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
            ip_address: "127.0.0.1".to_string(),
            resources: None,
            hypervisor_type: "kvm".to_string(),
            hypervisor_version: "unknown".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        serde_json::to_value(&node_info)
            .map_err(|e| RpcError::serialization_error(e))
    }

    /// 处理异步启动虚拟机（内部方法，用于通知处理）
    async fn handle_start_vm_async_internal(&self, payload: serde_json::Value) -> Result<(), RpcError> {
        let req: serde_json::Value = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        let vm_id = req.get("vm_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 vm_id 参数".to_string()))?;

        let name = req.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 name 参数".to_string()))?;

        let vcpu = req.get("vcpu")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| RpcError::invalid_params("缺少 vcpu 参数".to_string()))?;

        let memory_mb = req.get("memory_mb")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| RpcError::invalid_params("缺少 memory_mb 参数".to_string()))?;

        let os_type = req.get("os_type")
            .and_then(|v| v.as_str())
            .unwrap_or("linux");

        info!("异步启动虚拟机: vm_id={}, name={}", vm_id, name);

        // 解析磁盘配置
        let mut volumes = Vec::new();
        if let Some(volumes_json) = req.get("volumes") {
            if let Ok(volumes_array) = serde_json::from_value::<Vec<serde_json::Value>>(volumes_json.clone()) {
                for volume_json in volumes_array {
                    if let Ok(volume) = serde_json::from_value::<crate::hypervisor::VolumeConfig>(volume_json) {
                        volumes.push(volume);
                    }
                }
            }
        }

        // 解析网络配置
        let mut networks = Vec::new();
        if let Some(networks_json) = req.get("networks") {
            if let Ok(networks_array) = serde_json::from_value::<Vec<serde_json::Value>>(networks_json.clone()) {
                for network_json in networks_array {
                    if let Ok(network) = serde_json::from_value::<crate::hypervisor::NetworkConfig>(network_json) {
                        networks.push(network);
                    }
                }
            }
        }

        // 确保网络配置：检查每个网络对应的 Bridge 是否存在，如果不存在则自动创建
        for network_config in &networks {
            if let Err(e) = self.ensure_network_bridge(&network_config.network_name, &network_config.bridge_name).await {
                error!("网络配置失败: network_id={}, bridge={}, error={}",
                       network_config.network_name, network_config.bridge_name, e);
                return Err(RpcError::new(
                    RpcErrorCode::NetworkError,
                    format!("网络配置失败: {}", e),
                ));
            }
        }

        // 构建虚拟机配置
        let config = crate::hypervisor::VMConfig {
            name: name.to_string(),
            uuid: vm_id.to_string(),
            vcpu: vcpu as u32,
            memory_mb: memory_mb as u64,
            os_type: os_type.to_string(),
            volumes,
            networks,
        };

        // 异步执行启动操作，不等待结果
        let hypervisor = self.hypervisor.clone();
        let vm_id = vm_id.to_string();
        let notification_sender = self.notification_sender.clone();
        
        tokio::spawn(async move {
            match hypervisor.start_vm_with_config(&vm_id, &config).await {
                Ok(_) => {
                    info!("虚拟机 {} 异步启动成功", vm_id);
                    
                    // 发送成功通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "start_vm",
                                "success": true,
                                "message": "虚拟机启动成功"
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送完成通知失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("虚拟机 {} 异步启动失败: {}", vm_id, e);
                    
                    // 发送失败通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "start_vm",
                                "success": false,
                                "message": format!("虚拟机启动失败: {}", e)
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送失败通知失败: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// 处理异步停止虚拟机（内部方法，用于通知处理）
    pub async fn handle_stop_vm_async_internal(&self, payload: serde_json::Value) -> Result<(), RpcError> {
        let req: VmAsyncOperationRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("处理异步停止虚拟机通知: vm_id={}", req.vm_id);

        // 异步执行停止操作，不等待结果
        let hypervisor = self.hypervisor.clone();
        let vm_id = req.vm_id.clone();
        let force = req.force;
        let notification_sender = self.notification_sender.clone();
        
        tokio::spawn(async move {
            match hypervisor.stop_vm(&vm_id, force).await {
                Ok(_) => {
                    info!("虚拟机 {} 异步停止成功", vm_id);
                    
                    // 发送成功通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "stop_vm",
                                "success": true,
                                "message": "虚拟机停止成功"
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送完成通知失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("虚拟机 {} 异步停止失败: {}", vm_id, e);
                    
                    // 发送失败通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "stop_vm",
                                "success": false,
                                "message": format!("虚拟机停止失败: {}", e)
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送失败通知失败: {}", e);
                        }
                    }
                }
            }
        });

        // 通知处理完成，不返回响应
        Ok(())
    }

    /// 处理异步重启虚拟机（内部方法，用于通知处理）
    async fn handle_restart_vm_async_internal(&self, payload: serde_json::Value) -> Result<(), RpcError> {
        let req: serde_json::Value = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        let vm_id = req.get("vm_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 vm_id 参数".to_string()))?;

        let force = req.get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        info!("异步重启虚拟机: vm_id={}, force={}", vm_id, force);

        // 异步执行重启操作：优雅停止（失败则强制）+ 再启动
        let hypervisor = self.hypervisor.clone();
        let vm_id_string = vm_id.to_string();
        let notification_sender = self.notification_sender.clone();

        tokio::spawn(async move {
            // 优雅停止
            let stop_result = match hypervisor.stop_vm(&vm_id_string, force).await {
                Ok(v) => Ok(v),
                Err(_) => hypervisor.stop_vm(&vm_id_string, true).await,
            };

            match stop_result {
                Ok(_) => {
                    // 等2秒再启动
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    match hypervisor.start_vm(&vm_id_string).await {
                        Ok(_) => {
                            info!("虚拟机 {} 异步重启成功", vm_id_string);
                            if let Some(sender) = notification_sender {
                                let notification = RpcMessage::notification(
                                    "vm_operation_completed",
                                    serde_json::json!({
                                        "vm_id": vm_id_string,
                                        "operation": "restart_vm",
                                        "success": true,
                                        "message": "虚拟机重启成功"
                                    }),
                                );
                                if let Err(e) = sender.send(notification) {
                                    error!("发送重启完成通知失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("虚拟机 {} 启动失败(重启流程): {}", vm_id_string, e);
                            if let Some(sender) = notification_sender {
                                let notification = RpcMessage::notification(
                                    "vm_operation_completed",
                                    serde_json::json!({
                                        "vm_id": vm_id_string,
                                        "operation": "restart_vm",
                                        "success": false,
                                        "message": format!("虚拟机重启失败(启动阶段): {}", e)
                                    }),
                                );
                                if let Err(e) = sender.send(notification) {
                                    error!("发送重启失败通知失败: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("虚拟机 {} 停止失败(重启流程): {}", vm_id_string, e);
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id_string,
                                "operation": "restart_vm",
                                "success": false,
                                "message": format!("虚拟机重启失败(停止阶段): {}", e)
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送重启失败通知失败: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    // ========================================================================
    // 存储管理处理
    // ========================================================================

    async fn handle_create_volume(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: CreateVolumeRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("创建存储卷: {} (ID: {})", req.name, req.volume_id);

        // 使用请求中的存储池ID
        let pool_id = &req.pool_id;

        // 确保存储池已注册（自动从 Server 获取信息并注册）
        if let Err(e) = self.ensure_storage_pool_registered(pool_id).await {
            error!("确保存储池注册失败: {}", e);
            return Err(e);
        }

        match self.storage.create_volume(
            pool_id,
            &req.volume_id,
            &req.name,
            req.size_gb,
            &req.format,
            req.source.as_deref(),  // 传递source参数到存储层
        ).await {
            Ok(volume_info) => {
                let response = CreateVolumeResponse {
                    success: true,
                    message: "存储卷创建成功".to_string(),
                    path: Some(volume_info.path),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("创建存储卷失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::VolumeCreateFailed,
                    format!("创建存储卷失败: {}", e),
                ))
            }
        }
    }

    async fn handle_delete_volume(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: DeleteVolumeRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("删除存储卷: {}", req.volume_id);

        // 确保存储池已注册
        if let Err(e) = self.ensure_storage_pool_registered(&req.pool_id).await {
            error!("确保存储池注册失败: {}", e);
            return Err(e);
        }

        match self.storage.delete_volume(&req.pool_id, &req.volume_id).await {
            Ok(_) => {
                let response = DeleteVolumeResponse {
                    success: true,
                    message: "存储卷已删除".to_string(),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("删除存储卷失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::VolumeDeleteFailed,
                    format!("删除存储卷失败: {}", e),
                ))
            }
        }
    }

    async fn handle_resize_volume(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: ResizeVolumeRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("调整存储卷大小: {} -> {} GB", req.volume_id, req.new_size_gb);

        // 确保存储池已注册
        if let Err(e) = self.ensure_storage_pool_registered(&req.pool_id).await {
            error!("确保存储池注册失败: {}", e);
            return Err(e);
        }

        match self.storage.resize_volume(&req.pool_id, &req.volume_id, req.new_size_gb).await {
            Ok(_) => {
                let response = ResizeVolumeResponse {
                    success: true,
                    message: "存储卷大小已调整".to_string(),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("调整存储卷大小失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::StorageError,
                    format!("调整存储卷大小失败: {}", e),
                ))
            }
        }
    }

    async fn handle_snapshot_volume(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let _req: SnapshotVolumeRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("创建存储卷快照: {} -> {}", _req.volume_id, _req.snapshot_name);

        // TODO: 实现快照功能
        match Err::<String, common::Error>(common::Error::Internal("快照功能未实现".to_string())) {
            Ok(snapshot_id) => {
                let response = SnapshotVolumeResponse {
                    success: true,
                    message: "快照创建成功".to_string(),
                    snapshot_id: Some(snapshot_id),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("创建快照失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::StorageError,
                    format!("创建快照失败: {}", e),
                ))
            }
        }
    }

    async fn handle_clone_volume(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: CloneVolumeRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("克隆存储卷: {} -> {} (名称: {})", req.source_volume_id, req.target_volume_id, req.target_name);

        // 确保存储池已注册
        if let Err(e) = self.ensure_storage_pool_registered(&req.pool_id).await {
            error!("确保存储池注册失败: {}", e);
            return Err(e);
        }

        match self.storage.clone_volume(
            &req.pool_id,
            &req.source_volume_id,
            &req.target_volume_id,
            &req.target_name,
        ).await {
            Ok(volume_info) => {
                let response = CloneVolumeResponse {
                    success: true,
                    message: "存储卷克隆成功".to_string(),
                    path: Some(volume_info.path),
                };
                Ok(serde_json::to_value(response).unwrap())
            }
            Err(e) => {
                error!("克隆存储卷失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::StorageError,
                    format!("克隆存储卷失败: {}", e),
                ))
            }
        }
    }

    async fn handle_get_volume_info(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: GetVolumeInfoRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("获取存储卷信息: {}", req.volume_id);

        // 确保存储池已注册
        if let Err(e) = self.ensure_storage_pool_registered(&req.pool_id).await {
            error!("确保存储池注册失败: {}", e);
            return Err(e);
        }

        match self.storage.get_volume_info(&req.pool_id, &req.volume_id).await {
            Ok(volume_info) => {
                serde_json::to_value(&volume_info)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("获取存储卷信息失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::VolumeNotFound,
                    format!("存储卷不存在: {}", req.volume_id),
                ))
            }
        }
    }

    async fn handle_list_volumes(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: ListVolumesRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("列出存储卷: pool_id={:?}", req.pool_id);

        let pool_id = req.pool_id.as_deref().unwrap_or("");
        
        // 如果指定了存储池，确保已注册
        if !pool_id.is_empty() {
            if let Err(e) = self.ensure_storage_pool_registered(pool_id).await {
                error!("确保存储池注册失败: {}", e);
                return Err(e);
            }
        }

        match self.storage.list_volumes(pool_id).await {
            Ok(volumes) => {
                // 转换为 common::ws_rpc::VolumeInfo
                let rpc_volumes: Vec<common::ws_rpc::VolumeInfo> = volumes.iter().map(|v| {
                    common::ws_rpc::VolumeInfo {
                        volume_id: v.volume_id.clone(),
                        name: v.name.clone(),
                        path: v.path.clone(),
                        size_gb: v.size_gb,
                        actual_size_gb: v.actual_size_gb,
                        format: v.format.clone(),
                        status: v.status.clone(),
                    }
                }).collect();
                
                let response = ListVolumesResponse { volumes: rpc_volumes };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("列出存储卷失败: {}", e);
                Err(RpcError::internal_error(format!("列出存储卷失败: {}", e)))
            }
        }
    }

    // ========================================================================
    // 网络管理处理
    // ========================================================================

    async fn handle_create_network(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: CreateNetworkRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("创建网络: {} (ID: {})", req.name, req.network_id);

        let vlan_id = req.vlan_id.as_ref().and_then(|v| v.parse::<u32>().ok());
        match self.network.create_network(
            &req.network_id,
            &req.name,
            &req.network_type,
            &req.bridge_name,
            vlan_id,
        ).await {
            Ok(_) => {
                let response = CreateNetworkResponse {
                    success: true,
                    message: "网络创建成功".to_string(),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("创建网络失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::NetworkCreateFailed,
                    format!("创建网络失败: {}", e),
                ))
            }
        }
    }

    async fn handle_delete_network(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: DeleteNetworkRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("删除网络: {}", req.network_id);

        match self.network.delete_network(&req.network_id, "bridge", None).await {
            Ok(_) => {
                let response = DeleteNetworkResponse {
                    success: true,
                    message: "网络已删除".to_string(),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("删除网络失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::NetworkDeleteFailed,
                    format!("删除网络失败: {}", e),
                ))
            }
        }
    }

    async fn handle_attach_interface(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: AttachInterfaceRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("附加网络接口到虚拟机: {}", req.vm_id);

        match self.network.attach_interface(&req.vm_id, &req.interface.bridge_name).await {
            Ok(_) => {
                let response = AttachInterfaceResponse {
                    success: true,
                    message: "网络接口已附加".to_string(),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("附加网络接口失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::NetworkError,
                    format!("附加网络接口失败: {}", e),
                ))
            }
        }
    }

    async fn handle_detach_interface(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let req: DetachInterfaceRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        info!("从虚拟机分离网络接口: {}", req.vm_id);

        match self.network.detach_interface(&req.vm_id, &req.mac_address).await {
            Ok(_) => {
                let response = DetachInterfaceResponse {
                    success: true,
                    message: "网络接口已分离".to_string(),
                };
                serde_json::to_value(&response)
                    .map_err(|e| RpcError::serialization_error(e))
            }
            Err(e) => {
                error!("分离网络接口失败: {}", e);
                Err(RpcError::new(
                    RpcErrorCode::NetworkError,
                    format!("分离网络接口失败: {}", e),
                ))
            }
        }
    }
    
    /// 确保网络 Bridge 存在并可用，如果不存在则根据网络信息自动创建
    /// 
    /// 功能：
    /// 1. 检查 Bridge 是否存在
    /// 2. 如果不存在，从 Bridge 名称推断 VLAN ID 并自动创建网络
    /// 3. 验证 Bridge 是否启动并可用
    async fn ensure_network_bridge(&self, network_id: &str, bridge_name: &str) -> Result<(), RpcError> {
        // 检查 Bridge 是否存在
        if !self.network.bridge_exists(bridge_name).await {
            info!("网络 Bridge '{}' 不存在，开始自动创建", bridge_name);
            
            // 从 bridge_name 推断 VLAN ID（格式：br-vlan100）
            let vlan_id = if bridge_name.starts_with("br-vlan") {
                bridge_name.strip_prefix("br-vlan")
                    .and_then(|s| s.parse::<u32>().ok())
            } else {
                None
            };
            
            if let Some(vlan) = vlan_id {
                // 自动创建 VLAN 网络（包括 Bridge 和 VLAN 子接口）
                if let Err(e) = self.network.create_network(
                    network_id,
                    &format!("auto-created-{}", network_id),
                    "bridge",
                    bridge_name,
                    Some(vlan),
                ).await {
                    error!("自动创建 VLAN 网络失败: {}", e);
                    return Err(RpcError::new(
                        RpcErrorCode::NetworkError,
                        format!("自动创建 VLAN 网络失败: {}", e),
                    ));
                }
                info!("成功自动创建 VLAN 网络: network_id={}, bridge={}, vlan={}", network_id, bridge_name, vlan);
            } else {
                // 自动创建无 VLAN 网络（直接使用 Provider 接口）
                if let Err(e) = self.network.create_network(
                    network_id,
                    &format!("auto-created-{}", network_id),
                    "bridge",
                    bridge_name,
                    None,
                ).await {
                    error!("自动创建无 VLAN 网络失败: {}", e);
                    return Err(RpcError::new(
                        RpcErrorCode::NetworkError,
                        format!("自动创建无 VLAN 网络失败: {}", e),
                    ));
                }
                info!("成功自动创建无 VLAN 网络: network_id={}, bridge={}", network_id, bridge_name);
            }
        }
        
        // 检查 Bridge 是否启动并可用
        if !self.network.is_bridge_up(bridge_name).await {
            return Err(RpcError::new(
                RpcErrorCode::NetworkError,
                format!("网络 Bridge '{}' 未启动或不可用，请检查网络配置", bridge_name),
            ));
        }
        
        info!("网络配置完成: network_id={}, bridge={}", network_id, bridge_name);
        Ok(())
    }

    /// 处理挂载存储卷请求
    async fn handle_attach_volume(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let request: AttachVolumeRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::new(
                RpcErrorCode::InvalidRequest,
                format!("解析请求参数失败: {}", e),
            ))?;

        info!("🔗 挂载存储卷到虚拟机: vm_id={}, volume_id={}", request.vm_id, request.volume_id);

        // 检查虚拟机是否存在
        if !self.hypervisor.vm_exists(&request.vm_id).await
            .map_err(|e| RpcError::new(RpcErrorCode::VmOperationFailed, format!("检查虚拟机失败: {}", e)))? {
            return Err(RpcError::new(
                RpcErrorCode::VmNotFound,
                format!("虚拟机不存在: {}", request.vm_id),
            ));
        }

        // 调用虚拟化管理器挂载存储卷
        match self.hypervisor.attach_volume(
            &request.vm_id,
            &request.volume_id,
            &request.volume_path,
            request.bus_type,
            request.device_type,
            &request.format,
        ).await {
            Ok(device) => {
                info!("✅ 存储卷挂载成功: vm_id={}, volume_id={}, device={}", 
                      request.vm_id, request.volume_id, device);
                
                let response = AttachVolumeResponse {
                    success: true,
                    message: "存储卷挂载成功".to_string(),
                    device: Some(device),
                };
                Ok(serde_json::to_value(response)
                    .map_err(|e| RpcError::new(RpcErrorCode::InternalError, format!("序列化响应失败: {}", e)))?)
            }
            Err(e) => {
                error!("❌ 存储卷挂载失败: vm_id={}, volume_id={}, error={}", 
                       request.vm_id, request.volume_id, e);
                Err(RpcError::new(
                    RpcErrorCode::VmOperationFailed,
                    format!("存储卷挂载失败: {}", e),
                ))
            }
        }
    }

    /// 处理分离存储卷请求
    async fn handle_detach_volume(&self, payload: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let request: DetachVolumeRequest = serde_json::from_value(payload)
            .map_err(|e| RpcError::new(
                RpcErrorCode::InvalidRequest,
                format!("解析请求参数失败: {}", e),
            ))?;

        info!("🔌 从虚拟机分离存储卷: vm_id={}, volume_id={}", 
              request.vm_id, request.volume_id);

        // 检查虚拟机是否存在
        if !self.hypervisor.vm_exists(&request.vm_id).await
            .map_err(|e| RpcError::new(RpcErrorCode::VmOperationFailed, format!("检查虚拟机失败: {}", e)))? {
            return Err(RpcError::new(
                RpcErrorCode::VmNotFound,
                format!("虚拟机不存在: {}", request.vm_id),
            ));
        }

        // 调用虚拟化管理器分离存储卷
        match self.hypervisor.detach_volume(
            &request.vm_id,
            &request.volume_id,
        ).await {
            Ok(_) => {
                info!("✅ 存储卷分离成功: vm_id={}, volume_id={}", 
                      request.vm_id, request.volume_id);
                
                let response = DetachVolumeResponse {
                    success: true,
                    message: "存储卷分离成功".to_string(),
                };
                Ok(serde_json::to_value(response)
                    .map_err(|e| RpcError::new(RpcErrorCode::InternalError, format!("序列化响应失败: {}", e)))?)
            }
            Err(e) => {
                error!("❌ 存储卷分离失败: vm_id={}, volume_id={}, error={}", 
                       request.vm_id, request.volume_id, e);
                Err(RpcError::new(
                    RpcErrorCode::VmOperationFailed,
                    format!("存储卷分离失败: {}", e),
                ))
            }
        }
    }

    /// 处理异步挂载存储卷（内部方法，用于通知处理）
    async fn handle_attach_volume_async_internal(&self, payload: serde_json::Value) -> Result<(), RpcError> {
        let req: serde_json::Value = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        let vm_id = req.get("vm_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 vm_id 参数".to_string()))?;

        let volume_id = req.get("volume_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 volume_id 参数".to_string()))?;

        let volume_path = req.get("volume_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 volume_path 参数".to_string()))?;

        let bus_type = req.get("bus_type")
            .and_then(|v| v.as_str())
            .unwrap_or("virtio");

        let device_type = req.get("device_type")
            .and_then(|v| v.as_str())
            .unwrap_or("disk");

        let format = req.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("qcow2");

        info!("异步挂载存储卷: vm_id={}, volume_id={}", vm_id, volume_id);

        // 异步执行挂载操作，不等待结果
        let hypervisor = self.hypervisor.clone();
        let vm_id = vm_id.to_string();
        let volume_id = volume_id.to_string();
        let volume_path = volume_path.to_string();
        let bus_type = bus_type.to_string();
        let device_type = device_type.to_string();
        let format = format.to_string();
        let notification_sender = self.notification_sender.clone();
        
        tokio::spawn(async move {
            // 转换字符串为枚举类型
            let bus_type_enum = match bus_type.as_str() {
                "virtio" => DiskBusType::Virtio,
                "scsi" => DiskBusType::Scsi,
                "ide" => DiskBusType::Ide,
                _ => DiskBusType::Virtio,
            };

            let device_type_enum = match device_type.as_str() {
                "disk" => DiskDeviceType::Disk,
                "cdrom" => DiskDeviceType::Cdrom,
                _ => DiskDeviceType::Disk,
            };

            match hypervisor.attach_volume(
                &vm_id,
                &volume_id,
                &volume_path,
                bus_type_enum,
                device_type_enum,
                &format,
            ).await {
                Ok(_) => {
                    info!("虚拟机 {} 存储卷 {} 异步挂载成功", vm_id, volume_id);
                    
                    // 发送成功通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "attach_volume",
                                "success": true,
                                "message": "存储卷挂载成功"
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送完成通知失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("虚拟机 {} 存储卷 {} 异步挂载失败: {}", vm_id, volume_id, e);
                    
                    // 发送失败通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "attach_volume",
                                "success": false,
                                "message": format!("存储卷挂载失败: {}", e)
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送失败通知失败: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// 处理异步分离存储卷（内部方法，用于通知处理）
    async fn handle_detach_volume_async_internal(&self, payload: serde_json::Value) -> Result<(), RpcError> {
        let req: serde_json::Value = serde_json::from_value(payload)
            .map_err(|e| RpcError::invalid_params(format!("参数错误: {}", e)))?;

        let vm_id = req.get("vm_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 vm_id 参数".to_string()))?;

        let volume_id = req.get("volume_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("缺少 volume_id 参数".to_string()))?;

        info!("异步分离存储卷: vm_id={}, volume_id={}", vm_id, volume_id);

        // 异步执行分离操作，不等待结果
        let hypervisor = self.hypervisor.clone();
        let vm_id = vm_id.to_string();
        let volume_id = volume_id.to_string();
        let notification_sender = self.notification_sender.clone();
        
        tokio::spawn(async move {
            match hypervisor.detach_volume(&vm_id, &volume_id).await {
                Ok(_) => {
                    info!("虚拟机 {} 存储卷 {} 异步分离成功", vm_id, volume_id);
                    
                    // 发送成功通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "detach_volume",
                                "success": true,
                                "message": "存储卷分离成功"
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送完成通知失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("虚拟机 {} 存储卷 {} 异步分离失败: {}", vm_id, volume_id, e);
                    
                    // 发送失败通知到 Server
                    if let Some(sender) = notification_sender {
                        let notification = RpcMessage::notification(
                            "vm_operation_completed",
                            serde_json::json!({
                                "vm_id": vm_id,
                                "operation": "detach_volume",
                                "success": false,
                                "message": format!("存储卷分离失败: {}", e)
                            }),
                        );
                        if let Err(e) = sender.send(notification) {
                            error!("发送失败通知失败: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }
}


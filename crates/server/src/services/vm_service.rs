/// 虚拟机管理服务

use chrono::Utc;
use uuid::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};

use crate::db::models::vm::{
    CreateVmDto, UpdateVmDto, VmListResponse, VmResponse, VmStatus, DiskSpec, NetworkInterfaceSpec,
    AttachVolumeDto, DetachVolumeDto, VmDiskResponse,
    Entity as VmEntity, Column as VmColumn, ActiveModel as VmActiveModel,
};
use crate::db::models::volume::{Entity as VolumeEntity, Column as VolumeColumn, ActiveModel as VolumeActiveModel};
use crate::db::models::network::{Entity as NetworkEntity};
use crate::db::models::task::{Entity as TaskEntity, ActiveModel as TaskActiveModel};
use crate::db::models::node::{Entity as NodeEntity, Column as NodeColumn};
use crate::app_state::AppState;
use crate::services::network_service::NetworkService;
use crate::ws::FrontendMessage;
use common::ws_rpc::{
    VmOperationRequest, CreateVmRequest, DiskSpec as ProtoDiskSpec, 
    NetworkInterfaceSpec as ProtoNetworkInterfaceSpec, CreateVmResponse, VmOperationResponse
};
use tracing::{info, warn, error};
use std::time::Duration;

pub struct VmService {
    state: AppState,
}

impl VmService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 获取节点名称
    async fn get_node_name(&self, node_id: &str) -> Option<String> {
        if let Ok(Some(node)) = NodeEntity::find_by_id(node_id.to_string())
            .one(&self.state.sea_db())
            .await
        {
            Some(node.hostname)
        } else {
            None
        }
    }

    /// 将 VM 转换为 VmResponse，包含节点名称
    async fn vm_to_response(&self, vm: crate::db::models::vm::Vm) -> VmResponse {
        let mut response = VmResponse::from(vm.clone());
        
        // 获取节点名称
        if let Some(node_id) = &vm.node_id {
            response.node_name = self.get_node_name(node_id).await;
        }
        
        response
    }

    /// 发送 VM 状态更新通知给前端
    async fn notify_vm_status_update(&self, vm_id: &str, status: &str, message: Option<&str>) {
        let frontend_msg = FrontendMessage::VmStatusUpdate {
            vm_id: vm_id.to_string(),
            status: status.to_string(),
            message: message.map(|s| s.to_string()),
        };

        let count = self.state.frontend_manager().broadcast(frontend_msg).await;
        if count > 0 {
            info!("已向 {} 个前端连接发送 VM {} 状态更新: {}", count, vm_id, status);
        }
    }

    /// 创建虚拟机
    pub async fn create_vm(&self, dto: CreateVmDto) -> anyhow::Result<VmResponse> {
        let db = &self.state.sea_db();
        
        // 生成 VM ID
        let vm_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // 验证volumes存在并且可用
        if let Some(ref disks) = dto.disks {
            for disk in disks {
                let volume = VolumeEntity::find_by_id(&disk.volume_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("存储卷 {} 不存在", disk.volume_id))?;
                
                if volume.status != "available" {
                    return Err(anyhow::anyhow!("存储卷 {} 状态不可用: {}", disk.volume_id, volume.status));
                }
                
                if volume.vm_id.is_some() {
                    return Err(anyhow::anyhow!("存储卷 {} 已被其他虚拟机使用", disk.volume_id));
                }
            }
        }

        // 验证网络并分配 IP 地址
        let mut network_interfaces_with_ip = Vec::new();
        let mut ip_allocations = Vec::new(); // 保存 IP 分配记录信息
        if let Some(ref networks) = dto.networks {
            let network_service = NetworkService::new(self.state.clone());
            
            for network_spec in networks {
                // 验证网络是否存在
                let network = NetworkEntity::find_by_id(&network_spec.network_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("网络 {} 不存在", network_spec.network_id))?;
                
                // 为 VM 预留 IP（不设置 vm_id）
                let ip_allocation = network_service
                    .allocate_ip(&network_spec.network_id)
                    .await?;
                
                info!("为 VM {} 在网络 {} 预留 IP: {}", vm_id, network.name, ip_allocation.ip_address);
                
                // 生成 MAC 地址（如果未提供）
                let mac_address = network_spec.mac_address.clone()
                    .unwrap_or_else(|| Self::generate_mac_address());
                
                // 创建带 IP 的网络接口配置
                let network_with_ip = NetworkInterfaceSpec {
                    network_id: network_spec.network_id.clone(),
                    mac_address: Some(mac_address.clone()),
                    ip_address: Some(ip_allocation.ip_address.clone()),
                    model: network_spec.model.clone(),
                    bridge_name: Some(format!("br-vlan{}", network.vlan_id.unwrap_or(0))),
                };
                
                network_interfaces_with_ip.push(network_with_ip);
                
                // 更新 IP 分配记录，添加 MAC 地址
                use crate::db::models::ip_allocation::{Entity as IpAllocationEntity, ActiveModel as IpAllocationActiveModel};
                let ip_record = IpAllocationEntity::find_by_id(&ip_allocation.id)
                    .one(db)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("IP 分配记录不存在"))?;
                
                let mut ip_active: IpAllocationActiveModel = ip_record.into();
                ip_active.mac_address = Set(Some(mac_address));
                ip_active.update(db).await?;
                
                // 保存 IP 分配记录信息，用于后续更新 vm_id
                ip_allocations.push(ip_allocation);
            }
        }

        // 序列化 disks 和 networks（使用分配了 IP 的网络配置）
        let disk_ids_json = dto
            .disks
            .as_ref()
            .map(|disks| serde_json::to_value(disks).ok())
            .flatten();
        
        let network_interfaces_json = if !network_interfaces_with_ip.is_empty() {
            serde_json::to_value(&network_interfaces_with_ip).ok()
        } else {
            None
        };

        // 创建 ActiveModel
        let vm_active = VmActiveModel {
            id: Set(vm_id.clone()),
            name: Set(dto.name.clone()),
            node_id: Set(Some(dto.node_id.clone())),
            status: Set(VmStatus::Stopped.as_str().to_string()),
            vcpu: Set(dto.vcpu as i32),
            memory_mb: Set(dto.memory_mb as i64),
            disk_ids: Set(disk_ids_json),
            network_interfaces: Set(network_interfaces_json),
            metadata: Set(dto.metadata.clone()),
            uuid: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            started_at: Set(None),
            stopped_at: Set(None),
        };

        // 插入数据库
        let mut vm = vm_active.insert(db).await?;

        // 更新volumes的vm_id
        if let Some(ref disks) = dto.disks {
            for disk in disks {
                let volume = VolumeEntity::find_by_id(&disk.volume_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("存储卷 {} 不存在", disk.volume_id))?;
                
                let mut volume_active: VolumeActiveModel = volume.into();
                volume_active.vm_id = Set(Some(vm_id.clone()));
                volume_active.status = Set("in-use".to_string());
                volume_active.updated_at = Set(now.into());
                volume_active.update(db).await?;
            }
        }
        
        // VM 数据库记录创建完成后，更新 IP 分配的 vm_id
        let network_service = NetworkService::new(self.state.clone());
        for ip_allocation in ip_allocations {
            if let Err(e) = network_service.update_ip_vm_id(&ip_allocation.id, &vm_id).await {
                error!("更新 IP 分配 vm_id 失败: {}", e);
                // 如果更新失败，释放预留的 IP
                if let Err(release_err) = network_service.release_ip(&ip_allocation.network_id, &vm_id).await {
                    error!("释放预留 IP 失败: {}", release_err);
                }
            } else {
                info!("成功更新 IP {} 的 vm_id 为 {}", ip_allocation.ip_address, vm_id);
            }
        }

        // 调用Agent创建虚拟机
        if let Some(ref disks) = dto.disks {
            // 使用 WebSocket RPC 调用 Agent 创建虚拟机
            // 转换disks为proto格式，需要查询volume的实际路径
            let mut proto_disks = Vec::new();
            for disk in disks.iter() {
                // 查询volume的实际路径
                let volume = VolumeEntity::find_by_id(&disk.volume_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("存储卷 {} 不存在", disk.volume_id))?;
                
                let volume_path = volume.path.ok_or_else(|| {
                    anyhow::anyhow!("存储卷 {} 没有路径信息", disk.volume_id)
                })?;

                proto_disks.push(ProtoDiskSpec {
                    volume_id: disk.volume_id.clone(),  // 保持volume_id用于标识
                    device: disk.device.clone(),
                    bootable: disk.bootable,
                    volume_path,  // 使用专门的字段传递路径
                });
            }

            // 转换networks为proto格式（使用分配了 IP 的网络配置）
            let proto_networks: Vec<ProtoNetworkInterfaceSpec> = network_interfaces_with_ip
                .iter()
                .map(|net| {
                    ProtoNetworkInterfaceSpec {
                        network_id: net.network_id.clone(),
                        mac_address: net.mac_address.clone().unwrap_or_default(),
                        ip_address: net.ip_address.clone().unwrap_or_default(),
                        model: net.model.clone(),
                        bridge_name: net.bridge_name.clone().unwrap_or_default(),
                    }
                })
                .collect();

            let request = CreateVmRequest {
                vm_id: vm_id.clone(),
                name: dto.name.clone(),
                vcpu: dto.vcpu,
                memory_mb: dto.memory_mb,
                disks: proto_disks,
                networks: proto_networks,
                metadata: dto.metadata
                    .as_ref()
                    .and_then(|m| m.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default(),
            };

            // 通过 WebSocket RPC 调用 Agent
            let node_id = vm.node_id.as_ref()
                .ok_or_else(|| anyhow::anyhow!("虚拟机未关联节点"))?;
            
            let response_msg = self.state.agent_manager()
                .call(
                    node_id,
                    "create_vm",
                    serde_json::to_value(&request)?,
                    Duration::from_secs(120)  // VM 创建可能需要较长时间
                )
                .await
                .map_err(|e| anyhow::anyhow!("WebSocket RPC 调用失败: {}", e))?;

            let result: CreateVmResponse = serde_json::from_value(
                response_msg.payload.ok_or_else(|| anyhow::anyhow!("响应无数据"))?
            )?;

            if !result.success {
                return Err(anyhow::anyhow!("Agent创建虚拟机失败: {}", result.message));
            }
        }

        Ok(self.vm_to_response(vm).await)
    }

    /// 获取虚拟机列表
    pub async fn list_vms(
        &self,
        page: usize,
        page_size: usize,
        node_id: Option<String>,
        status: Option<String>,
    ) -> anyhow::Result<VmListResponse> {
        let db = &self.state.sea_db();

        // 构建查询条件
        let mut query = VmEntity::find();

        if let Some(nid) = node_id {
            query = query.filter(VmColumn::NodeId.eq(nid));
        }

        if let Some(s) = status {
            query = query.filter(VmColumn::Status.eq(s));
        }

        // 获取总数
        let total = query.clone().count(db).await? as usize;

        // 执行分页查询
        let vms = query
            .order_by_desc(VmColumn::CreatedAt)
            .offset(((page - 1) * page_size) as u64)
            .limit(page_size as u64)
            .all(db)
            .await?;

        let vm_responses: Vec<VmResponse> = {
            let mut responses = Vec::new();
            for vm in vms {
                responses.push(self.vm_to_response(vm).await);
            }
            responses
        };

        Ok(VmListResponse {
            vms: vm_responses,
            total,
            page,
            page_size,
        })
    }

    /// 获取单个虚拟机详情
    pub async fn get_vm(&self, id: &str) -> anyhow::Result<Option<VmResponse>> {
        let vm = VmEntity::find_by_id(id.to_string())
            .one(&self.state.sea_db())
            .await?;

        Ok(if let Some(vm) = vm {
            Some(self.vm_to_response(vm).await)
        } else {
            None
        })
    }

    /// 更新虚拟机
    pub async fn update_vm(&self, id: &str, dto: UpdateVmDto) -> anyhow::Result<VmResponse> {
        let db = &self.state.sea_db();
        let now = Utc::now();

        // 先查询现有的虚拟机
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        // 转换为 ActiveModel
        let mut vm_active: VmActiveModel = vm.into();

        // 更新字段
        if let Some(name) = dto.name {
            vm_active.name = Set(name);
        }
        if let Some(vcpu) = dto.vcpu {
            vm_active.vcpu = Set(vcpu as i32);
        }
        if let Some(memory_mb) = dto.memory_mb {
            vm_active.memory_mb = Set(memory_mb as i64);
        }
        if let Some(disks) = dto.disks {
            let disks_json = serde_json::to_value(disks)?;
            vm_active.disk_ids = Set(Some(disks_json));
        }
        if let Some(networks) = dto.networks {
            let networks_json = serde_json::to_value(networks)?;
            vm_active.network_interfaces = Set(Some(networks_json));
        }
        if let Some(metadata) = dto.metadata {
            vm_active.metadata = Set(Some(metadata));
        }

        vm_active.updated_at = Set(now.into());

        // 执行更新
        let vm = vm_active.update(db).await?;

        Ok(self.vm_to_response(vm).await)
    }

    /// 删除虚拟机
    pub async fn delete_vm(&self, id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 先查询 VM 信息
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?;
        
        // 释放 VM 的所有 IP 地址
        if let Some(ref vm_record) = vm {
            let network_service = NetworkService::new(self.state.clone());
            
            // 从网络接口配置中获取所有网络 ID
            if let Some(ref network_interfaces) = vm_record.network_interfaces {
                if let Ok(interfaces) = serde_json::from_value::<Vec<NetworkInterfaceSpec>>(network_interfaces.clone()) {
                    for interface in interfaces {
                        if let Err(e) = network_service.release_ip(&interface.network_id, id).await {
                            warn!("释放 VM {} 在网络 {} 的 IP 失败: {}", id, interface.network_id, e);
                        } else {
                            info!("成功释放 VM {} 在网络 {} 的 IP", id, interface.network_id);
                        }
                    }
                }
            }
        }

        if let Some(vm) = vm {
            // 如果 VM 正在运行，先停止
            if vm.status == VmStatus::Running.as_str() {
                return Err(anyhow::anyhow!("无法删除正在运行的虚拟机，请先停止"));
            }

            // 调用 Agent 删除虚拟机
            if let Some(node_id) = &vm.node_id {
                // 使用 WebSocket RPC 调用 Agent 删除虚拟机
                let request = VmOperationRequest {
                    vm_id: id.to_string(),
                    force: false,
                };
                
                // 尝试调用 Agent，但即使失败也继续删除数据库记录
                match self.state.agent_manager().call(
                    &node_id,
                    "delete_vm",
                    serde_json::to_value(&request).unwrap_or_default(),
                    Duration::from_secs(60)
                ).await {
                    Ok(_) => info!("成功通知 Agent 删除虚拟机: {}", id),
                    Err(e) => warn!("通知 Agent 删除虚拟机失败（将继续删除数据库记录）: {}", e),
                }
            }

            // 清理关联的volumes - 将vm_id设置为null，状态改为available
            let now = Utc::now();
            let volumes = VolumeEntity::find()
                .filter(VolumeColumn::VmId.eq(id))
                .all(db)
                .await?;

            for volume in volumes {
                let mut volume_active: VolumeActiveModel = volume.into();
                volume_active.vm_id = Set(None);
                volume_active.status = Set("available".to_string());
                volume_active.updated_at = Set(now.into());
                volume_active.update(db).await?;
            }

            // 从数据库删除
            VmEntity::delete_by_id(id.to_string())
                .exec(db)
                .await?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("虚拟机不存在"))
        }
    }

    /// 启动虚拟机
    pub async fn start_vm(&self, id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询 VM 信息
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        if vm.status == VmStatus::Running.as_str() {
            return Err(anyhow::anyhow!("虚拟机已经在运行中"));
        }

        let node_id = vm
            .node_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未分配节点"))?;


        // 使用 WebSocket RPC 调用 Agent 启动虚拟机
        let request = VmOperationRequest {
            vm_id: id.to_string(),
            force: false,
        };

        let node_id = vm.node_id.as_ref()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未关联节点"))?;
        
        let response_msg = self.state.agent_manager()
            .call(
                node_id,
                "start_vm",
                serde_json::to_value(&request)?,
                Duration::from_secs(60)
            )
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket RPC 调用失败: {}", e))?;

        let result: VmOperationResponse = serde_json::from_value(
            response_msg.payload.ok_or_else(|| anyhow::anyhow!("响应无数据"))?
        )?;

        if !result.success {
            return Err(anyhow::anyhow!("启动虚拟机失败: {}", result.message));
        }

        // 更新数据库状态
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set(VmStatus::Running.as_str().to_string());
        vm_active.started_at = Set(Some(now.into()));
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 发送状态更新通知给前端
        self.notify_vm_status_update(id, "running", Some("虚拟机启动成功")).await;

        Ok(())
    }

    /// 停止虚拟机
    pub async fn stop_vm(&self, id: &str, force: bool) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询 VM 信息
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        if vm.status == VmStatus::Stopped.as_str() {
            return Err(anyhow::anyhow!("虚拟机已经停止"));
        }

        let node_id = vm
            .node_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未分配节点"))?;

        // 创建异步任务
        let task_id = uuid::Uuid::new_v4().to_string();
        let task_payload = serde_json::json!({
            "vm_id": id,
            "force": force,
            "operation": "stop_vm"
        });

        // 在数据库中创建任务记录
        let task = TaskActiveModel {
            id: Set(task_id.clone()),
            task_type: Set("stop_vm".to_string()),
            status: Set("pending".to_string()),
            progress: Set(0),
            payload: Set(task_payload.clone()),
            target_type: Set(Some("vm".to_string())),
            target_id: Set(Some(id.to_string())),
            node_id: Set(vm.node_id.clone()),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };
        task.insert(db).await.map_err(|e| anyhow::anyhow!("创建任务记录失败: {}", e))?;
        info!("debug: 创建任务记录成功: {}", task_id);

        info!("debug: 发送异步通知给 Agent，包含 task_id");
        // 发送异步通知给 Agent，包含 task_id
        let node_id = vm.node_id.as_ref()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未关联节点"))?;
        
        info!("debug: 获取节点 ID: {}", node_id);
        
        let request = serde_json::json!({
            "vm_id": id,
            "force": force,
            "task_id": task_id
        });
        info!("debug: 发送异步通知给 Agent，生成请求: {}", request);

        self.state.agent_manager()
            .notify(
                node_id,
                "stop_vm_async",
                request,
            )
            .await
            .map_err(|e| anyhow::anyhow!("发送异步通知失败: {}", e))?;

        // 立即更新 VM 状态为 "stopping"
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set("stopping".to_string());
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 发送状态更新通知给前端
        self.notify_vm_status_update(id, "stopping", Some("虚拟机停止中")).await;

        Ok(())
    }

    /// 重启虚拟机
    pub async fn restart_vm(&self, id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询 VM 信息
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        let node_id = vm
            .node_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未分配节点"))?;

        // 使用 WebSocket RPC 调用 Agent 重启虚拟机
        let request = VmOperationRequest {
            vm_id: id.to_string(),
            force: false,
        };

        let node_id = vm.node_id.as_ref()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未关联节点"))?;
        
        let response_msg = self.state.agent_manager()
            .call(
                node_id,
                "restart_vm",
                serde_json::to_value(&request)?,
                Duration::from_secs(60)
            )
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket RPC 调用失败: {}", e))?;

        let result: VmOperationResponse = serde_json::from_value(
            response_msg.payload.ok_or_else(|| anyhow::anyhow!("响应无数据"))?
        )?;

        if !result.success {
            return Err(anyhow::anyhow!("重启虚拟机失败: {}", result.message));
        }

        // 更新数据库状态
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.started_at = Set(Some(now.into()));
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        Ok(())
    }

    /// 迁移虚拟机
    pub async fn migrate_vm(
        &self,
        id: &str,
        target_node_id: &str,
        live: bool,
    ) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询 VM 信息
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        let source_node_id = vm
            .node_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未分配节点"))?;

        if source_node_id == target_node_id {
            return Err(anyhow::anyhow!("源节点和目标节点相同"));
        }

        // 更新状态为迁移中
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set(VmStatus::Migrating.as_str().to_string());
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // TODO: 实际的迁移逻辑应该异步执行
        // 这里简化处理，直接更新 node_id
        
        // 更新 VM 的节点 ID 和状态
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;
        
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.node_id = Set(Some(target_node_id.to_string()));
        vm_active.status = Set(if live { 
            VmStatus::Running.as_str() 
        } else { 
            VmStatus::Stopped.as_str() 
        }.to_string());
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        Ok(())
    }

    /// 获取指定节点上的虚拟机列表
    pub async fn list_vms_by_node(&self, node_id: &str) -> anyhow::Result<Vec<VmResponse>> {
        let vms = VmEntity::find()
            .filter(VmColumn::NodeId.eq(node_id))
            .order_by_desc(VmColumn::CreatedAt)
            .all(&self.state.sea_db())
            .await?;

        let mut responses = Vec::new();
        for vm in vms {
            responses.push(self.vm_to_response(vm).await);
        }
        Ok(responses)
    }

    /// 更新虚拟机状态
    pub async fn update_vm_status(&self, id: &str, status: VmStatus) -> anyhow::Result<()> {
        let db = &self.state.sea_db();
        let now = Utc::now();
        
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set(status.as_str().to_string());
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        Ok(())
    }

    /// 附加存储卷到虚拟机
    pub async fn attach_volume(&self, vm_id: &str, dto: AttachVolumeDto) -> anyhow::Result<()> {
        let db = &self.state.sea_db();
        let now = Utc::now();

        // 检查虚拟机是否存在
        let vm = VmEntity::find_by_id(vm_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        // 检查存储卷是否存在且可用
        let volume = VolumeEntity::find_by_id(&dto.volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储卷不存在"))?;

        if volume.status != "available" {
            return Err(anyhow::anyhow!("存储卷状态不可用: {}", volume.status));
        }

        if volume.vm_id.is_some() {
            return Err(anyhow::anyhow!("存储卷已被其他虚拟机使用"));
        }

        // 获取当前的磁盘列表
        let mut disks: Vec<DiskSpec> = vm.disk_ids
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // 检查设备名是否已存在
        if disks.iter().any(|d| d.device == dto.device) {
            return Err(anyhow::anyhow!("设备名 {} 已被使用", dto.device));
        }

        // 添加新磁盘
        disks.push(DiskSpec {
            volume_id: dto.volume_id.clone(),
            device: dto.device.clone(),
            bootable: dto.bootable.unwrap_or(false),
        });

        // 更新虚拟机的磁盘列表
        let disks_json = serde_json::to_value(&disks)?;
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.disk_ids = Set(Some(disks_json));
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 更新存储卷的vm_id
        let mut volume_active: VolumeActiveModel = volume.into();
        volume_active.vm_id = Set(Some(vm_id.to_string()));
        volume_active.status = Set("in-use".to_string());
        volume_active.updated_at = Set(now.into());
        volume_active.update(db).await?;

        Ok(())
    }

    /// 从虚拟机分离存储卷
    pub async fn detach_volume(&self, vm_id: &str, dto: DetachVolumeDto) -> anyhow::Result<()> {
        let db = &self.state.sea_db();
        let now = Utc::now();

        // 检查虚拟机是否存在
        let vm = VmEntity::find_by_id(vm_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        // 检查VM是否在运行
        if vm.status == VmStatus::Running.as_str() {
            return Err(anyhow::anyhow!("无法从运行中的虚拟机分离存储卷，请先停止虚拟机"));
        }

        // 获取当前的磁盘列表
        let mut disks: Vec<DiskSpec> = vm.disk_ids
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // 查找并移除指定的磁盘
        let original_len = disks.len();
        disks.retain(|d| d.volume_id != dto.volume_id);

        if disks.len() == original_len {
            return Err(anyhow::anyhow!("存储卷未附加到此虚拟机"));
        }

        // 更新虚拟机的磁盘列表
        let disks_json = if disks.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::to_value(&disks)?
        };
        
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.disk_ids = Set(Some(disks_json));
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 更新存储卷状态
        if let Some(volume) = VolumeEntity::find_by_id(&dto.volume_id).one(db).await? {
            let mut volume_active: VolumeActiveModel = volume.into();
            volume_active.vm_id = Set(None);
            volume_active.status = Set("available".to_string());
            volume_active.updated_at = Set(now.into());
            volume_active.update(db).await?;
        }

        Ok(())
    }

    /// 获取虚拟机的所有存储卷
    pub async fn list_vm_volumes(&self, vm_id: &str) -> anyhow::Result<Vec<VmDiskResponse>> {
        let db = &self.state.sea_db();

        // 检查虚拟机是否存在
        let vm = VmEntity::find_by_id(vm_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        // 获取磁盘列表
        let disks: Vec<DiskSpec> = vm.disk_ids
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let mut result = Vec::new();

        for disk in disks {
            // 查询volume详细信息
            if let Some(volume) = VolumeEntity::find_by_id(&disk.volume_id).one(db).await? {
                result.push(VmDiskResponse {
                    volume_id: disk.volume_id.clone(),
                    device: disk.device.clone(),
                    bootable: disk.bootable,
                    volume_name: Some(volume.name),
                    size_gb: Some(volume.size_gb),
                    volume_type: Some(volume.volume_type),
                    path: volume.path,
                });
            } else {
                // Volume不存在，返回基本信息
                result.push(VmDiskResponse {
                    volume_id: disk.volume_id.clone(),
                    device: disk.device.clone(),
                    bootable: disk.bootable,
                    volume_name: None,
                    size_gb: None,
                    volume_type: None,
                    path: None,
                });
            }
        }

        Ok(result)
    }

    /// 获取虚拟机的网络信息
    pub async fn list_vm_networks(&self, vm_id: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let db = &self.state.sea_db();

        // 检查虚拟机是否存在
        let vm = VmEntity::find_by_id(vm_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        // 获取网络接口列表
        let network_interfaces: Vec<NetworkInterfaceSpec> = vm.network_interfaces
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let mut result = Vec::new();

        for interface in network_interfaces {
            // 查询网络详细信息
            if let Some(network) = NetworkEntity::find_by_id(&interface.network_id).one(db).await? {
                let network_info = serde_json::json!({
                    "network_id": interface.network_id,
                    "network_name": network.name,
                    "ip_address": interface.ip_address,
                    "mac_address": interface.mac_address,
                    "model": interface.model,
                    "bridge_name": interface.bridge_name,
                    "network_type": network.network_type,
                    "cidr": network.cidr,
                    "vlan_id": network.vlan_id
                });
                result.push(network_info);
            } else {
                // 网络不存在，返回基本信息
                let network_info = serde_json::json!({
                    "network_id": interface.network_id,
                    "network_name": "未知网络",
                    "ip_address": interface.ip_address,
                    "mac_address": interface.mac_address,
                    "model": interface.model,
                    "bridge_name": interface.bridge_name,
                    "network_type": null,
                    "cidr": null,
                    "vlan_id": null
                });
                result.push(network_info);
            }
        }

        Ok(result)
    }

    /// 生成 MAC 地址
    /// 使用标准的 VM MAC 地址前缀 52:54:00（QEMU/KVM 使用的前缀）
    fn generate_mac_address() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!(
            "52:54:00:{:02x}:{:02x}:{:02x}",
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>()
        )
    }
}

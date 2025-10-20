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
use crate::db::models::node::{Entity as NodeEntity};
use crate::app_state::AppState;
use crate::services::network_service::NetworkService;
use crate::ws::FrontendMessage;
use tracing::{info, warn, error};

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
    /// 
    /// 按照 vms.md 流程：API -> Server保存数据到DB -> UI提示成功
    /// Server保存元数据，agent无需操作。
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
                    bridge_name: Some(match network.vlan_id {
                        Some(vlan_id) => format!("br-vlan{}", vlan_id),
                        None => "br-default".to_string(),
                    }),
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
        let volumes_json = dto
            .disks
            .as_ref()
            .map(|disks| serde_json::to_value(disks).ok())
            .flatten();
        
        let network_interfaces_json = if !network_interfaces_with_ip.is_empty() {
            serde_json::to_value(&network_interfaces_with_ip).ok()
        } else {
            None
        };

        // 确定操作系统类型，默认为 linux
        let os_type = dto.os_type.clone().unwrap_or_else(|| "linux".to_string());

        // 创建 ActiveModel
        let vm_active = VmActiveModel {
            id: Set(vm_id.clone()),
            name: Set(dto.name.clone()),
            node_id: Set(Some(dto.node_id.clone())),
            status: Set(VmStatus::Stopped.as_str().to_string()),
            vcpu: Set(dto.vcpu as i32),
            memory_mb: Set(dto.memory_mb as i64),
            os_type: Set(os_type),
            volumes: Set(volumes_json),
            network_interfaces: Set(network_interfaces_json),
            metadata: Set(dto.metadata.clone()),
            uuid: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            started_at: Set(None),
            stopped_at: Set(None),
        };

        // 插入数据库
        let vm = vm_active.insert(db).await?;

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

        // 按照 vms.md 流程：仅保存到数据库，不调用 agent
        info!("虚拟机 {} 创建成功，已保存到数据库", vm_id);

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
        if let Some(os_type) = dto.os_type {
            vm_active.os_type = Set(os_type);
        }
        if let Some(disks) = dto.disks {
            let volumes_json = serde_json::to_value(disks)?;
            vm_active.volumes = Set(Some(volumes_json));
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
    /// 
    /// 按照 vms.md 流程：
    /// API -> Server清理DB
    pub async fn delete_vm(&self, id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 先查询 VM 信息
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?;
        
        if let Some(vm) = vm {
            // 如果 VM 正在运行，先停止
            if vm.status == VmStatus::Running.as_str() {
                return Err(anyhow::anyhow!("无法删除正在运行的虚拟机，请先停止"));
            }

            // 释放 VM 的所有 IP 地址
            let network_service = NetworkService::new(self.state.clone());
            
            // 从网络接口配置中获取所有网络 ID
            if let Some(ref network_interfaces) = vm.network_interfaces {
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

            // 从数据库删除虚拟机记录
            VmEntity::delete_by_id(id.to_string())
                .exec(db)
                .await?;

            info!("虚拟机 {} 已从数据库删除", id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("虚拟机不存在"))
        }
    }

    /// 启动虚拟机
    /// 
    /// 按照 vms.md 流程：
    /// API -> Server记录DB -> UI提示进行中
    /// --(notify)-> agent 重新define xml，启动虚拟机 --(notify)-> Server更新db记录 -> UI提示完成
    /// Agent需要重新define xml，确保虚拟机配置与数据库一致。
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

        // 通知 Agent 所需的字段从 Model 读取，避免 ActiveValue 参与序列化
        let node_id = vm.node_id.clone().ok_or_else(|| anyhow::anyhow!("虚拟机未关联节点"))?;
        // 组装 Agent 所需的磁盘信息（DiskConfig）
        let mut vm_start_volumes = Vec::new();
        if let Some(ref volumes_json) = vm.volumes {
            if let Ok(volumes) = serde_json::from_value::<Vec<DiskSpec>>(volumes_json.clone()) {
                for v in volumes {
                    // 查询 volume 以获取路径与格式
                    let vol = VolumeEntity::find_by_id(&v.volume_id)
                        .one(db)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!(format!("存储卷不存在: {}", v.volume_id)))?;

                    let volume_path = vol.path.ok_or_else(|| anyhow::anyhow!(format!("存储卷缺少路径: {}", v.volume_id)))?;
                    let format = vol.volume_type;

                    let volume_value = serde_json::json!({
                        "volume_id": v.volume_id,
                        "volume_path": volume_path,
                        "bus_type": v.bus_type,
                        "device_type": v.device_type,
                        "format": format
                    });
                    vm_start_volumes.push(volume_value);
                }
            }
        }

        let start_request = serde_json::json!({
            "vm_id": id,
            "name": vm.name,
            "vcpu": vm.vcpu,
            "memory_mb": vm.memory_mb,
            "os_type": vm.os_type,
            // 新字段：按 Agent 期望结构提供的磁盘数组
            "volumes": vm_start_volumes,
            // 先保持原有网络结构，后续再转换为 Agent 期望的 NetworkConfig
            "networks": vm.network_interfaces,
            "metadata": vm.metadata
        });

        // 更新数据库状态为"启动中"
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set("starting".to_string());
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 异步通知 Agent，不等待结果
        self.state.agent_manager()
            .notify(
                &node_id,
                "start_vm_async",
                start_request,
            )
            .await
            .map_err(|e| anyhow::anyhow!("发送启动通知失败: {}", e))?;

        info!("虚拟机 {} 启动通知已发送给 Agent", id);
        Ok(())
    }

    /// 停止虚拟机
    /// 
    /// 按照 vms.md 流程：
    /// API -> Server记录DB -> UI提示进行中
    /// --(notify)-> agent 关机并undefine xml --(notify)-> Server更新db记录 -> UI提示完成
    /// 关机需要区是否为强制关机模式。在非强制失败后，自用使用强制关机。
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

        // 通知 Agent 所需字段从 Model 读取
        let node_id = vm.node_id.clone().ok_or_else(|| anyhow::anyhow!("虚拟机未关联节点"))?;
        
        let stop_request = serde_json::json!({
            "vm_id": id,
            "force": force
        });

        // 异步通知 Agent，不等待结果
        self.state.agent_manager()
            .notify(
                &node_id,
                "stop_vm_async",
                stop_request,
            )
            .await
            .map_err(|e| anyhow::anyhow!("发送停止通知失败: {}", e))?;

        info!("虚拟机 {} 停止通知已发送给 Agent", id);

        // 更新数据库状态为"停止中"
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set("stopping".to_string());
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        Ok(())
    }

    /// 重启虚拟机（异步）
    /// 
    /// 按照 vms.md 流程：
    /// API -> Server记录DB -> UI提示进行中
    /// --(notify)-> agent 尝试软关机并启动，否则强制关机并启动 --(notify)-> Server更新db记录 -> UI提示完成
    pub async fn restart_vm(&self, id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询 VM 信息
        let vm = VmEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        // 通知 Agent 所需字段
        let node_id = vm
            .node_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("虚拟机未关联节点"))?;

        let request = serde_json::json!({
            "vm_id": id,
            // 先走软关机，失败由 Agent 端自动执行强制关机
            "force": false
        });

        // 更新数据库状态为"重启中"
        let now = Utc::now();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set("restarting".to_string());
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 异步通知 Agent，不等待结果
        self.state
            .agent_manager()
            .notify(&node_id, "restart_vm_async", request)
            .await
            .map_err(|e| anyhow::anyhow!("发送重启通知失败: {}", e))?;

        info!("虚拟机 {} 重启通知已发送给 Agent", id);
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

    

    /// 附加存储卷到虚拟机
    /// 
    /// 按照 vms.md 流程：
    /// API -> Server记录DB -> UI提示进行中
    /// --(notify)-> agent 热挂载磁盘，并标记持久 --(notify)-> Server更新db记录 -> UI提示完成
    /// 若虚拟机未开机，则不需要调用agent，仅更新db即可。开机会重新define。
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
        let mut disks: Vec<DiskSpec> = vm.volumes
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // 添加新磁盘
        disks.push(DiskSpec {
            volume_id: dto.volume_id.clone(),
            bus_type: dto.bus_type.clone().unwrap_or_default(),
            device_type: dto.device_type.clone().unwrap_or_default(),
        });

        // 更新虚拟机的磁盘列表
        let disks_json = serde_json::to_value(&disks)?;
        // 在转换前保留运行状态与 node_id 以便后续热挂载通知
        let vm_running = vm.status == VmStatus::Running.as_str();
        let vm_node_id = vm.node_id.clone();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.volumes = Set(Some(disks_json));
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 在转换前保留 volume 字段用于后续请求
        let volume_path = volume.path.clone();
        let volume_type = volume.volume_type.clone();
        // 更新存储卷的vm_id
        let mut volume_active: VolumeActiveModel = volume.into();
        volume_active.vm_id = Set(Some(vm_id.to_string()));
        volume_active.status = Set("in-use".to_string());
        volume_active.updated_at = Set(now.into());
        volume_active.update(db).await?;

        // 如果虚拟机正在运行，通知 Agent 热挂载
        if vm_running {
            if let Some(node_id) = &vm_node_id {
                let request = serde_json::json!({
                    "vm_id": vm_id,
                    "volume_id": dto.volume_id,
                    "volume_path": volume_path,
                    "bus_type": dto.bus_type.clone().unwrap_or_default(),
                    "device_type": dto.device_type.clone().unwrap_or_default(),
                    "format": volume_type
                });

                // 异步通知 Agent，不等待结果
                self.state.agent_manager()
                    .notify(
                        node_id,
                        "attach_volume_async",
                        request,
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("发送挂载通知失败: {}", e))?;

                info!("虚拟机 {} 存储卷挂载通知已发送给 Agent", vm_id);
            }
        } else {
            info!("虚拟机 {} 未运行，存储卷将在下次启动时自动挂载", vm_id);
        }

        Ok(())
    }

    /// 从虚拟机分离存储卷
    /// 
    /// 按照 vms.md 流程：
    /// API -> Server记录DB -> UI提示进行中
    /// --(notify)-> agent 热解除磁盘，并标记持久 --(notify)-> Server更新db记录 -> UI提示完成
    /// 若虚拟机未开机，则不需要调用agent，仅更新db即可。开机会重新define。
    pub async fn detach_volume(&self, vm_id: &str, dto: DetachVolumeDto) -> anyhow::Result<()> {
        let db = &self.state.sea_db();
        let now = Utc::now();

        // 检查虚拟机是否存在
        let vm = VmEntity::find_by_id(vm_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        // 获取当前的磁盘列表
        let mut disks: Vec<DiskSpec> = vm.volumes
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // 查找要分离的磁盘（允许不存在，实现最终一致性）
        let disk_exists = disks.iter()
            .any(|d| d.volume_id == dto.volume_id);
        
        if !disk_exists {
            tracing::warn!("⚠️ 存储卷未附加到此虚拟机，但继续执行分离操作以确保最终一致性: vm_id={}, volume_id={}", vm_id, dto.volume_id);
        }

        // 从磁盘列表中移除指定的磁盘
        disks.retain(|d| d.volume_id != dto.volume_id);

        // 更新虚拟机的磁盘列表
        let disks_json_opt = if disks.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&disks)?)
        };

        // 在转换前保留运行状态与 node_id 以便后续热分离通知
        let vm_running = vm.status == VmStatus::Running.as_str();
        let vm_node_id = vm.node_id.clone();
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.volumes = Set(disks_json_opt);
        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        // 更新存储卷状态（允许存储卷不存在，实现最终一致性）
        if let Some(volume) = VolumeEntity::find_by_id(&dto.volume_id).one(db).await? {
            let mut volume_active: VolumeActiveModel = volume.into();
            volume_active.vm_id = Set(None);
            volume_active.status = Set("available".to_string());
            volume_active.updated_at = Set(now.into());
            volume_active.update(db).await?;
            tracing::info!("✅ 存储卷状态已更新为可用: volume_id={}", dto.volume_id);
        } else {
            tracing::warn!("⚠️ 存储卷不存在，跳过状态更新: volume_id={}", dto.volume_id);
        }

        // 如果虚拟机正在运行，通知 Agent 热分离
        if vm_running {
            if let Some(node_id) = &vm_node_id {
                let request = serde_json::json!({
                    "vm_id": vm_id,
                    "volume_id": dto.volume_id
                });

                // 异步通知 Agent，不等待结果
                self.state.agent_manager()
                    .notify(
                        node_id,
                        "detach_volume_async",
                        request,
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("发送分离通知失败: {}", e))?;

                info!("虚拟机 {} 存储卷分离通知已发送给 Agent", vm_id);
            }
        } else {
            info!("虚拟机 {} 未运行，存储卷分离已完成", vm_id);
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
        let disks: Vec<DiskSpec> = vm.volumes
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let mut result = Vec::new();

        for (idx, disk) in disks.iter().enumerate() {
            // 自动生成设备名
            let device_name = match disk.device_type {
                common::ws_rpc::types::DiskDeviceType::Disk => {
                    format!("vd{}", (b'a' + idx as u8) as char)
                }
                common::ws_rpc::types::DiskDeviceType::Cdrom => {
                    format!("hd{}", (b'a' + idx as u8) as char)
                }
            };
            
            // 查询volume详细信息
            if let Some(volume) = VolumeEntity::find_by_id(&disk.volume_id).one(db).await? {
                result.push(VmDiskResponse {
                    volume_id: disk.volume_id.clone(),
                    device: device_name,
                    bootable: idx == 0, // 第一个磁盘默认为启动盘
                    bus_type: disk.bus_type.clone(),
                    device_type: disk.device_type.clone(),
                    volume_name: Some(volume.name),
                    size_gb: Some(volume.size_gb),
                    volume_type: Some(volume.volume_type),
                    path: volume.path,
                });
            } else {
                // Volume不存在，返回基本信息
                result.push(VmDiskResponse {
                    volume_id: disk.volume_id.clone(),
                    device: device_name,
                    bootable: idx == 0, // 第一个磁盘默认为启动盘
                    bus_type: disk.bus_type.clone(),
                    device_type: disk.device_type.clone(),
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

    /// 处理 Agent 的虚拟机操作完成通知
    pub async fn handle_vm_operation_completed(&self, vm_id: &str, operation: &str, success: bool, message: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();
        let now = Utc::now();

        // 查询 VM 信息
        let vm = VmEntity::find_by_id(vm_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在"))?;

        let mut vm_active: VmActiveModel = vm.into();

        match operation {
            "start_vm" => {
                if success {
                    vm_active.status = Set(VmStatus::Running.as_str().to_string());
                    vm_active.started_at = Set(Some(now.into()));
                    self.notify_vm_status_update(vm_id, "running", Some("虚拟机启动成功")).await;
                } else {
                    vm_active.status = Set(VmStatus::Stopped.as_str().to_string());
                    self.notify_vm_status_update(vm_id, "stopped", Some(&format!("虚拟机启动失败: {}", message))).await;
                }
            }
            "stop_vm" => {
                if success {
                    vm_active.status = Set(VmStatus::Stopped.as_str().to_string());
                    vm_active.stopped_at = Set(Some(now.into()));
                    self.notify_vm_status_update(vm_id, "stopped", Some("虚拟机停止成功")).await;
                } else {
                    // 停止失败，保持当前状态
                    self.notify_vm_status_update(vm_id, "error", Some(&format!("虚拟机停止失败: {}", message))).await;
                }
            }
            "restart_vm" => {
                if success {
                    vm_active.status = Set(VmStatus::Running.as_str().to_string());
                    vm_active.started_at = Set(Some(now.into()));
                    self.notify_vm_status_update(vm_id, "running", Some("虚拟机重启成功")).await;
                } else {
                    // 重启失败后状态取决于失败阶段，简化为 error
                    vm_active.status = Set(VmStatus::Error.as_str().to_string());
                    self.notify_vm_status_update(vm_id, "error", Some(&format!("虚拟机重启失败: {}", message))).await;
                }
            }
            "attach_volume" => {
                if success {
                    self.notify_vm_status_update(vm_id, "running", Some("存储卷挂载成功")).await;
                } else {
                    self.notify_vm_status_update(vm_id, "error", Some(&format!("存储卷挂载失败: {}", message))).await;
                }
            }
            "detach_volume" => {
                if success {
                    self.notify_vm_status_update(vm_id, "running", Some("存储卷分离成功")).await;
                } else {
                    self.notify_vm_status_update(vm_id, "error", Some(&format!("存储卷分离失败: {}", message))).await;
                }
            }
            _ => {
                warn!("未知的虚拟机操作: {}", operation);
            }
        }

        vm_active.updated_at = Set(now.into());
        vm_active.update(db).await?;

        info!("虚拟机 {} 操作 {} 完成: success={}, message={}", vm_id, operation, success, message);
        Ok(())
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

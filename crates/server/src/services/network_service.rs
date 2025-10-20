/// 网络管理服务

use chrono::Utc;
use uuid::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use tracing::{error, info};
use std::net::Ipv4Addr;

use crate::db::models::network::{
    CreateNetworkDto, UpdateNetworkDto, NetworkListResponse, NetworkResponse,
    Entity as NetworkEntity, Column as NetworkColumn, ActiveModel as NetworkActiveModel,
};
use crate::db::models::ip_allocation::{
    IpAllocationListResponse, IpAllocationResponse, IpAllocationStatus,
    Entity as IpAllocationEntity, Column as IpAllocationColumn, ActiveModel as IpAllocationActiveModel,
};
use crate::db::models::vm::Entity as VmEntity;
use crate::app_state::AppState;

pub struct NetworkService {
    state: AppState,
}

impl NetworkService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 创建网络
    pub async fn create_network(&self, dto: CreateNetworkDto) -> anyhow::Result<NetworkResponse> {
        let network_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let network_active = NetworkActiveModel {
            id: Set(network_id.clone()),
            name: Set(dto.name.clone()),
            network_type: Set(dto.network_type.clone()),
            cidr: Set(dto.cidr.clone()),
            gateway: Set(dto.gateway.clone()),
            mtu: Set(dto.mtu.or(Some(1500))),
            vlan_id: Set(dto.vlan_id),
            metadata: Set(dto.metadata),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let network = network_active.insert(&self.state.sea_db()).await?;

        // 如果提供了 CIDR，初始化 IP 池
        if let Some(ref cidr) = dto.cidr {
            if let Err(e) = self.initialize_ip_pool(&network_id, cidr, dto.gateway.as_deref()).await {
                error!("初始化 IP 池失败: {}", e);
            }
        }

        // 注意：网络基础设施（Bridge、VLAN 子接口）将在 VM 创建时按需在节点上自动创建
        // Server 端只负责维护网络元数据，不进行实际的网络配置
        info!("网络元数据已保存，实际网络配置将在 VM 创建时按需创建");

        Ok(NetworkResponse::from(network))
    }

    /// 初始化 IP 池
    async fn initialize_ip_pool(&self, network_id: &str, cidr: &str, gateway: Option<&str>) -> anyhow::Result<()> {
        // 解析 CIDR
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("无效的 CIDR 格式"));
        }

        let base_ip: Ipv4Addr = parts[0].parse()?;
        let prefix_len: u8 = parts[1].parse()?;

        if prefix_len > 30 {
            // 网络太小，不创建 IP 池
            return Ok(());
        }

        // 计算网络中可用的 IP 数量
        let host_bits = 32 - prefix_len;
        let total_ips = 2u32.pow(host_bits as u32);

        // 最多创建 254 个 IP（避免大网络创建过多记录）
        let max_ips = std::cmp::min(total_ips - 2, 254); // 减去网络地址和广播地址

        let base_u32 = u32::from(base_ip);
        let db = &self.state.sea_db();

        for i in 1..=max_ips {
            let ip_u32 = base_u32 + i;
            let ip = Ipv4Addr::from(ip_u32);
            let ip_str = ip.to_string();

            // 跳过网关 IP
            if let Some(gw) = gateway {
                if gw == &ip_str {
                    continue;
                }
            }

            let allocation_id = Uuid::new_v4().to_string();
            let now = Utc::now();

            let allocation_active = IpAllocationActiveModel {
                id: Set(allocation_id),
                network_id: Set(network_id.to_string()),
                ip_address: Set(ip_str),
                mac_address: Set(None),
                vm_id: Set(None),
                status: Set(IpAllocationStatus::Available.as_str().to_string()),
                allocated_at: Set(None),
                created_at: Set(now.into()),
            };

            if let Err(e) = allocation_active.insert(db).await {
                error!("创建 IP 分配记录失败: {}", e);
            }
        }

        info!("为网络 {} 初始化了 {} 个 IP 地址", network_id, max_ips);
        Ok(())
    }

    /// 获取网络列表
    pub async fn list_networks(
        &self,
        page: usize,
        page_size: usize,
        network_type: Option<String>,
    ) -> anyhow::Result<NetworkListResponse> {
        let db = &self.state.sea_db();

        let mut query = NetworkEntity::find();

        if let Some(nt) = network_type {
            query = query.filter(NetworkColumn::NetworkType.eq(nt));
        }

        let total = query.clone().count(db).await? as usize;

        let networks = query
            .order_by_desc(NetworkColumn::CreatedAt)
            .offset(((page - 1) * page_size) as u64)
            .limit(page_size as u64)
            .all(db)
            .await?;

        let network_responses: Vec<NetworkResponse> = networks.into_iter().map(NetworkResponse::from).collect();

        Ok(NetworkListResponse {
            networks: network_responses,
            total,
            page,
            page_size,
        })
    }

    /// 获取单个网络
    pub async fn get_network(&self, network_id: &str) -> anyhow::Result<NetworkResponse> {
        let db = &self.state.sea_db();

        let network = NetworkEntity::find_by_id(network_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("网络不存在"))?;

        Ok(NetworkResponse::from(network))
    }

    /// 更新网络
    pub async fn update_network(&self, network_id: &str, dto: UpdateNetworkDto) -> anyhow::Result<NetworkResponse> {
        let db = &self.state.sea_db();

        let network = NetworkEntity::find_by_id(network_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("网络不存在"))?;

        let mut network_active: NetworkActiveModel = network.into();

        if let Some(name) = dto.name {
            network_active.name = Set(name);
        }
        if let Some(cidr) = dto.cidr {
            network_active.cidr = Set(Some(cidr));
        }
        if let Some(gateway) = dto.gateway {
            network_active.gateway = Set(Some(gateway));
        }
        if let Some(mtu) = dto.mtu {
            network_active.mtu = Set(Some(mtu));
        }
        if let Some(metadata) = dto.metadata {
            network_active.metadata = Set(Some(metadata));
        }

        network_active.updated_at = Set(Utc::now().into());

        let updated_network = network_active.update(db).await?;
        Ok(NetworkResponse::from(updated_network))
    }

    /// 删除网络
    pub async fn delete_network(&self, network_id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 检查是否有 VM 使用此网络
        let allocated_ips = IpAllocationEntity::find()
            .filter(IpAllocationColumn::NetworkId.eq(network_id))
            .filter(IpAllocationColumn::Status.eq(IpAllocationStatus::Allocated.as_str()))
            .count(db)
            .await?;

        if allocated_ips > 0 {
            return Err(anyhow::anyhow!("网络正在被 {} 个虚拟机使用，无法删除", allocated_ips));
        }

        // 删除网络（级联删除 IP 分配）
        NetworkEntity::delete_by_id(network_id)
            .exec(db)
            .await?;

        // 调用所有节点 Agent 删除网络
        if let Err(e) = self.delete_network_on_agents(network_id).await {
            error!("在 Agent 上删除网络失败: {}", e);
        }

        info!("网络 {} 已删除", network_id);
        Ok(())
    }

    /// 注意：此方法已废弃，网络基础设施现在按需清理
    /// 
    /// 新的网络删除流程：
    /// 1. Server 端只删除网络元数据和 IP 分配记录
    /// 2. 网络基础设施（Bridge、VLAN 子接口）会在相关 VM 删除时自动清理
    /// 3. 这样可以避免误删正在使用的网络基础设施
    #[allow(dead_code)]
    async fn delete_network_on_agents(&self, _network_id: &str) -> anyhow::Result<()> {
        // 此方法已废弃，网络基础设施现在按需清理
        info!("网络基础设施将在相关 VM 删除时自动清理");
        Ok(())
    }

    /// 分配 IP 地址（预留状态，不设置 vm_id）
    pub async fn allocate_ip(&self, network_id: &str) -> anyhow::Result<IpAllocationResponse> {
        let db = &self.state.sea_db();

        // 查找可用的 IP
        let available_ip = IpAllocationEntity::find()
            .filter(IpAllocationColumn::NetworkId.eq(network_id))
            .filter(IpAllocationColumn::Status.eq(IpAllocationStatus::Available.as_str()))
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("网络中没有可用的 IP 地址"))?;

        // 更新为预留状态，不设置 vm_id
        let mut ip_active: IpAllocationActiveModel = available_ip.into();
        ip_active.status = Set(IpAllocationStatus::Reserved.as_str().to_string());
        ip_active.allocated_at = Set(Some(Utc::now().into()));

        let updated_ip = ip_active.update(db).await?;
        Ok(IpAllocationResponse::from(updated_ip))
    }

    /// 更新 IP 分配的 vm_id（VM 创建成功后调用）
    pub async fn update_ip_vm_id(&self, ip_allocation_id: &str, vm_id: &str) -> anyhow::Result<IpAllocationResponse> {
        let db = &self.state.sea_db();

        // 查找预留的 IP
        let reserved_ip = IpAllocationEntity::find_by_id(ip_allocation_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("IP 分配记录不存在"))?;

        if reserved_ip.status != IpAllocationStatus::Reserved.as_str() {
            return Err(anyhow::anyhow!("IP 分配记录状态不是预留状态"));
        }

        // 更新为已分配状态并设置 vm_id
        let mut ip_active: IpAllocationActiveModel = reserved_ip.into();
        ip_active.vm_id = Set(Some(vm_id.to_string()));
        ip_active.status = Set(IpAllocationStatus::Allocated.as_str().to_string());

        let updated_ip = ip_active.update(db).await?;
        Ok(IpAllocationResponse::from(updated_ip))
    }

    /// 释放 IP 地址
    pub async fn release_ip(&self, network_id: &str, vm_id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查找已分配给该 VM 的 IP
        let allocated_ips = IpAllocationEntity::find()
            .filter(IpAllocationColumn::NetworkId.eq(network_id))
            .filter(IpAllocationColumn::VmId.eq(vm_id))
            .all(db)
            .await?;

        for ip in allocated_ips {
            let mut ip_active: IpAllocationActiveModel = ip.into();
            ip_active.vm_id = Set(None);
            ip_active.mac_address = Set(None);
            ip_active.status = Set(IpAllocationStatus::Available.as_str().to_string());
            ip_active.allocated_at = Set(None);

            ip_active.update(db).await?;
        }

        Ok(())
    }

    /// 列出网络的 IP 分配
    pub async fn list_ip_allocations(
        &self,
        network_id: &str,
        page: usize,
        page_size: usize,
        status: Option<String>,
    ) -> anyhow::Result<IpAllocationListResponse> {
        let db = &self.state.sea_db();

        let mut query = IpAllocationEntity::find()
            .filter(IpAllocationColumn::NetworkId.eq(network_id));

        if let Some(s) = status {
            query = query.filter(IpAllocationColumn::Status.eq(s));
        }

        let total = query.clone().count(db).await? as usize;

        let allocations = query
            .order_by_asc(IpAllocationColumn::IpAddress)
            .offset(((page - 1) * page_size) as u64)
            .limit(page_size as u64)
            .all(db)
            .await?;

        let allocation_responses: Vec<IpAllocationResponse> = {
            let mut responses = Vec::new();
            for allocation in allocations {
                let mut response = IpAllocationResponse::from(allocation.clone());
                
                // 如果有vm_id，获取虚拟机名称
                if let Some(vm_id) = &allocation.vm_id {
                    if let Ok(Some(vm)) = VmEntity::find_by_id(vm_id.clone()).one(db).await {
                        response.vm_name = Some(vm.name);
                    }
                }
                
                responses.push(response);
            }
            responses
        };

        Ok(IpAllocationListResponse {
            allocations: allocation_responses,
            total,
            page,
            page_size,
        })
    }
}


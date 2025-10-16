/// IP 分配数据模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// IP 分配模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ip_allocations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub network_id: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub vm_id: Option<String>,
    pub status: String,  // available, allocated, reserved
    
    // 时间戳
    pub allocated_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::network::Entity",
        from = "Column::NetworkId",
        to = "super::network::Column::Id"
    )]
    Network,
    
    #[sea_orm(
        belongs_to = "super::vm::Entity",
        from = "Column::VmId",
        to = "super::vm::Column::Id"
    )]
    Vm,
}

impl Related<super::network::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Network.def()
    }
}

impl Related<super::vm::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vm.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// IP 分配状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IpAllocationStatus {
    Available,
    Allocated,
    Reserved,
}

impl IpAllocationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IpAllocationStatus::Available => "available",
            IpAllocationStatus::Allocated => "allocated",
            IpAllocationStatus::Reserved => "reserved",
        }
    }
}

impl From<String> for IpAllocationStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "available" => IpAllocationStatus::Available,
            "allocated" => IpAllocationStatus::Allocated,
            "reserved" => IpAllocationStatus::Reserved,
            _ => IpAllocationStatus::Available,
        }
    }
}

/// 创建 IP 分配 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateIpAllocationDto {
    pub network_id: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub vm_id: Option<String>,
    pub status: Option<String>,
}

/// IP 分配响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct IpAllocationResponse {
    pub id: String,
    pub network_id: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub vm_id: Option<String>,
    pub vm_name: Option<String>,
    pub status: String,
    pub allocated_at: Option<String>,
    pub created_at: String,
}

impl From<Model> for IpAllocationResponse {
    fn from(ip: Model) -> Self {
        Self {
            id: ip.id,
            network_id: ip.network_id,
            ip_address: ip.ip_address,
            mac_address: ip.mac_address,
            vm_id: ip.vm_id,
            vm_name: None, // 将在服务层设置
            status: ip.status,
            allocated_at: ip.allocated_at.map(|dt| dt.to_rfc3339()),
            created_at: ip.created_at.to_rfc3339(),
        }
    }
}

/// IP 分配列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct IpAllocationListResponse {
    pub allocations: Vec<IpAllocationResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}


/// 网络数据模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

/// 网络模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "networks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    #[sea_orm(column_name = "type")]
    pub network_type: String,  // bridge, ovs, macvlan
    pub cidr: Option<String>,
    pub gateway: Option<String>,
    pub mtu: Option<i32>,
    pub vlan_id: Option<i32>,
    
    // 元数据
    pub metadata: Option<JsonValue>,
    
    // 时间戳
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::ip_allocation::Entity")]
    IpAllocations,
}

impl Related<super::ip_allocation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::IpAllocations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 网络类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Bridge,
    Ovs,
    Macvlan,
}

impl NetworkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkType::Bridge => "bridge",
            NetworkType::Ovs => "ovs",
            NetworkType::Macvlan => "macvlan",
        }
    }
}

impl From<String> for NetworkType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "bridge" => NetworkType::Bridge,
            "ovs" => NetworkType::Ovs,
            "macvlan" => NetworkType::Macvlan,
            _ => NetworkType::Bridge,
        }
    }
}

/// 创建网络 DTO
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateNetworkDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    
    #[validate(length(min = 1, max = 50))]
    pub network_type: String,
    
    pub cidr: Option<String>,
    pub gateway: Option<String>,
    pub mtu: Option<i32>,
    pub vlan_id: Option<i32>,
    pub metadata: Option<JsonValue>,
}

/// 更新网络 DTO
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateNetworkDto {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    
    pub cidr: Option<String>,
    pub gateway: Option<String>,
    pub mtu: Option<i32>,
    pub metadata: Option<JsonValue>,
}

/// 网络响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkResponse {
    pub id: String,
    pub name: String,
    pub network_type: String,
    pub cidr: Option<String>,
    pub gateway: Option<String>,
    pub mtu: Option<i32>,
    pub vlan_id: Option<i32>,
    pub metadata: Option<JsonValue>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Model> for NetworkResponse {
    fn from(network: Model) -> Self {
        Self {
            id: network.id,
            name: network.name,
            network_type: network.network_type,
            cidr: network.cidr,
            gateway: network.gateway,
            mtu: network.mtu,
            vlan_id: network.vlan_id,
            metadata: network.metadata,
            created_at: network.created_at.to_rfc3339(),
            updated_at: network.updated_at.to_rfc3339(),
        }
    }
}

/// 网络列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkListResponse {
    pub networks: Vec<NetworkResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}


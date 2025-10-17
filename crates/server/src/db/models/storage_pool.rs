/// 存储池数据模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 存储池模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "storage_pools")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    #[sea_orm(column_name = "type")]
    pub pool_type: String,  // nfs, lvm, ceph, iscsi
    pub status: String,     // active, inactive, error
    
    // 存储池配置 (JSON)
    pub config: JsonValue,
    
    // 容量信息
    pub capacity_gb: Option<i64>,
    pub allocated_gb: Option<i64>,
    pub available_gb: Option<i64>,
    
    // 关联信息
    pub node_id: Option<String>,
    
    // 元数据
    pub metadata: Option<JsonValue>,
    
    // 时间戳
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::volume::Entity")]
    Volumes,
    
    #[sea_orm(
        belongs_to = "super::node::Entity",
        from = "Column::NodeId",
        to = "super::node::Column::Id"
    )]
    Node,
}

impl Related<super::volume::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Volumes.def()
    }
}

impl Related<super::node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Node.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 存储池状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StoragePoolStatus {
    Active,
    Inactive,
    Error,
}

impl StoragePoolStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoragePoolStatus::Active => "active",
            StoragePoolStatus::Inactive => "inactive",
            StoragePoolStatus::Error => "error",
        }
    }
}

/// 创建存储池 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStoragePoolDto {
    pub name: String,
    pub pool_type: String,
    pub config: JsonValue,
    pub capacity_gb: Option<i64>,
    pub node_id: Option<String>,
    pub metadata: Option<JsonValue>,
}

/// 更新存储池 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStoragePoolDto {
    pub name: Option<String>,
    pub status: Option<String>,
    pub config: Option<JsonValue>,
    pub capacity_gb: Option<i64>,
    pub allocated_gb: Option<i64>,
    pub available_gb: Option<i64>,
    pub node_id: Option<String>,
    pub metadata: Option<JsonValue>,
}

/// 存储池响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct StoragePoolResponse {
    pub id: String,
    pub name: String,
    pub pool_type: String,
    pub status: String,
    pub config: JsonValue,
    pub capacity_gb: Option<i64>,
    pub allocated_gb: Option<i64>,
    pub available_gb: Option<i64>,
    pub node_id: Option<String>,
    pub node_name: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Model> for StoragePoolResponse {
    fn from(pool: Model) -> Self {
        Self {
            id: pool.id,
            name: pool.name,
            pool_type: pool.pool_type,
            status: pool.status,
            config: pool.config,
            capacity_gb: pool.capacity_gb,
            allocated_gb: pool.allocated_gb,
            available_gb: pool.available_gb,
            node_id: pool.node_id,
            node_name: None, // 需要单独查询节点名称
            metadata: pool.metadata,
            created_at: pool.created_at.to_rfc3339(),
            updated_at: pool.updated_at.to_rfc3339(),
        }
    }
}

/// 存储池列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct StoragePoolListResponse {
    pub pools: Vec<StoragePoolResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}


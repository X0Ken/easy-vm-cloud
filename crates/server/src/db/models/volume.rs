/// 存储卷数据模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 存储卷模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "volumes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    #[sea_orm(column_name = "type")]
    pub volume_type: String,  // lvm, qcow2, raw, ceph, nfs
    pub size_gb: i64,
    pub pool_id: String,
    pub path: Option<String>,
    pub status: String,  // available, in-use, creating, deleting, error
    
    // 关联信息
    pub node_id: Option<String>,
    pub vm_id: Option<String>,
    
    // 元数据
    pub metadata: Option<JsonValue>,
    
    // 时间戳
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::storage_pool::Entity",
        from = "Column::PoolId",
        to = "super::storage_pool::Column::Id"
    )]
    StoragePool,
    
    #[sea_orm(
        belongs_to = "super::node::Entity",
        from = "Column::NodeId",
        to = "super::node::Column::Id"
    )]
    Node,
    
    #[sea_orm(
        belongs_to = "super::vm::Entity",
        from = "Column::VmId",
        to = "super::vm::Column::Id"
    )]
    Vm,
}

impl Related<super::storage_pool::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StoragePool.def()
    }
}

impl Related<super::node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Node.def()
    }
}

impl Related<super::vm::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vm.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 存储卷状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum VolumeStatus {
    Available,
    InUse,
    Creating,
    Deleting,
    Error,
}

impl VolumeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            VolumeStatus::Available => "available",
            VolumeStatus::InUse => "in-use",
            VolumeStatus::Creating => "creating",
            VolumeStatus::Deleting => "deleting",
            VolumeStatus::Error => "error",
        }
    }
}

/// 创建存储卷 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVolumeDto {
    pub name: String,
    pub pool_id: String,
    pub size_gb: i64,
    pub volume_type: String,  // qcow2, raw
    pub node_id: Option<String>,
    pub source: Option<String>,  // 外部URL，用于下载初始数据
    pub metadata: Option<JsonValue>,
}

/// 更新存储卷 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateVolumeDto {
    pub name: Option<String>,
    pub status: Option<String>,
    pub path: Option<String>,
    pub node_id: Option<String>,
    pub vm_id: Option<String>,
    pub metadata: Option<JsonValue>,
}

/// 调整存储卷大小 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeVolumeDto {
    pub new_size_gb: i64,
}

/// 存储卷响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeResponse {
    pub id: String,
    pub name: String,
    pub volume_type: String,
    pub size_gb: i64,
    pub pool_id: String,
    pub pool_name: Option<String>,
    pub path: Option<String>,
    pub status: String,
    pub node_id: Option<String>,
    pub vm_id: Option<String>,
    pub vm_name: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Model> for VolumeResponse {
    fn from(volume: Model) -> Self {
        Self {
            id: volume.id,
            name: volume.name,
            volume_type: volume.volume_type,
            size_gb: volume.size_gb,
            pool_id: volume.pool_id,
            pool_name: None, // 将在服务层填充
            path: volume.path,
            status: volume.status,
            node_id: volume.node_id,
            vm_id: volume.vm_id,
            vm_name: None, // 将在服务层填充
            metadata: volume.metadata,
            created_at: volume.created_at.to_rfc3339(),
            updated_at: volume.updated_at.to_rfc3339(),
        }
    }
}

/// 存储卷列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeListResponse {
    pub volumes: Vec<VolumeResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}


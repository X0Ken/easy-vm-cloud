/// 存储卷快照数据模型
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 快照模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "snapshots")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub volume_id: String,
    pub status: String, // creating, available, deleting, error
    pub size_gb: Option<i64>,
    pub snapshot_tag: Option<String>, // qemu/libvirt 中的实际快照标签
    pub description: Option<String>,

    // 元数据
    pub metadata: Option<JsonValue>,

    // 时间戳
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::volume::Entity",
        from = "Column::VolumeId",
        to = "super::volume::Column::Id"
    )]
    Volume,
}

impl Related<super::volume::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Volume.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 快照状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotStatus {
    Creating,
    Available,
    Deleting,
    Error,
}

impl SnapshotStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SnapshotStatus::Creating => "creating",
            SnapshotStatus::Available => "available",
            SnapshotStatus::Deleting => "deleting",
            SnapshotStatus::Error => "error",
        }
    }
}

/// 创建快照 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSnapshotDto {
    pub name: String,
    pub volume_id: String,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
}

/// 更新快照 DTO（仅允许更新名称和描述）
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSnapshotDto {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// 快照响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotResponse {
    pub id: String,
    pub name: String,
    pub volume_id: String,
    pub volume_name: Option<String>,
    pub status: String,
    pub size_gb: Option<i64>,
    pub snapshot_tag: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Model> for SnapshotResponse {
    fn from(snapshot: Model) -> Self {
        Self {
            id: snapshot.id,
            name: snapshot.name,
            volume_id: snapshot.volume_id,
            volume_name: None, // 将在服务层填充
            status: snapshot.status,
            size_gb: snapshot.size_gb,
            snapshot_tag: snapshot.snapshot_tag,
            description: snapshot.description,
            metadata: snapshot.metadata,
            created_at: snapshot.created_at.to_rfc3339(),
            updated_at: snapshot.updated_at.to_rfc3339(),
        }
    }
}

/// 快照列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotListResponse {
    pub snapshots: Vec<SnapshotResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

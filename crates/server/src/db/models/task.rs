/// 任务数据模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 任务模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub task_type: String,  // create_vm, delete_vm, migrate_vm, stop_vm, etc.
    pub status: String,     // pending, running, completed, failed, cancelled
    pub progress: i32,      // 0-100
    
    // 任务数据
    pub payload: JsonValue,
    pub result: Option<JsonValue>,
    pub error_message: Option<String>,
    
    // 关联信息
    pub target_type: Option<String>,  // vm, node, volume, network
    pub target_id: Option<String>,
    pub node_id: Option<String>,
    
    // 重试信息
    pub retry_count: i32,
    pub max_retries: i32,
    
    // 用户信息
    pub created_by: Option<i32>,
    
    // 时间戳
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub started_at: Option<DateTimeWithTimeZone>,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::node::Entity",
        from = "Column::NodeId",
        to = "super::node::Column::Id"
    )]
    Node,
    
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Node.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 任务状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }
}

impl From<String> for TaskStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => TaskStatus::Pending,
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        }
    }
}

/// 任务类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    CreateVm,
    DeleteVm,
    StartVm,
    StopVm,
    RestartVm,
    MigrateVm,
    CreateVolume,
    DeleteVolume,
    CreateNetwork,
    DeleteNetwork,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::CreateVm => "create_vm",
            TaskType::DeleteVm => "delete_vm",
            TaskType::StartVm => "start_vm",
            TaskType::StopVm => "stop_vm",
            TaskType::RestartVm => "restart_vm",
            TaskType::MigrateVm => "migrate_vm",
            TaskType::CreateVolume => "create_volume",
            TaskType::DeleteVolume => "delete_volume",
            TaskType::CreateNetwork => "create_network",
            TaskType::DeleteNetwork => "delete_network",
        }
    }
}

/// 任务响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub task_type: String,
    pub status: String,
    pub progress: i32,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub node_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub started_at: Option<DateTimeWithTimeZone>,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

impl From<Model> for TaskResponse {
    fn from(task: Model) -> Self {
        Self {
            id: task.id,
            task_type: task.task_type,
            status: task.status,
            progress: task.progress,
            target_type: task.target_type,
            target_id: task.target_id,
            node_id: task.node_id,
            error_message: task.error_message,
            created_at: task.created_at,
            updated_at: task.updated_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
        }
    }
}

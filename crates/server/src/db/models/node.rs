use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 节点模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "nodes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub hostname: String,
    pub ip_address: String,
    pub status: String,
    pub hypervisor_type: Option<String>,
    pub hypervisor_version: Option<String>,
    
    // 资源信息
    pub cpu_cores: Option<i32>,
    pub cpu_threads: Option<i32>,
    pub memory_total: Option<i64>,
    pub disk_total: Option<i64>,
    
    // 元数据
    pub metadata: Option<serde_json::Value>,
    
    // 时间戳
    pub last_heartbeat: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// 为了兼容现有代码，保留 Node 类型别名
pub type Node = Model;

/// 节点状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Online,
    Offline,
    Maintenance,
    Error,
}

impl NodeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeStatus::Online => "online",
            NodeStatus::Offline => "offline",
            NodeStatus::Maintenance => "maintenance",
            NodeStatus::Error => "error",
        }
    }
}

impl From<String> for NodeStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "online" => NodeStatus::Online,
            "offline" => NodeStatus::Offline,
            "maintenance" => NodeStatus::Maintenance,
            "error" => NodeStatus::Error,
            _ => NodeStatus::Offline,
        }
    }
}

/// 创建节点 DTO
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateNodeDto {
    #[validate(length(min = 1, max = 255))]
    pub hostname: String,
    
    #[validate(length(min = 1, max = 45))]
    pub ip_address: String,
    
    pub hypervisor_type: Option<String>,
    pub hypervisor_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// 更新节点 DTO
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateNodeDto {
    #[validate(length(min = 1, max = 255))]
    pub hostname: Option<String>,
    
    #[validate(length(min = 1, max = 45))]
    pub ip_address: Option<String>,
    
    pub status: Option<String>,
    pub hypervisor_type: Option<String>,
    pub hypervisor_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// 节点响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeResponse {
    pub id: String,
    pub hostname: String,
    pub ip_address: String,
    pub status: String,
    pub hypervisor_type: Option<String>,
    pub hypervisor_version: Option<String>,
    pub cpu_cores: Option<i32>,
    pub cpu_threads: Option<i32>,
    pub memory_total: Option<i64>,
    pub disk_total: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub last_heartbeat: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Node> for NodeResponse {
    fn from(node: Node) -> Self {
        Self {
            id: node.id,
            hostname: node.hostname,
            ip_address: node.ip_address,
            status: node.status,
            hypervisor_type: node.hypervisor_type,
            hypervisor_version: node.hypervisor_version,
            cpu_cores: node.cpu_cores,
            cpu_threads: node.cpu_threads,
            memory_total: node.memory_total,
            disk_total: node.disk_total,
            metadata: node.metadata,
            last_heartbeat: node.last_heartbeat.map(|dt| dt.to_rfc3339()),
            created_at: node.created_at.to_rfc3339(),
            updated_at: node.updated_at.to_rfc3339(),
        }
    }
}

/// 节点列表响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeListResponse {
    pub nodes: Vec<NodeResponse>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
}

/// 节点心跳更新 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeHeartbeatDto {
    pub cpu_cores: Option<i32>,
    pub cpu_threads: Option<i32>,
    pub memory_total: Option<i64>,
    pub disk_total: Option<i64>,
    pub hypervisor_type: Option<String>,
    pub hypervisor_version: Option<String>,
}

/// 节点统计信息
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatsResponse {
    pub total_nodes: i64,
    pub online_nodes: i64,
    pub offline_nodes: i64,
    pub maintenance_nodes: i64,
    pub error_nodes: i64,
}


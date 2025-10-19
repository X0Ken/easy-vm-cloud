/// 虚拟机数据模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 虚拟机模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vms")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub uuid: Option<String>,
    pub name: String,
    pub node_id: Option<String>,
    pub status: String,
    
    // 配置信息
    pub vcpu: i32,
    pub memory_mb: i64,
    pub os_type: String,  // 操作系统类型: linux, windows
    
    // 磁盘和网络配置 (JSON)
    pub disk_ids: Option<JsonValue>,
    pub network_interfaces: Option<JsonValue>,
    
    // 元数据
    pub metadata: Option<JsonValue>,
    
    // 时间戳
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub started_at: Option<DateTimeWithTimeZone>,
    pub stopped_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// 为了兼容现有代码，保留 Vm 类型别名
pub type Vm = Model;

/// VM 状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VmStatus {
    Running,
    Stopped,
    Paused,
    Migrating,
    Error,
}


impl VmStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            VmStatus::Running => "running",
            VmStatus::Stopped => "stopped",
            VmStatus::Paused => "paused",
            VmStatus::Migrating => "migrating",
            VmStatus::Error => "error",
        }
    }
}

impl From<String> for VmStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "running" => VmStatus::Running,
            "stopped" => VmStatus::Stopped,
            "paused" => VmStatus::Paused,
            "migrating" => VmStatus::Migrating,
            "error" => VmStatus::Error,
            _ => VmStatus::Stopped,
        }
    }
}

/// 创建 VM DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVmDto {
    pub name: String,
    pub node_id: String,
    pub vcpu: u32,
    pub memory_mb: u64,
    pub os_type: Option<String>,  // 操作系统类型，默认为 linux
    pub disks: Option<Vec<DiskSpec>>,
    pub networks: Option<Vec<NetworkInterfaceSpec>>,
    pub metadata: Option<JsonValue>,
}

/// 更新 VM DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateVmDto {
    pub name: Option<String>,
    pub vcpu: Option<u32>,
    pub memory_mb: Option<u64>,
    pub os_type: Option<String>,  // 操作系统类型
    pub disks: Option<Vec<DiskSpec>>,
    pub networks: Option<Vec<NetworkInterfaceSpec>>,
    pub metadata: Option<JsonValue>,
}

/// VM 响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct VmResponse {
    pub id: String,
    pub uuid: Option<String>,
    pub name: String,
    pub node_id: Option<String>,
    pub node_name: Option<String>,
    pub status: String,
    pub vcpu: i32,
    pub memory_mb: i64,
    pub os_type: String,  // 操作系统类型
    pub disk_ids: Option<JsonValue>,
    pub network_interfaces: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
}

impl From<Vm> for VmResponse {
    fn from(vm: Vm) -> Self {
        VmResponse {
            id: vm.id,
            uuid: vm.uuid,
            name: vm.name,
            node_id: vm.node_id,
            node_name: None, // 将在服务层设置
            status: vm.status,
            vcpu: vm.vcpu,
            memory_mb: vm.memory_mb,
            os_type: vm.os_type,
            disk_ids: vm.disk_ids,
            network_interfaces: vm.network_interfaces,
            metadata: vm.metadata,
            created_at: vm.created_at.to_rfc3339(),
            updated_at: vm.updated_at.to_rfc3339(),
            started_at: vm.started_at.map(|t| t.to_rfc3339()),
            stopped_at: vm.stopped_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// VM 列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct VmListResponse {
    pub vms: Vec<VmResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

/// 磁盘规格
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiskSpec {
    pub volume_id: String,
    pub device: String,
    pub bootable: bool,
}

/// 网络接口规格
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkInterfaceSpec {
    pub network_id: String,
    pub mac_address: Option<String>,
    pub ip_address: Option<String>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_name: Option<String>,
}

/// Attach Volume 请求
#[derive(Debug, Serialize, Deserialize)]
pub struct AttachVolumeDto {
    pub volume_id: String,
    pub device: String,  // vda, vdb, etc.
    pub bootable: Option<bool>,
}

/// Detach Volume 请求
#[derive(Debug, Serialize, Deserialize)]
pub struct DetachVolumeDto {
    pub volume_id: String,
}

/// VM磁盘信息响应
#[derive(Debug, Serialize, Deserialize)]
pub struct VmDiskResponse {
    pub volume_id: String,
    pub device: String,
    pub bootable: bool,
    pub volume_name: Option<String>,
    pub size_gb: Option<i64>,
    pub volume_type: Option<String>,
    pub path: Option<String>,
}


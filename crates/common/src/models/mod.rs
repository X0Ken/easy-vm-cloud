/// 共享数据模型
/// 
/// 定义 Server 和 Agent 共享的数据结构

use serde::{Deserialize, Serialize};

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Online,
    Offline,
    Maintenance,
    Error,
}

/// 虚拟机状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VmStatus {
    Running,
    Stopped,
    Paused,
    Migrating,
    Error,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    CreateVm,
    DeleteVm,
    StartVm,
    StopVm,
    MigrateVm,
    CreateVolume,
    DeleteVolume,
    SnapshotVolume,
    CreateNetwork,
    DeleteNetwork,
}

/// 存储类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Lvm,
    Qcow2,
    Raw,
    Ceph,
    Nfs,
}

/// 网络类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Bridge,
    Ovs,
    Macvlan,
}

/// 虚拟化类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HypervisorType {
    Kvm,
    Qemu,
    Xen,
}

/// 常量定义
pub mod constants {
    /// 默认 Server 端口
    pub const DEFAULT_SERVER_PORT: u16 = 3000;
    
    /// 默认 Agent gRPC 端口
    pub const DEFAULT_AGENT_PORT: u16 = 50051;
    
    /// 默认心跳间隔（秒）
    pub const DEFAULT_HEARTBEAT_INTERVAL: u64 = 30;
    
    /// 默认节点离线超时（秒）
    pub const DEFAULT_NODE_TIMEOUT: u64 = 90;
}


/// WebSocket RPC 数据类型定义
/// 
/// 对应原来 proto 中定义的消息类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// 心跳相关
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub node_id: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub server_time: i64,
}

// ============================================================================
// 节点信息
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNodeInfoRequest {
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub hostname: String,
    pub ip_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceInfo>,
    pub hypervisor_type: String,
    pub hypervisor_version: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub memory_total: u64,
    pub memory_available: u64,
    pub disk_total: u64,
    pub disk_available: u64,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
}

// ============================================================================
// 虚拟机管理
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmRequest {
    pub vm_id: String,
    pub name: String,
    pub vcpu: u32,
    pub memory_mb: u64,
    pub disks: Vec<DiskSpec>,
    pub networks: Vec<NetworkInterfaceSpec>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSpec {
    pub volume_id: String,
    pub device: String,
    pub bootable: bool,
    pub volume_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceSpec {
    pub network_id: String,
    pub mac_address: String,
    pub ip_address: String,
    pub model: String,
    pub bridge_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmOperationRequest {
    pub vm_id: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmOperationResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmAsyncOperationRequest {
    pub vm_id: String,
    pub task_id: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmAsyncOperationResponse {
    pub success: bool,
    pub message: String,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub vm_id: String,
    pub uuid: String,
    pub name: String,
    pub state: String,
    pub vcpu: u32,
    pub memory_mb: u64,
    pub disks: Vec<DiskInfo>,
    pub networks: Vec<NetworkInterfaceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub volume_id: String,
    pub device: String,
    pub size_bytes: u64,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    pub network_id: String,
    pub mac_address: String,
    pub ip_address: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage_percent: f64,
    pub memory_used_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVmsRequest {
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVmsResponse {
    pub vms: Vec<VmInfo>,
}

// ============================================================================
// 虚拟机迁移
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateVmRequest {
    pub vm_id: String,
    pub target_node_id: String,
    pub target_node_address: String,
    pub live_migration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationProgress {
    pub vm_id: String,
    pub stage: String,
    pub progress_percent: f64,
    pub message: String,
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// 存储管理
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVolumeRequest {
    pub volume_id: String,
    pub name: String,
    pub size_gb: u64,
    pub storage_type: String,
    pub format: String,
    pub pool_id: String,  // 存储池ID，Agent会自动获取存储池信息
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePoolConfig {
    pub pool_id: String,
    pub pool_name: String,
    pub pool_type: String,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVolumeResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVolumeRequest {
    pub volume_id: String,
    pub pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVolumeResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeVolumeRequest {
    pub volume_id: String,
    pub new_size_gb: u64,
    pub pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeVolumeResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotVolumeRequest {
    pub volume_id: String,
    pub snapshot_name: String,
    pub pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotVolumeResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVolumeInfoRequest {
    pub volume_id: String,
    pub pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub volume_id: String,
    pub name: String,
    pub path: String,
    pub size_gb: u64,
    pub actual_size_gb: u64,
    pub format: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVolumesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVolumesResponse {
    pub volumes: Vec<VolumeInfo>,
}

// ============================================================================
// 网络管理
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNetworkRequest {
    pub network_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub network_type: String,
    pub bridge_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNetworkResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteNetworkRequest {
    pub network_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteNetworkResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachInterfaceRequest {
    pub vm_id: String,
    pub interface: NetworkInterfaceSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachInterfaceResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetachInterfaceRequest {
    pub vm_id: String,
    pub mac_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetachInterfaceResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Agent 注册
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub node_id: String,
    pub hostname: String,
    pub ip_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// 节点资源信息上报
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResourceInfo {
    pub node_id: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub memory_total: u64,  // bytes
    pub disk_total: u64,     // bytes
    pub hypervisor_type: Option<String>,
    pub hypervisor_version: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResourceInfoResponse {
    pub success: bool,
    pub message: String,
}


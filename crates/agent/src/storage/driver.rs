/// 存储驱动抽象层
/// 
/// 定义统一的存储驱动接口，支持多种存储后端

use async_trait::async_trait;
use common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 存储卷信息
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

/// 存储池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePoolConfig {
    pub pool_id: String,
    pub pool_name: String,
    pub storage_type: String,
    pub config: HashMap<String, String>,
}

/// 存储驱动 Trait
#[async_trait]
pub trait StorageDriver: Send + Sync + 'static {
    /// 创建存储卷
    async fn create_volume(
        &self,
        volume_id: &str,
        name: &str,
        size_gb: u64,
        format: &str,
        source: Option<&str>,  // 外部URL，可选
    ) -> Result<VolumeInfo>;

    /// 删除存储卷
    async fn delete_volume(&self, volume_id: &str) -> Result<()>;

    /// 调整存储卷大小
    async fn resize_volume(&self, volume_id: &str, new_size_gb: u64) -> Result<VolumeInfo>;

    /// 获取存储卷信息
    async fn get_volume_info(&self, volume_id: &str) -> Result<VolumeInfo>;

    /// 列出所有存储卷
    async fn list_volumes(&self) -> Result<Vec<VolumeInfo>>;

    /// 创建快照
    async fn create_snapshot(&self, volume_id: &str, snapshot_name: &str) -> Result<String>;

    /// 获取存储驱动类型
    fn driver_type(&self) -> &str;
}


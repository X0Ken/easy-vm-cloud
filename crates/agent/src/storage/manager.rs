/// 存储管理器
/// 
/// 负责管理多种存储驱动，根据存储类型分发请求

use common::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::driver::{StorageDriver, StoragePoolConfig, VolumeInfo};
use super::nfs::NfsDriver;

/// 存储管理器
pub struct StorageManager {
    /// 存储驱动映射: pool_id -> driver
    drivers: Arc<RwLock<HashMap<String, Arc<dyn StorageDriver>>>>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            drivers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册存储池驱动
    pub async fn register_pool(&self, pool_config: StoragePoolConfig) -> Result<()> {
        info!("Registering storage pool: {} (type: {})", pool_config.pool_name, pool_config.storage_type);

        let driver: Arc<dyn StorageDriver> = match pool_config.storage_type.as_str() {
            "nfs" => Arc::new(NfsDriver::new(pool_config.clone())?),
            // 未来可以添加更多驱动类型
            // "lvm" => Arc::new(LvmDriver::new(pool_config.clone())?),
            // "ceph" => Arc::new(CephDriver::new(pool_config.clone())?),
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Unsupported storage type: {}",
                    pool_config.storage_type
                )));
            }
        };

        let mut drivers = self.drivers.write().await;
        drivers.insert(pool_config.pool_id.clone(), driver);

        Ok(())
    }

    /// 获取存储驱动
    async fn get_driver(&self, pool_id: &str) -> Result<Arc<dyn StorageDriver>> {
        let drivers = self.drivers.read().await;
        drivers
            .get(pool_id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Storage pool {} not found", pool_id)))
    }

    /// 创建存储卷
    pub async fn create_volume(
        &self,
        pool_id: &str,
        volume_id: &str,
        name: &str,
        size_gb: u64,
        format: &str,
        source: Option<&str>,  // 外部URL，可选
    ) -> Result<VolumeInfo> {
        debug!("Creating volume: pool={}, id={}, name={}, size={}GB, format={}, source={:?}", 
            pool_id, volume_id, name, size_gb, format, source);

        let driver = self.get_driver(pool_id).await?;
        driver.create_volume(volume_id, name, size_gb, format, source).await
    }

    /// 删除存储卷
    pub async fn delete_volume(&self, pool_id: &str, volume_id: &str) -> Result<()> {
        debug!("Deleting volume: pool={}, id={}", pool_id, volume_id);

        let driver = self.get_driver(pool_id).await?;
        driver.delete_volume(volume_id).await
    }

    /// 调整存储卷大小
    pub async fn resize_volume(&self, pool_id: &str, volume_id: &str, new_size_gb: u64) -> Result<VolumeInfo> {
        debug!("Resizing volume: pool={}, id={}, new_size={}GB", pool_id, volume_id, new_size_gb);

        let driver = self.get_driver(pool_id).await?;
        driver.resize_volume(volume_id, new_size_gb).await
    }

    /// 获取存储卷信息
    pub async fn get_volume_info(&self, pool_id: &str, volume_id: &str) -> Result<VolumeInfo> {
        debug!("Getting volume info: pool={}, id={}", pool_id, volume_id);

        let driver = self.get_driver(pool_id).await?;
        driver.get_volume_info(volume_id).await
    }

    /// 列出存储卷
    pub async fn list_volumes(&self, pool_id: &str) -> Result<Vec<VolumeInfo>> {
        debug!("Listing volumes: pool={}", pool_id);

        let driver = self.get_driver(pool_id).await?;
        driver.list_volumes().await
    }

    /// 创建快照
    pub async fn create_snapshot(&self, pool_id: &str, volume_id: &str, snapshot_name: &str) -> Result<String> {
        debug!("Creating snapshot: pool={}, volume={}, snapshot={}", pool_id, volume_id, snapshot_name);

        let driver = self.get_driver(pool_id).await?;
        driver.create_snapshot(volume_id, snapshot_name).await
    }

    /// 克隆存储卷
    pub async fn clone_volume(
        &self,
        pool_id: &str,
        source_volume_id: &str,
        target_volume_id: &str,
        target_name: &str,
    ) -> Result<VolumeInfo> {
        debug!("Cloning volume: pool={}, source={}, target={}, name={}", 
            pool_id, source_volume_id, target_volume_id, target_name);

        let driver = self.get_driver(pool_id).await?;
        driver.clone_volume(source_volume_id, target_volume_id, target_name).await
    }

    /// 检查存储池是否已注册
    pub async fn is_pool_registered(&self, pool_id: &str) -> bool {
        let drivers = self.drivers.read().await;
        drivers.contains_key(pool_id)
    }

    /// 获取已注册的存储池列表
    pub async fn list_registered_pools(&self) -> Vec<String> {
        let drivers = self.drivers.read().await;
        drivers.keys().cloned().collect()
    }
}


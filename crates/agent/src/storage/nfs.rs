/// NFS 存储驱动
/// 
/// 在 NFS 共享目录中创建和管理 qcow2/raw 格式的磁盘镜像

use async_trait::async_trait;
use common::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use super::driver::{StorageDriver, StoragePoolConfig, VolumeInfo};

/// NFS 存储驱动
pub struct NfsDriver {
    /// 存储池配置
    pool_config: StoragePoolConfig,
    /// NFS 挂载点路径
    mount_path: PathBuf,
}

impl NfsDriver {
    /// 创建新的 NFS 驱动实例
    pub fn new(pool_config: StoragePoolConfig) -> Result<Self> {
        // 从配置中获取 NFS 挂载路径
        let mount_path = pool_config
            .config
            .get("mount_path")
            .ok_or_else(|| Error::Config("NFS mount_path not configured".to_string()))?;

        let mount_path = PathBuf::from(mount_path);

        Ok(Self {
            pool_config,
            mount_path,
        })
    }

    /// 获取卷的完整路径
    fn get_volume_path(&self, volume_id: &str, format: &str) -> PathBuf {
        self.mount_path.join(format!("{}.{}", volume_id, format))
    }

    /// 解析卷文件格式
    fn parse_volume_format(path: &Path) -> String {
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("raw")
            .to_string()
    }

    /// 检测文件的实际格式
    async fn detect_file_format(&self, file_path: &Path) -> Result<String> {
        // 使用 qemu-img info 命令检测文件格式
        let output = Command::new("qemu-img")
            .arg("info")
            .arg(file_path)
            .output()
            .await
            .map_err(|e| Error::Storage(format!("Failed to run qemu-img info: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("qemu-img info failed: {}", stderr);
            return Err(Error::Storage(format!("Failed to detect file format: {}", stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // 解析 qemu-img info 输出，查找 "file format:" 行
        for line in output_str.lines() {
            if line.trim().starts_with("file format:") {
                let format = line.split(':').nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "raw".to_string());
                return Ok(format);
            }
        }

        // 如果无法检测到格式，默认为 raw
        warn!("Could not detect file format for {:?}, defaulting to raw", file_path);
        Ok("raw".to_string())
    }

    /// 从路径中提取卷 ID
    fn extract_volume_id(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }

    /// 获取文件实际大小（GB）
    async fn get_file_actual_size(&self, path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path).await
            .map_err(|e| Error::Storage(format!("Failed to get file metadata: {}", e)))?;
        
        let size_bytes = metadata.len();
        Ok(size_bytes / (1024 * 1024 * 1024))
    }

    /// 获取 qcow2 虚拟大小
    async fn get_qcow2_virtual_size(&self, path: &Path) -> Result<u64> {
        let output = Command::new("qemu-img")
            .arg("info")
            .arg("--output=json")
            .arg(path)
            .output()
            .await
            .map_err(|e| Error::Storage(format!("Failed to run qemu-img info: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Storage(format!("qemu-img info failed: {}", stderr)));
        }

        let info: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| Error::Storage(format!("Failed to parse qemu-img output: {}", e)))?;

        let virtual_size = info["virtual-size"]
            .as_u64()
            .ok_or_else(|| Error::Storage("virtual-size not found in qemu-img output".to_string()))?;

        Ok(virtual_size / (1024 * 1024 * 1024))
    }

    /// 创建空白存储卷（内部方法）
    async fn create_blank_volume(
        &self,
        volume_id: &str,
        name: &str,
        size_gb: u64,
        format: &str,
        volume_path: &std::path::Path,
    ) -> Result<VolumeInfo> {
        // 根据格式创建磁盘镜像
        match format {
            "qcow2" => {
                // 使用 qemu-img 创建 qcow2 镜像
                let output = Command::new("qemu-img")
                    .arg("create")
                    .arg("-f")
                    .arg("qcow2")
                    .arg(volume_path)
                    .arg(format!("{}G", size_gb))
                    .output()
                    .await
                    .map_err(|e| Error::Storage(format!("Failed to run qemu-img: {}", e)))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("qemu-img create failed: {}", stderr);
                    return Err(Error::Storage(format!("Failed to create qcow2 image: {}", stderr)));
                }
            }
            "raw" => {
                // 使用 qemu-img 创建 raw 镜像
                let output = Command::new("qemu-img")
                    .arg("create")
                    .arg("-f")
                    .arg("raw")
                    .arg(volume_path)
                    .arg(format!("{}G", size_gb))
                    .output()
                    .await
                    .map_err(|e| Error::Storage(format!("Failed to run qemu-img: {}", e)))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("qemu-img create failed: {}", stderr);
                    return Err(Error::Storage(format!("Failed to create raw image: {}", stderr)));
                }
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Unsupported format: {}",
                    format
                )));
            }
        }

        info!("Successfully created volume {} at {:?}", volume_id, volume_path);

        // 获取实际大小
        let actual_size_gb = self.get_file_actual_size(volume_path).await?;

        Ok(VolumeInfo {
            volume_id: volume_id.to_string(),
            name: name.to_string(),
            path: volume_path.to_string_lossy().to_string(),
            size_gb,
            actual_size_gb,
            format: format.to_string(),
            status: "available".to_string(),
        })
    }

    /// 从外部URL创建存储卷（内部方法）
    async fn create_volume_from_url_internal(
        &self,
        volume_id: &str,
        name: &str,
        size_gb: u64,
        format: &str,
        source_url: &str,
        volume_path: &std::path::Path,
    ) -> Result<VolumeInfo> {
        info!(
            "Creating NFS volume from URL: id={}, name={}, size={}GB, format={}, url={}",
            volume_id, name, size_gb, format, source_url
        );

        // 下载外部URL的内容到临时文件
        let temp_path = volume_path.with_extension("tmp");
        
        // 使用curl下载文件
        let output = Command::new("curl")
            .arg("-L")  // 跟随重定向
            .arg("-o")
            .arg(&temp_path)
            .arg(source_url)
            .output()
            .await
            .map_err(|e| Error::Storage(format!("Failed to download from URL: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("curl download failed: {}", stderr);
            return Err(Error::Storage(format!("Failed to download from URL: {}", stderr)));
        }

        // 检测下载文件的格式
        let detected_format = self.detect_file_format(&temp_path).await?;
        info!("Detected downloaded file format: {}", detected_format);

        // 根据目标格式转换下载的文件
        match format {
            "qcow2" => {
                // 如果检测到的格式已经是qcow2，直接重命名
                if detected_format == "qcow2" {
                    fs::rename(&temp_path, volume_path).await
                        .map_err(|e| Error::Storage(format!("Failed to rename qcow2 file: {}", e)))?;
                } else {
                    // 使用 qemu-img 转换到 qcow2 格式
                    let output = Command::new("qemu-img")
                        .arg("convert")
                        .arg("-f")
                        .arg(&detected_format)
                        .arg("-O")
                        .arg("qcow2")
                        .arg("-o")
                        .arg("preallocation=metadata")
                        .arg(&temp_path)
                        .arg(volume_path)
                        .output()
                        .await
                        .map_err(|e| Error::Storage(format!("Failed to run qemu-img convert: {}", e)))?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("qemu-img convert failed: {}", stderr);
                        return Err(Error::Storage(format!("Failed to convert to qcow2: {}", stderr)));
                    }
                }
            }
            "raw" => {
                // 如果检测到的格式已经是raw，直接重命名
                if detected_format == "raw" {
                    fs::rename(&temp_path, volume_path).await
                        .map_err(|e| Error::Storage(format!("Failed to rename downloaded file: {}", e)))?;
                } else {
                    // 转换到 raw 格式
                    let output = Command::new("qemu-img")
                        .arg("convert")
                        .arg("-f")
                        .arg(&detected_format)
                        .arg("-O")
                        .arg("raw")
                        .arg(&temp_path)
                        .arg(volume_path)
                        .output()
                        .await
                        .map_err(|e| Error::Storage(format!("Failed to run qemu-img convert: {}", e)))?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("qemu-img convert failed: {}", stderr);
                        return Err(Error::Storage(format!("Failed to convert to raw: {}", stderr)));
                    }
                }
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Unsupported format: {}",
                    format
                )));
            }
        }

        // 统一调整到指定大小
        let output = Command::new("qemu-img")
            .arg("resize")
            .arg(volume_path)
            .arg(format!("{}G", size_gb))
            .output()
            .await
            .map_err(|e| Error::Storage(format!("Failed to run qemu-img resize: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("qemu-img resize failed: {}", stderr);
            return Err(Error::Storage(format!("Failed to resize {} image: {}", format, stderr)));
        }

        // 清理临时文件
        if temp_path.exists() {
            let _ = fs::remove_file(&temp_path).await;
        }

        info!("Successfully created volume {} from URL at {:?}", volume_id, volume_path);

        // 获取实际大小
        let actual_size_gb = self.get_file_actual_size(volume_path).await?;

        Ok(VolumeInfo {
            volume_id: volume_id.to_string(),
            name: name.to_string(),
            path: volume_path.to_string_lossy().to_string(),
            size_gb,
            actual_size_gb,
            format: format.to_string(),
            status: "available".to_string(),
        })
    }
}





#[async_trait]
impl StorageDriver for NfsDriver {
    async fn create_volume(
        &self,
        volume_id: &str,
        name: &str,
        size_gb: u64,
        format: &str,
        source: Option<&str>,  // 外部URL，可选
    ) -> Result<VolumeInfo> {
        info!(
            "Creating NFS volume: id={}, name={}, size={}GB, format={}, source={:?}",
            volume_id, name, size_gb, format, source
        );

        let volume_path = self.get_volume_path(volume_id, format);

        // 检查文件是否已存在
        if volume_path.exists() {
            return Err(Error::AlreadyExists(format!(
                "Volume {} already exists",
                volume_id
            )));
        }

        // 确保目录存在
        if let Some(parent) = volume_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| Error::Storage(format!("Failed to create directory: {}", e)))?;
        }

        // 根据是否有source URL选择不同的创建方式
        if let Some(source_url) = source {
            // 从外部URL创建存储卷
            self.create_volume_from_url_internal(volume_id, name, size_gb, format, source_url, &volume_path).await
        } else {
            // 创建空白存储卷
            self.create_blank_volume(volume_id, name, size_gb, format, &volume_path).await
        }
    }



    async fn delete_volume(&self, volume_id: &str) -> Result<()> {
        info!("Deleting NFS volume: {}", volume_id);

        // 尝试找到卷文件（可能是 qcow2 或 raw）
        let formats = vec!["qcow2", "raw"];
        let mut found = false;

        for format in formats {
            let volume_path = self.get_volume_path(volume_id, format);
            
            if volume_path.exists() {
                fs::remove_file(&volume_path).await
                    .map_err(|e| Error::Storage(format!("Failed to delete volume file: {}", e)))?;
                
                info!("Successfully deleted volume {} at {:?}", volume_id, volume_path);
                found = true;
                break;
            }
        }

        if !found {
            return Err(Error::NotFound(format!("Volume {} not found", volume_id)));
        }

        Ok(())
    }

    async fn resize_volume(&self, volume_id: &str, new_size_gb: u64) -> Result<VolumeInfo> {
        info!("Resizing NFS volume: {} to {}GB", volume_id, new_size_gb);

        // 尝试找到卷文件
        let formats = vec!["qcow2", "raw"];
        let mut volume_path = None;
        let mut format = "raw";

        for fmt in formats {
            let path = self.get_volume_path(volume_id, fmt);
            if path.exists() {
                volume_path = Some(path);
                format = fmt;
                break;
            }
        }

        let volume_path = volume_path.ok_or_else(|| {
            Error::NotFound(format!("Volume {} not found", volume_id))
        })?;

        // 使用 qemu-img resize 调整大小
        let output = Command::new("qemu-img")
            .arg("resize")
            .arg(&volume_path)
            .arg(format!("{}G", new_size_gb))
            .output()
            .await
            .map_err(|e| Error::Storage(format!("Failed to run qemu-img resize: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("qemu-img resize failed: {}", stderr);
            return Err(Error::Storage(format!("Failed to resize volume: {}", stderr)));
        }

        info!("Successfully resized volume {}", volume_id);

        // 获取调整后的信息
        let actual_size_gb = self.get_file_actual_size(&volume_path).await?;
        let size_gb = if format == "qcow2" {
            self.get_qcow2_virtual_size(&volume_path).await?
        } else {
            new_size_gb
        };

        Ok(VolumeInfo {
            volume_id: volume_id.to_string(),
            name: volume_id.to_string(),
            path: volume_path.to_string_lossy().to_string(),
            size_gb,
            actual_size_gb,
            format: format.to_string(),
            status: "available".to_string(),
        })
    }

    async fn get_volume_info(&self, volume_id: &str) -> Result<VolumeInfo> {
        debug!("Getting NFS volume info: {}", volume_id);

        // 尝试找到卷文件
        let formats = vec!["qcow2", "raw"];
        let mut volume_path = None;
        let mut format = "raw";

        for fmt in formats {
            let path = self.get_volume_path(volume_id, fmt);
            if path.exists() {
                volume_path = Some(path);
                format = fmt;
                break;
            }
        }

        let volume_path = volume_path.ok_or_else(|| {
            Error::NotFound(format!("Volume {} not found", volume_id))
        })?;

        let actual_size_gb = self.get_file_actual_size(&volume_path).await?;
        let size_gb = if format == "qcow2" {
            self.get_qcow2_virtual_size(&volume_path).await?
        } else {
            actual_size_gb
        };

        Ok(VolumeInfo {
            volume_id: volume_id.to_string(),
            name: volume_id.to_string(),
            path: volume_path.to_string_lossy().to_string(),
            size_gb,
            actual_size_gb,
            format: format.to_string(),
            status: "available".to_string(),
        })
    }

    async fn list_volumes(&self) -> Result<Vec<VolumeInfo>> {
        debug!("Listing NFS volumes in {:?}", self.mount_path);

        let mut volumes = Vec::new();

        // 读取目录中的所有卷文件
        let mut entries = fs::read_dir(&self.mount_path).await
            .map_err(|e| Error::Storage(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::Storage(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            
            // 只处理文件
            if !path.is_file() {
                continue;
            }

            let format = Self::parse_volume_format(&path);
            
            // 只处理 qcow2 和 raw 格式
            if format != "qcow2" && format != "raw" {
                continue;
            }

            if let Some(volume_id) = Self::extract_volume_id(&path) {
                let actual_size_gb = self.get_file_actual_size(&path).await.unwrap_or(0);
                let size_gb = if format == "qcow2" {
                    self.get_qcow2_virtual_size(&path).await.unwrap_or(0)
                } else {
                    actual_size_gb
                };

                volumes.push(VolumeInfo {
                    volume_id: volume_id.clone(),
                    name: volume_id,
                    path: path.to_string_lossy().to_string(),
                    size_gb,
                    actual_size_gb,
                    format,
                    status: "available".to_string(),
                });
            }
        }

        Ok(volumes)
    }

    async fn create_snapshot(&self, volume_id: &str, snapshot_name: &str) -> Result<String> {
        info!("Creating snapshot {} for volume {}", snapshot_name, volume_id);

        // 尝试找到卷文件
        let formats = vec!["qcow2", "raw"];
        let mut volume_path = None;
        let mut format = "raw";

        for fmt in formats {
            let path = self.get_volume_path(volume_id, fmt);
            if path.exists() {
                volume_path = Some(path);
                format = fmt;
                break;
            }
        }

        let volume_path = volume_path.ok_or_else(|| {
            Error::NotFound(format!("Volume {} not found", volume_id))
        })?;

        // qcow2 支持内部快照
        if format == "qcow2" {
            let output = Command::new("qemu-img")
                .arg("snapshot")
                .arg("-c")
                .arg(snapshot_name)
                .arg(&volume_path)
                .output()
                .await
                .map_err(|e| Error::Storage(format!("Failed to run qemu-img snapshot: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("qemu-img snapshot failed: {}", stderr);
                return Err(Error::Storage(format!("Failed to create snapshot: {}", stderr)));
            }

            info!("Successfully created snapshot {} for volume {}", snapshot_name, volume_id);
            Ok(snapshot_name.to_string())
        } else {
            // raw 格式使用拷贝创建快照
            let snapshot_path = self.mount_path.join(format!("{}-{}.{}", volume_id, snapshot_name, format));
            
            fs::copy(&volume_path, &snapshot_path).await
                .map_err(|e| Error::Storage(format!("Failed to copy file for snapshot: {}", e)))?;

            info!("Successfully created snapshot copy at {:?}", snapshot_path);
            Ok(snapshot_name.to_string())
        }
    }

    fn driver_type(&self) -> &str {
        "nfs"
    }
}

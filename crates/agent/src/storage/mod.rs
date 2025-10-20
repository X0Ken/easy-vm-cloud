/// 存储管理
/// 
/// 支持多种存储后端：LVM、QCOW2、Ceph、NFS

pub mod driver;
pub mod manager;
pub mod nfs;

pub use manager::StorageManager;


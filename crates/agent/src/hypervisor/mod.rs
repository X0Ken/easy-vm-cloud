/// 虚拟化管理
/// 
/// 与 libvirt/QEMU/KVM 交互

pub mod manager;

pub use manager::{
    HypervisorManager,
    VMConfig,
    DiskConfig,
    NetworkConfig,
    VMInfo,
};


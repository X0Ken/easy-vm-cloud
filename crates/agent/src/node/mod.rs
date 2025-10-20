/// 节点信息管理模块
/// 
/// 负责获取和管理 Agent 节点的各种信息，包括：
/// - 系统资源信息（CPU、内存、磁盘）
/// - 虚拟化能力检测
/// - 节点配置信息

use common::ws_rpc::NodeResourceInfo;
use std::error::Error;
use tracing::{debug, info};

/// 节点信息管理器
#[derive(Clone)]
pub struct NodeManager {
    /// 节点ID
    node_id: String,
    /// 主机名
    hostname: String,
    /// IP地址
    ip_address: String,
}

impl NodeManager {
    /// 创建新的节点管理器
    pub fn new(
        node_id: impl Into<String>,
        hostname: impl Into<String>,
        ip_address: impl Into<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            hostname: hostname.into(),
            ip_address: ip_address.into(),
        }
    }

    /// 获取节点ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 获取主机名
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// 获取IP地址
    pub fn ip_address(&self) -> &str {
        &self.ip_address
    }

    /// 获取系统资源信息
    pub fn get_system_resource_info(&self) -> Result<NodeResourceInfo, Box<dyn Error + Send + Sync>> {
        use sysinfo::{System, Disks};
        
        let mut sys = System::new_all();
        sys.refresh_all();
        
        // 获取 CPU 信息
        let cpu_cores = sys.cpus().len() as u32;
        let cpu_threads = sys.cpus().len() as u32;
        
        // 获取内存信息
        let memory_total = sys.total_memory() * 1024; // 转换为字节
        
        // 获取磁盘信息
        let disks = Disks::new_with_refreshed_list();
        let disk_total = disks.list().iter()
            .map(|disk| disk.total_space())
            .sum();
        
        // 获取虚拟化信息
        let hypervisor_type = self.detect_hypervisor_type();
        let hypervisor_version = self.detect_hypervisor_version();
        
        Ok(NodeResourceInfo {
            node_id: self.node_id.clone(),
            cpu_cores,
            cpu_threads,
            memory_total,
            disk_total,
            hypervisor_type: Some(hypervisor_type),
            hypervisor_version: Some(hypervisor_version),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// 检测虚拟化类型
    fn detect_hypervisor_type(&self) -> String {
        // 检测 KVM 支持
        if std::path::Path::new("/dev/kvm").exists() {
            debug!("检测到 KVM 支持");
            "kvm".to_string()
        } 
        // 检测 QEMU
        else if std::path::Path::new("/usr/bin/qemu-system-x86_64").exists() {
            debug!("检测到 QEMU");
            "qemu".to_string()
        }
        // 检测其他虚拟化技术
        else if std::path::Path::new("/usr/bin/vmware").exists() {
            debug!("检测到 VMware");
            "vmware".to_string()
        }
        else if std::path::Path::new("/usr/bin/virtualbox").exists() {
            debug!("检测到 VirtualBox");
            "virtualbox".to_string()
        }
        else {
            debug!("未检测到支持的虚拟化技术");
            "unknown".to_string()
        }
    }

    /// 检测虚拟化版本
    fn detect_hypervisor_version(&self) -> String {
        // 尝试获取 KVM 版本
        if std::path::Path::new("/dev/kvm").exists() {
            if let Ok(output) = std::process::Command::new("qemu-system-x86_64")
                .arg("--version")
                .output() {
                if let Ok(version_str) = String::from_utf8(output.stdout) {
                    if let Some(version) = version_str.lines().next() {
                        return version.to_string();
                    }
                }
            }
        }
        
        // 尝试获取 QEMU 版本
        if let Ok(output) = std::process::Command::new("qemu-system-x86_64")
            .arg("--version")
            .output() {
            if let Ok(version_str) = String::from_utf8(output.stdout) {
                if let Some(version) = version_str.lines().next() {
                    return version.to_string();
                }
            }
        }
        
        // 尝试获取 libvirt 版本
        if let Ok(output) = std::process::Command::new("virsh")
            .arg("version")
            .output() {
            if let Ok(version_str) = String::from_utf8(output.stdout) {
                if let Some(version) = version_str.lines().next() {
                    return format!("libvirt {}", version);
                }
            }
        }
        
        "unknown".to_string()
    }

    /// 获取节点基本信息
    pub fn get_node_basic_info(&self) -> NodeBasicInfo {
        NodeBasicInfo {
            node_id: self.node_id.clone(),
            hostname: self.hostname.clone(),
            ip_address: self.ip_address.clone(),
        }
    }

    /// 检查虚拟化能力
    pub fn check_virtualization_capability(&self) -> VirtualizationCapability {
        let hypervisor_type = self.detect_hypervisor_type();
        let has_kvm = std::path::Path::new("/dev/kvm").exists();
        let has_libvirt = std::path::Path::new("/usr/bin/virsh").exists();
        
        VirtualizationCapability {
            hypervisor_type,
            has_kvm,
            has_libvirt,
            supported_architectures: self.get_supported_architectures(),
        }
    }

    /// 获取支持的架构列表
    fn get_supported_architectures(&self) -> Vec<String> {
        let mut architectures = Vec::new();
        
        // 检查 x86_64 支持
        if std::path::Path::new("/usr/bin/qemu-system-x86_64").exists() {
            architectures.push("x86_64".to_string());
        }
        
        // 检查 ARM 支持
        if std::path::Path::new("/usr/bin/qemu-system-aarch64").exists() {
            architectures.push("aarch64".to_string());
        }
        
        // 检查其他架构
        if std::path::Path::new("/usr/bin/qemu-system-arm").exists() {
            architectures.push("arm".to_string());
        }
        
        if std::path::Path::new("/usr/bin/qemu-system-ppc64").exists() {
            architectures.push("ppc64".to_string());
        }
        
        architectures
    }
}

/// 节点基本信息
#[derive(Debug, Clone)]
pub struct NodeBasicInfo {
    pub node_id: String,
    pub hostname: String,
    pub ip_address: String,
}

/// 虚拟化能力信息
#[derive(Debug, Clone)]
pub struct VirtualizationCapability {
    pub hypervisor_type: String,
    pub has_kvm: bool,
    pub has_libvirt: bool,
    pub supported_architectures: Vec<String>,
}

impl VirtualizationCapability {
    /// 检查是否支持虚拟化
    pub fn is_virtualization_supported(&self) -> bool {
        self.has_kvm || self.has_libvirt
    }

    /// 获取主要虚拟化类型
    pub fn primary_hypervisor(&self) -> &str {
        if self.has_kvm {
            "kvm"
        } else if self.has_libvirt {
            "libvirt"
        } else {
            "none"
        }
    }
}

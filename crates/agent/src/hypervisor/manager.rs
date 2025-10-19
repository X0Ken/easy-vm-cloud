/// 虚拟化管理器
/// 
/// 负责与 libvirt 交互，管理虚拟机生命周期

use common::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use virt::connect::Connect;

pub struct HypervisorManager {
    conn: Arc<Mutex<Connect>>,
}

impl HypervisorManager {
    pub fn new() -> Result<Self> {
        // 连接到本地 QEMU/KVM hypervisor
        let conn = Connect::open(Some("qemu:///system"))
            .map_err(|e| common::Error::Internal(format!("无法连接到 libvirt: {}", e)))?;
        
        tracing::info!("✅ 成功连接到 libvirt");
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 检查虚拟机是否存在
    pub async fn vm_exists(&self, vm_id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        
        // 先尝试通过 UUID 查找
        if let Ok(_) = virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            return Ok(true);
        }
        
        // 再尝试通过名称查找
        if let Ok(_) = virt::domain::Domain::lookup_by_name(&conn, vm_id) {
            return Ok(true);
        }
        
        Ok(false)
    }

    /// 创建虚拟机
    pub async fn create_vm(&self, config: &VMConfig) -> Result<String> {
        tracing::info!("🔧 创建虚拟机: {}", config.name);
        
        let conn = self.conn.lock().await;
        
        // 生成虚拟机 XML 配置
        let xml = Self::generate_vm_xml(config)?;
        
        tracing::info!("虚拟机 XML 配置:\n{}", xml);
        
        // 使用 libvirt 定义虚拟机（但不启动）
        let domain = virt::domain::Domain::define_xml(&conn, &xml)
            .map_err(|e| common::Error::Internal(format!("无法定义虚拟机: {}", e)))?;
        
        // 使用传入的 UUID
        let uuid = &config.uuid;
        
        tracing::info!("✅ 虚拟机 {} 定义成功 (UUID: {})", config.name, uuid);
        
        Ok(uuid.clone())
    }
    
    /// 生成虚拟机 XML 配置
    fn generate_vm_xml(config: &VMConfig) -> Result<String> {
        use std::fmt::Write;
        
        let mut xml = String::new();
        
        // 使用传入的 UUID
        let vm_uuid = &config.uuid;
        
        writeln!(xml, "<domain type='kvm'>").unwrap();
        writeln!(xml, "  <name>{}</name>", config.name).unwrap();
        writeln!(xml, "  <uuid>{}</uuid>", vm_uuid).unwrap();
        writeln!(xml, "  <memory unit='MiB'>{}</memory>", config.memory_mb).unwrap();
        writeln!(xml, "  <currentMemory unit='MiB'>{}</currentMemory>", config.memory_mb).unwrap();
        writeln!(xml, "  <vcpu placement='static'>{}</vcpu>", config.vcpu).unwrap();
        
        // CPU 配置 - 根据操作系统类型优化
        if config.os_type == "windows" {
            // Windows 优化：使用 host-model 模式，启用更多特性
            writeln!(xml, "  <cpu mode='host-model' check='partial'>").unwrap();
            writeln!(xml, "    <topology sockets='1' dies='1' cores='{}' threads='1'/>", config.vcpu).unwrap();
            writeln!(xml, "    <feature policy='require' name='vmx'/>").unwrap();
            writeln!(xml, "    <feature policy='require' name='svm'/>").unwrap();
            writeln!(xml, "  </cpu>").unwrap();
        } else {
            // Linux 默认配置
            writeln!(xml, "  <cpu mode='host-passthrough' check='none'/>").unwrap();
        }
        
        // 操作系统配置
        writeln!(xml, "  <os>").unwrap();
        writeln!(xml, "    <type arch='x86_64' machine='pc-q35-7.2'>hvm</type>").unwrap();
        writeln!(xml, "  </os>").unwrap();
        
        // 特性 - 根据操作系统类型优化
        writeln!(xml, "  <features>").unwrap();
        writeln!(xml, "    <acpi/>").unwrap();
        writeln!(xml, "    <apic/>").unwrap();
        if config.os_type == "windows" {
            // Windows 优化特性
            writeln!(xml, "    <hyperv mode='custom'>").unwrap();
            writeln!(xml, "      <relaxed state='on'/>").unwrap();
            writeln!(xml, "      <vapic state='on'/>").unwrap();
            writeln!(xml, "      <spinlocks state='on' retries='8191'/>").unwrap();
            writeln!(xml, "      <vendor_id state='on' value='Microsoft Hv'/>").unwrap();
            writeln!(xml, "    </hyperv>").unwrap();
            writeln!(xml, "    <vmport state='off'/>").unwrap();
        }
        writeln!(xml, "  </features>").unwrap();
        
        // 时钟 - 根据操作系统类型优化
        if config.os_type == "windows" {
            // Windows 优化时钟配置
            writeln!(xml, "  <clock offset='localtime'>").unwrap();
            writeln!(xml, "    <timer name='rtc' tickpolicy='catchup'/>").unwrap();
            writeln!(xml, "    <timer name='pit' tickpolicy='delay'/>").unwrap();
            writeln!(xml, "    <timer name='hpet' present='no'/>").unwrap();
            writeln!(xml, "    <timer name='hypervclock' present='yes'/>").unwrap();
        } else {
            // Linux 默认时钟配置
            writeln!(xml, "  <clock offset='utc'>").unwrap();
            writeln!(xml, "    <timer name='rtc' tickpolicy='catchup'/>").unwrap();
            writeln!(xml, "    <timer name='pit' tickpolicy='delay'/>").unwrap();
            writeln!(xml, "    <timer name='hpet' present='no'/>").unwrap();
        }
        writeln!(xml, "  </clock>").unwrap();
        
        // 电源管理
        writeln!(xml, "  <on_poweroff>destroy</on_poweroff>").unwrap();
        writeln!(xml, "  <on_reboot>restart</on_reboot>").unwrap();
        writeln!(xml, "  <on_crash>destroy</on_crash>").unwrap();
        
        // 设备
        writeln!(xml, "  <devices>").unwrap();
        
        // 模拟器
        writeln!(xml, "    <emulator>/usr/bin/qemu-system-x86_64</emulator>").unwrap();
        
        // 磁盘 - 根据操作系统类型优化
        for (idx, disk) in config.disks.iter().enumerate() {
            writeln!(xml, "    <disk type='file' device='disk'>").unwrap();
            
            // Windows 优化磁盘配置
            if config.os_type == "windows" {
                writeln!(xml, "      <driver name='qemu' type='qcow2' cache='directsync' io='native'/>").unwrap();
            } else {
                writeln!(xml, "      <driver name='qemu' type='qcow2' cache='writeback'/>").unwrap();
            }
            
            writeln!(xml, "      <source file='{}'/>", disk.volume_path).unwrap();
            
            let device_name = if disk.device.is_empty() {
                format!("vd{}", (b'a' + idx as u8) as char)
            } else {
                disk.device.clone()
            };
            
            writeln!(xml, "      <target dev='{}' bus='virtio'/>", device_name).unwrap();
            
            if disk.bootable {
                writeln!(xml, "      <boot order='1'/>").unwrap();
            }
            
            writeln!(xml, "    </disk>").unwrap();
        }
        
        // 网络接口 - 根据操作系统类型优化
        for network in &config.networks {
            // 使用 Bridge 类型直接连接到 Linux Bridge
            writeln!(xml, "    <interface type='bridge'>").unwrap();
            
            if let Some(mac) = &network.mac_address {
                writeln!(xml, "      <mac address='{}'/>", mac).unwrap();
            }
            
            // 使用 bridge_name 而不是 network_name
            let bridge = if network.bridge_name.is_empty() {
                "virbr0"  // 默认 Bridge
            } else {
                &network.bridge_name
            };
            writeln!(xml, "      <source bridge='{}'/>", bridge).unwrap();
            
            let model = if network.model.is_empty() {
                if config.os_type == "windows" {
                    "e1000"  // Windows 优化：使用 e1000 网卡
                } else {
                    "virtio"  // Linux 默认：使用 virtio 网卡
                }
            } else {
                &network.model
            };
            
            writeln!(xml, "      <model type='{}'/>", model).unwrap();
            
            // Windows 网络优化
            if config.os_type == "windows" {
                writeln!(xml, "      <driver name='qemu'/>").unwrap();
            }
            
            writeln!(xml, "    </interface>").unwrap();
        }
        
        // 串口控制台
        writeln!(xml, "    <serial type='pty'>").unwrap();
        writeln!(xml, "      <target type='isa-serial' port='0'>").unwrap();
        writeln!(xml, "        <model name='isa-serial'/>").unwrap();
        writeln!(xml, "      </target>").unwrap();
        writeln!(xml, "    </serial>").unwrap();
        
        writeln!(xml, "    <console type='pty'>").unwrap();
        writeln!(xml, "      <target type='serial' port='0'/>").unwrap();
        writeln!(xml, "    </console>").unwrap();
        
        // VGA 图形 - 根据操作系统类型优化
        writeln!(xml, "    <graphics type='vnc' port='-1' autoport='yes' listen='0.0.0.0'>").unwrap();
        writeln!(xml, "      <listen type='address' address='0.0.0.0'/>").unwrap();
        writeln!(xml, "    </graphics>").unwrap();
        
        writeln!(xml, "    <video>").unwrap();
        if config.os_type == "windows" {
            // Windows 优化：使用 cirrus 显卡，更好的兼容性
            writeln!(xml, "      <model type='cirrus' vram='16384' heads='1' primary='yes'/>").unwrap();
        } else {
            // Linux 默认：使用 qxl 显卡
            writeln!(xml, "      <model type='qxl' ram='65536' vram='65536' vgamem='16384' heads='1' primary='yes'/>").unwrap();
        }
        writeln!(xml, "    </video>").unwrap();
        
        // 输入设备 - 根据操作系统类型优化
        if config.os_type == "windows" {
            // Windows 优化：使用 PS/2 设备，更好的兼容性
            writeln!(xml, "    <input type='mouse' bus='ps2'/>").unwrap();
            writeln!(xml, "    <input type='keyboard' bus='ps2'/>").unwrap();
        } else {
            // Linux 默认：使用 USB 设备
            writeln!(xml, "    <input type='tablet' bus='usb'>").unwrap();
            writeln!(xml, "      <address type='usb' bus='0' port='1'/>").unwrap();
            writeln!(xml, "    </input>").unwrap();
            
            writeln!(xml, "    <input type='mouse' bus='ps2'/>").unwrap();
            writeln!(xml, "    <input type='keyboard' bus='ps2'/>").unwrap();
        }
        
        writeln!(xml, "  </devices>").unwrap();
        writeln!(xml, "</domain>").unwrap();
        
        Ok(xml)
    }

    /// 启动虚拟机
    pub async fn start_vm(&self, vm_id: &str) -> Result<()> {
        // libvirt 域状态常量
        const VIR_DOMAIN_RUNNING: u32 = 1;
        const VIR_DOMAIN_PAUSED: u32 = 3;
        
        tracing::info!("🚀 启动虚拟机: {}", vm_id);
        
        let conn = self.conn.lock().await;
        
        // 通过 UUID 或名称查找虚拟机
        let domain = match virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            Ok(dom) => dom,
            Err(_) => {
                // 如果通过 UUID 查找失败，尝试通过名称查找
                virt::domain::Domain::lookup_by_name(&conn, vm_id)
                    .map_err(|e| common::Error::NotFound(format!("虚拟机不存在: {} ({})", vm_id, e)))?
            }
        };
        
        // 检查虚拟机当前状态
        let (state, _reason) = domain.get_state()
            .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;
        
        // 如果已经在运行，返回成功
        if state == VIR_DOMAIN_RUNNING {
            tracing::info!("✅ 虚拟机 {} 已经在运行", vm_id);
            return Ok(());
        }
        
        // 如果虚拟机处于暂停状态，恢复它
        if state == VIR_DOMAIN_PAUSED {
            tracing::info!("▶️ 恢复暂停的虚拟机: {}", vm_id);
            domain.resume()
                .map_err(|e| common::Error::Internal(format!("无法恢复虚拟机: {}", e)))?;
            tracing::info!("✅ 虚拟机 {} 已恢复", vm_id);
            return Ok(());
        }
        
        // 启动虚拟机
        domain.create()
            .map_err(|e| common::Error::Internal(format!("无法启动虚拟机: {}", e)))?;
        
        tracing::info!("✅ 虚拟机 {} 启动成功", vm_id);
        Ok(())
    }

    /// 停止虚拟机
    pub async fn stop_vm(&self, vm_id: &str, force: bool) -> Result<()> {
        // libvirt 域状态常量
        const VIR_DOMAIN_RUNNING: u32 = 1;
        const VIR_DOMAIN_PAUSED: u32 = 3;
        const VIR_DOMAIN_SHUTOFF: u32 = 5;
        
        tracing::info!("🛑 停止虚拟机: {} (强制: {})", vm_id, force);
        
        let conn = self.conn.lock().await;
        
        // 通过 UUID 或名称查找虚拟机
        let domain = match virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            Ok(dom) => dom,
            Err(_) => {
                // 如果通过 UUID 查找失败，尝试通过名称查找
                virt::domain::Domain::lookup_by_name(&conn, vm_id)
                    .map_err(|e| common::Error::NotFound(format!("虚拟机不存在: {} ({})", vm_id, e)))?
            }
        };
        
        // 检查虚拟机当前状态
        let (state, _reason) = domain.get_state()
            .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;
        
        // 如果已经停止，返回成功
        if state == VIR_DOMAIN_SHUTOFF {
            tracing::info!("✅ 虚拟机 {} 已经停止", vm_id);
            return Ok(());
        }
        
        // 如果虚拟机不在运行状态，无法停止
        if state != VIR_DOMAIN_RUNNING && state != VIR_DOMAIN_PAUSED {
            return Err(common::Error::Internal(format!("虚拟机 {} 不在运行状态，无法停止", vm_id)));
        }
        
        if force {
            // 强制停止虚拟机
            tracing::info!("⚡ 强制停止虚拟机: {}", vm_id);
            domain.destroy()
                .map_err(|e| common::Error::Internal(format!("无法强制停止虚拟机: {}", e)))?;
        } else {
            // 优雅停止虚拟机
            tracing::info!("🔄 优雅停止虚拟机: {}", vm_id);
            domain.shutdown()
                .map_err(|e| common::Error::Internal(format!("无法停止虚拟机: {}", e)))?;
            
            // 等待虚拟机停止（最多等待30秒）
            for _ in 0..30 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                
                let (state, _reason) = domain.get_state()
                    .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;
                
                if state == VIR_DOMAIN_SHUTOFF {
                    tracing::info!("✅ 虚拟机 {} 已优雅停止", vm_id);
                    return Ok(());
                }
            }
            
            // 如果优雅停止超时，尝试强制停止
            tracing::warn!("⚠️ 优雅停止超时，尝试强制停止虚拟机: {}", vm_id);
            domain.destroy()
                .map_err(|e| common::Error::Internal(format!("无法强制停止虚拟机: {}", e)))?;
        }
        
        tracing::info!("✅ 虚拟机 {} 停止成功", vm_id);
        Ok(())
    }

    /// 删除虚拟机
    pub async fn delete_vm(&self, vm_id: &str) -> Result<()> {
        // libvirt 域状态常量
        const VIR_DOMAIN_RUNNING: u32 = 1;
        const VIR_DOMAIN_PAUSED: u32 = 3;
        const VIR_DOMAIN_SHUTOFF: u32 = 5;
        
        tracing::info!("🗑️ 删除虚拟机: {}", vm_id);
        
        let conn = self.conn.lock().await;
        
        // 通过 UUID 或名称查找虚拟机
        let domain = match virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            Ok(dom) => dom,
            Err(_) => {
                // 如果通过 UUID 查找失败，尝试通过名称查找
                virt::domain::Domain::lookup_by_name(&conn, vm_id)
                    .map_err(|e| common::Error::NotFound(format!("虚拟机不存在: {} ({})", vm_id, e)))?
            }
        };
        
        // 检查虚拟机当前状态
        let (state, _reason) = domain.get_state()
            .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;
        
        // 如果虚拟机正在运行或暂停，先停止它
        if state == VIR_DOMAIN_RUNNING || state == VIR_DOMAIN_PAUSED {
            tracing::info!("🛑 虚拟机 {} 正在运行，先停止它", vm_id);
            
            // 强制停止虚拟机
            domain.destroy()
                .map_err(|e| common::Error::Internal(format!("无法停止虚拟机: {}", e)))?;
            
            // 等待虚拟机停止
            for _ in 0..10 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                
                let (state, _reason) = domain.get_state()
                    .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;
                
                if state == VIR_DOMAIN_SHUTOFF {
                    break;
                }
            }
        }
        
        // 删除虚拟机定义
        domain.undefine()
            .map_err(|e| common::Error::Internal(format!("无法删除虚拟机定义: {}", e)))?;
        
        tracing::info!("✅ 虚拟机 {} 删除成功", vm_id);
        Ok(())
    }

    /// 列出虚拟机
    pub async fn list_vms(&self) -> Result<Vec<VMInfo>> {
        // libvirt 域状态常量
        const VIR_DOMAIN_RUNNING: u32 = 1;
        const VIR_DOMAIN_BLOCKED: u32 = 2;
        const VIR_DOMAIN_PAUSED: u32 = 3;
        const VIR_DOMAIN_SHUTDOWN: u32 = 4;
        const VIR_DOMAIN_SHUTOFF: u32 = 5;
        const VIR_DOMAIN_CRASHED: u32 = 6;
        const VIR_DOMAIN_PMSUSPENDED: u32 = 7;
        
        tracing::info!("📋 列出所有虚拟机");
        
        let conn = self.conn.lock().await;
        
        // 获取所有域（虚拟机）
        let domains = conn.list_all_domains(0)
            .map_err(|e| common::Error::Internal(format!("无法列出虚拟机: {}", e)))?;
        
        let mut vm_list = Vec::new();
        
        for domain in domains {
            // 获取虚拟机信息
            let name = domain.get_name()
                .map_err(|e| common::Error::Internal(format!("无法获取虚拟机名称: {}", e)))?;
            
            let uuid = domain.get_uuid_string()
                .map_err(|e| common::Error::Internal(format!("无法获取虚拟机UUID: {}", e)))?;
            
            let (state, _reason) = domain.get_state()
                .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;
            
            // 将状态码转换为可读字符串
            let state_str = match state {
                VIR_DOMAIN_RUNNING => "运行中",
                VIR_DOMAIN_BLOCKED => "阻塞",
                VIR_DOMAIN_PAUSED => "暂停",
                VIR_DOMAIN_SHUTDOWN => "关闭中",
                VIR_DOMAIN_SHUTOFF => "已停止",
                VIR_DOMAIN_CRASHED => "崩溃",
                VIR_DOMAIN_PMSUSPENDED => "电源管理暂停",
                _ => "未知",
            };
            
            vm_list.push(VMInfo {
                id: uuid,
                name,
                state: state_str.to_string(),
            });
        }
        
        tracing::info!("✅ 找到 {} 个虚拟机", vm_list.len());
        Ok(vm_list)
    }
}

/// 虚拟机配置
pub struct VMConfig {
    pub name: String,
    pub uuid: String,  // 使用传入的 UUID
    pub vcpu: u32,
    pub memory_mb: u64,
    pub os_type: String,  // 操作系统类型: linux, windows
    pub disks: Vec<DiskConfig>,
    pub networks: Vec<NetworkConfig>,
}

/// 磁盘配置
pub struct DiskConfig {
    pub volume_path: String,
    pub device: String,  // vda, vdb, etc.
    pub bootable: bool,
}

/// 网络配置
pub struct NetworkConfig {
    pub network_name: String,
    pub bridge_name: String,  // Bridge 名称，例如：br-vlan100
    pub mac_address: Option<String>,
    pub model: String,  // virtio, e1000, etc.
}

/// 虚拟机信息
pub struct VMInfo {
    pub id: String,
    pub name: String,
    pub state: String,
}


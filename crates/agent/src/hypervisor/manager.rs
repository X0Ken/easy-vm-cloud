use common::ws_rpc::types::{DiskBusType, DiskDeviceType};
/// 虚拟化管理器
///
/// 负责与 libvirt 交互，管理虚拟机生命周期
use common::Result;
use serde::{Deserialize, Serialize};
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

        // 磁盘 - 根据操作系统类型和配置优化
        for (idx, volume) in config.volumes.iter().enumerate() {
            let device_type = match volume.device_type {
                DiskDeviceType::Disk => "disk",
                DiskDeviceType::Cdrom => "cdrom",
            };

            writeln!(xml, "    <disk type='file' device='{}'>", device_type).unwrap();

            // 根据设备类型和操作系统优化驱动配置
            match volume.device_type {
                DiskDeviceType::Disk => {
                    if config.os_type == "windows" {
                        writeln!(xml, "      <driver name='qemu' type='{}' cache='directsync' io='native'/>", volume.format).unwrap();
                    } else {
                        writeln!(xml, "      <driver name='qemu' type='{}' cache='writeback'/>", volume.format).unwrap();
                    }
                }
                DiskDeviceType::Cdrom => {
                    writeln!(xml, "      <driver name='qemu' type='raw'/>").unwrap();
                }
            }

            writeln!(xml, "      <source file='{}'/>", volume.volume_path).unwrap();

            // 添加序列号 - 使用 volume_id 作为序列号
            writeln!(xml, "      <serial>{}</serial>", volume.volume_id).unwrap();

            // 自动生成设备名 - 根据总线类型和设备类型
            let device_name = match (volume.bus_type.clone(), volume.device_type.clone()) {
                (DiskBusType::Virtio, DiskDeviceType::Disk) => format!("vd{}", (b'a' + idx as u8) as char),
                (DiskBusType::Scsi, DiskDeviceType::Disk) => format!("sd{}", (b'a' + idx as u8) as char),
                (DiskBusType::Ide, DiskDeviceType::Disk) => format!("hd{}", (b'a' + idx as u8) as char),
                (_, DiskDeviceType::Cdrom) => format!("hd{}", (b'a' + idx as u8) as char),
            };

            // 根据总线类型设置总线和控制器
            match volume.bus_type {
                DiskBusType::Virtio => {
                    writeln!(xml, "      <target dev='{}' bus='virtio'/>", device_name).unwrap();
                }
                DiskBusType::Scsi => {
                    writeln!(xml, "      <target dev='{}' bus='scsi'/>", device_name).unwrap();
                    writeln!(xml, "      <address type='drive' controller='0' bus='0' target='0' unit='{}'/>", idx).unwrap();
                }
                DiskBusType::Ide => {
                    writeln!(xml, "      <target dev='{}' bus='ide'/>", device_name).unwrap();
                }
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

        // VirtIO 串口控制器 - QGA 必需
        writeln!(xml, "    <controller type='virtio-serial' index='0'>").unwrap();
        writeln!(xml, "      <address type='pci' domain='0x0000' bus='0x00' slot='0x06' function='0x0'/>").unwrap();
        writeln!(xml, "    </controller>").unwrap();

        // 检查是否需要 virtio-scsi 控制器
        let needs_virtio_scsi = config.volumes.iter().any(|volume| volume.bus_type == DiskBusType::Scsi);
        if needs_virtio_scsi {
            writeln!(xml, "    <controller type='scsi' index='0' model='virtio-scsi'>").unwrap();
            writeln!(xml, "      <address type='pci' domain='0x0000' bus='0x00' slot='0x07' function='0x0'/>").unwrap();
            writeln!(xml, "    </controller>").unwrap();
        }

        // QEMU Guest Agent 串口设备
        writeln!(xml, "    <channel type='unix'>").unwrap();
        writeln!(xml, "      <source mode='bind'/>").unwrap();
        writeln!(xml, "      <target type='virtio' name='org.qemu.guest_agent.0'/>").unwrap();
        writeln!(xml, "      <address type='virtio-serial' controller='0' bus='0' port='1'/>").unwrap();
        writeln!(xml, "    </channel>").unwrap();

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

    /// 根据配置重新定义并启动虚拟机
    ///
    /// 按照 vms.md 流程：Agent需要重新define xml，确保虚拟机配置与数据库一致。
    pub async fn start_vm_with_config(&self, vm_id: &str, config: &VMConfig) -> Result<()> {
        tracing::info!("🚀 根据配置重新定义并启动虚拟机: {}", vm_id);

        let conn = self.conn.lock().await;

        // 检查虚拟机是否已存在
        if let Ok(domain) = virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            // 如果虚拟机已存在，先删除旧定义
            tracing::info!("虚拟机 {} 已存在，先删除旧定义", vm_id);
            let (state, _reason) = domain.get_state()
                .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;

            // 如果虚拟机正在运行，先停止
            if state == 1 { // VIR_DOMAIN_RUNNING
                tracing::info!("虚拟机 {} 正在运行，先停止", vm_id);
                domain.destroy()
                    .map_err(|e| common::Error::Internal(format!("无法停止虚拟机: {}", e)))?;
            }

            // 删除虚拟机定义
            domain.undefine()
                .map_err(|e| common::Error::Internal(format!("无法删除虚拟机定义: {}", e)))?;
        }

        // 生成新的虚拟机 XML 配置
        let xml = Self::generate_vm_xml(config)?;
        tracing::info!("虚拟机 XML 配置:\n{}", xml);

        // 重新定义虚拟机
        let _domain = virt::domain::Domain::define_xml(&conn, &xml)
            .map_err(|e| common::Error::Internal(format!("无法定义虚拟机: {}", e)))?;

        // 启动虚拟机
        let domain = virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id)
            .map_err(|e| common::Error::Internal(format!("无法查找虚拟机: {}", e)))?;

        domain.create()
            .map_err(|e| common::Error::Internal(format!("无法启动虚拟机: {}", e)))?;

        tracing::info!("✅ 虚拟机 {} 重新定义并启动成功", vm_id);
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

    /// 取消定义虚拟机（用于冷迁移）
    ///
    /// 从节点上移除虚拟机定义，但不删除磁盘文件
    /// 适用于共享存储环境下的冷迁移场景
    pub async fn undefine_vm(&self, vm_id: &str) -> Result<()> {
        tracing::info!("🗑️ 取消定义虚拟机: {}", vm_id);

        let conn = self.conn.lock().await;

        // 查找虚拟机
        let domain = match virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            Ok(dom) => dom,
            Err(_) => {
                // 如果通过 UUID 查找失败，尝试通过名称查找
                match virt::domain::Domain::lookup_by_name(&conn, vm_id) {
                    Ok(dom) => dom,
                    Err(_) => {
                        // 虚拟机不存在，按最终一致性处理，视为已达成目标
                        tracing::info!("ℹ️ 虚拟机 {} 不存在，已按成功处理", vm_id);
                        return Ok(());
                    }
                }
            }
        };

        // 检查虚拟机状态
        let (state, _reason) = domain
            .get_state()
            .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;

        const VIR_DOMAIN_SHUTOFF: u32 = 5;

        // 确保虚拟机已经停止
        if state != VIR_DOMAIN_SHUTOFF {
            return Err(common::Error::InvalidArgument(format!(
                "只能取消定义已停止的虚拟机，当前状态: {}",
                state
            )));
        }

        // 取消定义虚拟机（不删除存储）
        domain
            .undefine()
            .map_err(|e| common::Error::Internal(format!("取消定义虚拟机失败: {}", e)))?;

        tracing::info!("✅ 虚拟机 {} 已取消定义", vm_id);
        Ok(())
    }

    /// 挂载存储卷到虚拟机
    pub async fn attach_volume(
        &self,
        vm_id: &str,
        volume_id: &str,
        volume_path: &str,
        bus_type: DiskBusType,
        device_type: DiskDeviceType,
        format: &str,
    ) -> Result<String> {
        tracing::info!("🔗 挂载存储卷: vm_id={}, volume_id={}, path={}", vm_id, volume_id, volume_path);

        let conn = self.conn.lock().await;

        // 查找虚拟机
        let domain = if let Ok(domain) = virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            domain
        } else if let Ok(domain) = virt::domain::Domain::lookup_by_name(&conn, vm_id) {
            domain
        } else {
            return Err(common::Error::NotFound(format!("虚拟机不存在: {}", vm_id)));
        };

        // 检查虚拟机状态
        let (state, _reason) = domain.get_state()
            .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;

        // libvirt 域状态常量
        const VIR_DOMAIN_RUNNING: u32 = 1;

        if state != VIR_DOMAIN_RUNNING {
            return Err(common::Error::InvalidArgument(format!(
                "仅支持在运行中状态挂载存储卷，当前状态: {}",
                state
            )));
        }
        tracing::info!("虚拟机状态: {} (运行中: true)", state);

        // 获取当前磁盘设备列表，确定下一个设备名
        let device_name = self.get_next_disk_device(&domain).await?;

        // 构建磁盘XML配置
        let disk_xml = self.build_disk_xml(
            volume_path,
            &device_name,
            bus_type,
            device_type,
            format,
            volume_id,
        )?;

        tracing::debug!("磁盘XML配置: {}", disk_xml);

        // 仅在运行中执行热插拔
        tracing::info!("虚拟机正在运行，使用热插拔方式挂载存储卷");
        domain.attach_device(&disk_xml)
            .map_err(|e| common::Error::Internal(format!("挂载存储卷失败: {}", e)))?;

        tracing::info!("✅ 存储卷挂载成功: vm_id={}, volume_id={}, device={}", vm_id, volume_id, device_name);
        Ok(device_name)
    }

    /// 从虚拟机分离存储卷
    pub async fn detach_volume(
        &self,
        vm_id: &str,
        volume_id: &str,
    ) -> Result<()> {
        tracing::info!("🔌 分离存储卷: vm_id={}, volume_id={}", vm_id, volume_id);

        let conn = self.conn.lock().await;

        // 查找虚拟机
        let domain = if let Ok(domain) = virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id) {
            domain
        } else if let Ok(domain) = virt::domain::Domain::lookup_by_name(&conn, vm_id) {
            domain
        } else {
            return Err(common::Error::NotFound(format!("虚拟机不存在: {}", vm_id)));
        };

        // 检查虚拟机状态
        let (state, _reason) = domain.get_state()
            .map_err(|e| common::Error::Internal(format!("无法获取虚拟机状态: {}", e)))?;

        // libvirt 域状态常量
        const VIR_DOMAIN_RUNNING: u32 = 1;

        if state != VIR_DOMAIN_RUNNING {
            return Err(common::Error::InvalidArgument(format!(
                "仅支持在运行中状态分离存储卷，当前状态: {}",
                state
            )));
        }
        tracing::info!("虚拟机状态: {} (运行中: true)", state);

        // 获取虚拟机XML配置，找到要分离的设备详细信息
        let xml = domain.get_xml_desc(0)
            .map_err(|e| common::Error::Internal(format!("获取虚拟机XML失败: {}", e)))?;

        // 根据 volume_id 查找磁盘XML
        match self.find_disk_xml_by_volume_id(&xml, volume_id) {
            Ok(disk_xml) => {
                tracing::debug!("分离磁盘XML: {}", disk_xml);

                // 仅在运行中执行热拔插分离
                tracing::info!("虚拟机正在运行，使用热插拔方式分离存储卷");
                domain.detach_device(&disk_xml)
                    .map_err(|e| common::Error::Internal(format!("分离存储卷失败: {}", e)))?;

                tracing::info!("✅ 存储卷分离成功: vm_id={}, volume_id={}", vm_id, volume_id);
            }
            Err(common::Error::NotFound(_)) => {
                // 存储卷不存在，直接返回成功（最终一致性）
                tracing::warn!("⚠️ 存储卷不存在，跳过分离操作: vm_id={}, volume_id={}", vm_id, volume_id);
            }
            Err(e) => {
                // 其他错误仍然返回
                return Err(e);
            }
        }

        Ok(())
    }

    /// 获取下一个可用的磁盘设备名
    async fn get_next_disk_device(&self, domain: &virt::domain::Domain) -> Result<String> {
        // 获取虚拟机XML配置
        let xml = domain.get_xml_desc(0)
            .map_err(|e| common::Error::Internal(format!("获取虚拟机XML失败: {}", e)))?;

        // 解析XML，查找已使用的磁盘设备
        let used_devices = self.parse_disk_devices(&xml)?;

        // 生成下一个设备名 (vda, vdb, vdc, ...)
        for i in 0..26 {
            let device = format!("vd{}", (b'a' + i as u8) as char);
            if !used_devices.contains(&device) {
                return Ok(device);
            }
        }

        Err(common::Error::Internal("没有可用的磁盘设备名".to_string()))
    }

    /// 解析XML中的磁盘设备名
    fn parse_disk_devices(&self, xml: &str) -> Result<Vec<String>> {
        use roxmltree::Document;

        let doc = Document::parse(xml)
            .map_err(|e| common::Error::Internal(format!("解析XML失败: {}", e)))?;

        let mut devices = Vec::new();

        // 查找所有磁盘设备
        for node in doc.descendants() {
            if node.tag_name().name() == "disk" {
                if let Some(target) = node.children().find(|n| n.tag_name().name() == "target") {
                    if let Some(dev) = target.attribute("dev") {
                        devices.push(dev.to_string());
                    }
                }
            }
        }

        Ok(devices)
    }

    /// 构建磁盘XML配置
    fn build_disk_xml(
        &self,
        volume_path: &str,
        device_name: &str,
        bus_type: DiskBusType,
        device_type: DiskDeviceType,
        format: &str,
        volume_id: &str,
    ) -> Result<String> {
        let bus_str = match bus_type {
            DiskBusType::Virtio => "virtio",
            DiskBusType::Scsi => "scsi",
            DiskBusType::Ide => "ide",
        };

        let device_str = match device_type {
            DiskDeviceType::Disk => "disk",
            DiskDeviceType::Cdrom => "cdrom",
        };

        let xml = format!(
            r#"<disk type="file" device="{}">
                <driver name="qemu" type="{}"/>
                <source file="{}"/>
                <target dev="{}" bus="{}"/>
                <serial>{}</serial>
            </disk>"#,
            device_str, format, volume_path, device_name, bus_str, volume_id
        );

        Ok(xml)
    }

    /// 根据 volume_id 查找磁盘XML配置
    fn find_disk_xml_by_volume_id(&self, xml: &str, volume_id: &str) -> Result<String> {
        use roxmltree::Document;

        let doc = Document::parse(xml)
            .map_err(|e| common::Error::Internal(format!("解析XML失败: {}", e)))?;

        // 查找所有磁盘设备
        for node in doc.descendants() {
            if node.tag_name().name() == "disk" {
                // 查找serial元素，检查是否匹配volume_id
                if let Some(serial) = node.children().find(|n| n.tag_name().name() == "serial") {
                    if let Some(serial_text) = serial.text() {
                        if serial_text.trim() == volume_id {
                            // 找到匹配的磁盘，构建完整的磁盘XML
                            let device_type = node.attribute("device").unwrap_or("disk");

                            // 查找target元素
                            let target = node.children().find(|n| n.tag_name().name() == "target");
                            let device = target.and_then(|t| t.attribute("dev")).unwrap_or("vda");
                            let bus = target.and_then(|t| t.attribute("bus")).unwrap_or("virtio");

                            // 查找driver元素
                            let driver = node.children().find(|n| n.tag_name().name() == "driver");
                            let driver_name = driver.and_then(|d| d.attribute("name")).unwrap_or("qemu");
                            let driver_type = driver.and_then(|d| d.attribute("type")).unwrap_or("qcow2");

                            // 查找source元素
                            let source = node.children().find(|n| n.tag_name().name() == "source");
                            let file_path = source.and_then(|s| s.attribute("file")).unwrap_or("");

                            let disk_xml = format!(
                                r#"<disk type="file" device="{}">
                                    <driver name="{}" type="{}"/>
                                    <source file="{}"/>
                                    <target dev="{}" bus="{}"/>
                                    <serial>{}</serial>
                                </disk>"#,
                                device_type, driver_name, driver_type, file_path, device, bus, volume_id
                            );

                            return Ok(disk_xml);
                        }
                    }
                }
            }
        }

        Err(common::Error::NotFound(format!(
            "未找到存储卷: {}",
            volume_id
        )))
    }

    /// 执行虚拟机热迁移
    ///
    /// # 参数
    /// - vm_id: 虚拟机 ID 或名称
    /// - target_uri: 目标节点的 libvirt URI，例如: "qemu+ssh://user@target-host/system"
    /// - flags: 迁移选项标志
    ///
    /// # 返回
    /// - Ok(()) 表示迁移成功启动
    /// - Err 表示迁移失败
    pub async fn live_migrate(
        &self,
        vm_id: &str,
        target_uri: &str,
        flags: Option<u32>,
    ) -> Result<()> {
        let conn = self.conn.lock().await;

        tracing::info!(
            "🔄 开始热迁移虚拟机: vm_id={}, target={}",
            vm_id,
            target_uri
        );

        // 查找虚拟机
        let domain = virt::domain::Domain::lookup_by_name(&conn, vm_id)
            .or_else(|_| virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id))
            .map_err(|e| common::Error::NotFound(format!("虚拟机不存在: {}", e)))?;

        // 检查虚拟机状态
        let state = domain
            .get_state()
            .map_err(|e| common::Error::Internal(format!("获取虚拟机状态失败: {}", e)))?;

        const VIR_DOMAIN_RUNNING: u32 = 1;
        if state.0 != VIR_DOMAIN_RUNNING {
            return Err(common::Error::InvalidArgument(format!(
                "虚拟机未运行，当前状态: {}",
                state.0
            )));
        }

        // 设置迁移标志
        // VIR_MIGRATE_LIVE = 1 (热迁移)
        // VIR_MIGRATE_PEER2PEER = 2 (点对点迁移，源和目标直接通信)
        // VIR_MIGRATE_TUNNELLED = 4 (通过 libvirtd 隧道传输)
        // VIR_MIGRATE_PERSIST_DEST = 8 (在目标节点持久化定义)
        // VIR_MIGRATE_UNDEFINE_SOURCE = 16 (迁移后在源节点取消定义)
        // VIR_MIGRATE_COMPRESSED = 2048 (压缩传输数据)
        // VIR_MIGRATE_AUTO_CONVERGE = 8192 (自动收敛，在迁移困难时降低 CPU)
        // VIR_MIGRATE_NON_SHARED_DISK = 64 (非共享磁盘，需要迁移存储，不设置此标志表示共享存储，不迁移存储)
        // VIR_MIGRATE_UNSAFE = 512 (允许不安全迁移，用于共享存储但 libvirt 无法自动检测的场景)
        // 注意：后台使用共享存储，因此：
        // 1. 不设置 VIR_MIGRATE_NON_SHARED_DISK (64)，表示共享存储，不迁移存储
        // 2. 设置 VIR_MIGRATE_UNSAFE (512)，绕过 libvirt 的安全检查（在共享存储场景下是安全的）
        let migrate_flags = flags.unwrap_or(
            1 |    // VIR_MIGRATE_LIVE
            2 |    // VIR_MIGRATE_PEER2PEER
            8 |    // VIR_MIGRATE_PERSIST_DEST
            16 |   // VIR_MIGRATE_UNDEFINE_SOURCE
            512 |  // VIR_MIGRATE_UNSAFE (允许共享存储迁移，绕过安全检查)
            2048 | // VIR_MIGRATE_COMPRESSED
            8192, // VIR_MIGRATE_AUTO_CONVERGE
            // 不设置 VIR_MIGRATE_NON_SHARED_DISK (64)，表示共享存储，不迁移存储
        );

        tracing::info!("🔧 迁移标志: 0x{:x}", migrate_flags);

        // 执行迁移
        // 注意：这是阻塞调用，可能需要较长时间
        // 使用简化版本的 migrate 方法
        match domain.migrate(&conn, migrate_flags, None, Some(target_uri), 0) {
            Ok(_) => {
                tracing::info!("✅ 虚拟机热迁移成功: vm_id={}", vm_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ 虚拟机热迁移失败: vm_id={}, error={}", vm_id, e);
                Err(common::Error::Internal(format!("热迁移失败: {}", e)))
            }
        }
    }

    /// 获取虚拟机迁移进度信息
    ///
    /// # 参数
    /// - vm_id: 虚拟机 ID 或名称
    ///
    /// # 返回
    /// - Ok((progress, remaining_time)) 进度百分比和剩余时间(秒)
    /// - Err 表示获取失败
    pub async fn get_migration_progress(&self, vm_id: &str) -> Result<(f64, u64)> {
        let conn = self.conn.lock().await;

        // 查找虚拟机
        let domain = virt::domain::Domain::lookup_by_name(&conn, vm_id)
            .or_else(|_| virt::domain::Domain::lookup_by_uuid_string(&conn, vm_id))
            .map_err(|e| common::Error::NotFound(format!("虚拟机不存在: {}", e)))?;

        // 获取作业信息
        match domain.get_job_info() {
            Ok(job_info) => {
                // job_info 包含迁移进度信息
                // 计算进度百分比
                let total = job_info.data_total.unwrap_or(0);
                let processed = job_info.data_processed.unwrap_or(0);

                let progress = if total > 0 {
                    (processed as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                let remaining_time = job_info.time_remaining.unwrap_or(0);

                Ok((progress, remaining_time))
            }
            Err(e) => {
                tracing::warn!("获取迁移进度失败: {}", e);
                Ok((0.0, 0))
            }
        }
    }

    /// 取消正在进行的虚拟机迁移
    ///
    /// # 参数
    /// - vm_id: 虚拟机 ID 或名称
    ///
    /// # 返回
    /// - Ok(()) 表示取消成功
    /// - Err 表示取消失败
    pub async fn abort_migration(&self, vm_id: &str) -> Result<()> {
        let _conn = self.conn.lock().await;

        tracing::info!("⏹️ 取消虚拟机迁移: vm_id={}", vm_id);

        // 尝试中止当前作业
        // 注意：libvirt Rust 绑定可能不支持此方法，返回未实现错误
        tracing::warn!("⚠️ 取消迁移功能暂不支持，请等待迁移完成");
        Err(common::Error::Internal("取消迁移功能暂不支持".to_string()))
    }
}

/// 虚拟机配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMConfig {
    pub name: String,
    pub uuid: String,  // 使用传入的 UUID
    pub vcpu: u32,
    pub memory_mb: u64,
    pub os_type: String,  // 操作系统类型: linux, windows
    pub volumes: Vec<VolumeConfig>,
    pub networks: Vec<NetworkConfig>,
}


/// 存储卷配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    pub volume_id: String,           // 存储卷ID，用作序列号
    pub volume_path: String,
    pub bus_type: DiskBusType,      // 总线类型: virtio, scsi, ide
    pub device_type: DiskDeviceType, // 设备类型: disk, cdrom
    pub format: String,              // 磁盘格式: qcow2, raw, vmdk 等
}

/// 网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub network_name: String,
    pub bridge_name: String,  // Bridge 名称，例如：br-vlan100
    pub mac_address: Option<String>,
    pub model: String,  // virtio, e1000, etc.
}

/// 虚拟机信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMInfo {
    pub id: String,
    pub name: String,
    pub state: String,
}


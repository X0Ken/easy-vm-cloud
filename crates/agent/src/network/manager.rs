/// 网络管理器
/// 
/// 负责创建、配置网络和网桥

use common::Result;
use tracing::info;
use crate::network::bridge::LinuxBridge;

pub struct NetworkManager {
    bridge: LinuxBridge,
}

impl NetworkManager {
    pub fn new(provider_interface: String) -> Self {
        Self {
            bridge: LinuxBridge::new(provider_interface),
        }
    }

    /// 创建网络
    pub async fn create_network(
        &self,
        network_id: &str,
        name: &str,
        network_type: &str,
        bridge_name: &str,
        vlan_id: Option<u32>,
    ) -> Result<()> {
        info!("创建网络: id={}, name={}, type={}, bridge={}, vlan={:?}", 
              network_id, name, network_type, bridge_name, vlan_id);

        match network_type {
            "bridge" => {
                if let Some(vlan) = vlan_id {
                    // 创建 VLAN 网络
                    self.bridge.create_vlan_network(vlan, bridge_name).await?;
                } else {
                    // 创建无 VLAN 网络
                    self.bridge.create_no_vlan_network(bridge_name).await?;
                }
            }
            "ovs" => {
                return Err(common::Error::Internal("暂不支持 OVS 网络".to_string()));
            }
            _ => {
                return Err(common::Error::Internal(format!("不支持的网络类型: {}", network_type)));
            }
        }

        Ok(())
    }

    /// 删除网络
    pub async fn delete_network(
        &self,
        network_id: &str,
        bridge_name: &str,
        vlan_id: Option<u32>,
    ) -> Result<()> {
        info!("删除网络: id={}, bridge={}, vlan={:?}", network_id, bridge_name, vlan_id);

        if let Some(vlan) = vlan_id {
            self.bridge.delete_vlan_network(vlan, bridge_name).await?;
        } else {
            self.bridge.delete_no_vlan_network(bridge_name).await?;
        }

        Ok(())
    }

    /// 附加网络接口到虚拟机
    pub async fn attach_interface(&self, _vm_id: &str, _network_id: &str) -> Result<()> {
        // 注意：在使用 libvirt 时，网络接口的附加通常在 VM 创建时通过 XML 配置完成
        // 这里暂时不需要额外操作，因为 VM 的 tap 设备会由 libvirt 自动连接到 Bridge
        info!("附加网络接口（由 libvirt 处理）");
        Ok(())
    }

    /// 从虚拟机分离网络接口
    pub async fn detach_interface(&self, _vm_id: &str, _mac_address: &str) -> Result<()> {
        // 注意：在使用 libvirt 时，网络接口的分离通常由 libvirt 处理
        info!("分离网络接口（由 libvirt 处理）");
        Ok(())
    }

    /// 获取 Bridge 名称（根据 VLAN ID）
    pub fn get_bridge_name(vlan_id: Option<u32>) -> String {
        LinuxBridge::generate_bridge_name(vlan_id)
    }
    
    /// 检查 Bridge 是否存在
    pub async fn bridge_exists(&self, bridge_name: &str) -> bool {
        self.bridge.bridge_exists(bridge_name).await
    }
    
    /// 检查 Bridge 是否启动并可用
    pub async fn is_bridge_up(&self, bridge_name: &str) -> bool {
        self.bridge.is_bridge_up(bridge_name).await
    }
}


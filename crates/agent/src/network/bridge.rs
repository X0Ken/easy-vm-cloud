/// Linux Bridge 网络实现
/// 
/// 使用 VLAN 进行网络隔离，通过配置的 provider 接口作为上行
/// 
/// 工作原理：
/// 1. 每个 VLAN 对应一个 Linux Bridge（例如：br-vlan100）
/// 2. Provider 接口的 VLAN 子接口（例如：eth0.100）连接到对应的 Bridge
/// 3. VM 的虚拟网口（tap 设备）连接到对应的 Bridge

use common::Result;
use std::process::Command;
use tracing::{debug, error, info, warn};

pub struct LinuxBridge {
    /// Provider 网络接口（例如：eth0）
    provider_interface: String,
}

impl LinuxBridge {
    pub fn new(provider_interface: String) -> Self {
        Self {
            provider_interface,
        }
    }

    /// 创建 VLAN 网络
    /// 
    /// 步骤：
    /// 1. 检查并创建 VLAN Bridge（例如：br-vlan100）
    /// 2. 检查并创建 Provider 接口的 VLAN 子接口（例如：eth0.100）
    /// 3. 将 VLAN 子接口添加到 Bridge
    pub async fn create_vlan_network(&self, vlan_id: u32, bridge_name: &str) -> Result<()> {
        info!("创建 VLAN {} 网络，Bridge: {}", vlan_id, bridge_name);

        // 1. 检查 Bridge 是否存在
        if !self.bridge_exists(bridge_name).await {
            info!("创建 Bridge: {}", bridge_name);
            self.create_bridge(bridge_name)?;
        } else {
            info!("Bridge {} 已存在", bridge_name);
        }

        // 2. 创建 Provider 接口的 VLAN 子接口
        let vlan_interface = format!("{}.{}", self.provider_interface, vlan_id);
        if !self.interface_exists(&vlan_interface)? {
            info!("创建 VLAN 子接口: {}", vlan_interface);
            self.create_vlan_interface(&vlan_interface, vlan_id)?;
        } else {
            info!("VLAN 子接口 {} 已存在", vlan_interface);
        }

        // 3. 将 VLAN 子接口添加到 Bridge
        if !self.interface_in_bridge(bridge_name, &vlan_interface)? {
            info!("将 {} 添加到 Bridge {}", vlan_interface, bridge_name);
            self.add_interface_to_bridge(bridge_name, &vlan_interface)?;
        } else {
            info!("接口 {} 已在 Bridge {} 中", vlan_interface, bridge_name);
        }

        // 4. 确保 Bridge 和 VLAN 子接口处于 UP 状态
        self.set_interface_up(&vlan_interface)?;
        self.set_interface_up(bridge_name)?;

        info!("VLAN {} 网络创建成功", vlan_id);
        Ok(())
    }

    /// 创建无 VLAN 网络
    /// 
    /// 步骤：
    /// 1. 检查并创建 Bridge（例如：br-default）
    /// 2. 将 Provider 接口直接添加到 Bridge
    /// 3. 确保 Bridge 和 Provider 接口处于 UP 状态
    pub async fn create_no_vlan_network(&self, bridge_name: &str) -> Result<()> {
        info!("创建无 VLAN 网络，Bridge: {}", bridge_name);

        // 1. 检查 Bridge 是否存在
        if !self.bridge_exists(bridge_name).await {
            info!("创建 Bridge: {}", bridge_name);
            self.create_bridge(bridge_name)?;
        } else {
            info!("Bridge {} 已存在", bridge_name);
        }

        // 2. 将 Provider 接口添加到 Bridge
        if !self.interface_in_bridge(bridge_name, &self.provider_interface)? {
            info!("将 {} 添加到 Bridge {}", self.provider_interface, bridge_name);
            self.add_interface_to_bridge(bridge_name, &self.provider_interface)?;
        } else {
            info!("接口 {} 已在 Bridge {} 中", self.provider_interface, bridge_name);
        }

        // 3. 确保 Bridge 和 Provider 接口处于 UP 状态
        self.set_interface_up(&self.provider_interface)?;
        self.set_interface_up(bridge_name)?;

        info!("无 VLAN 网络创建成功");
        Ok(())
    }

    /// 删除无 VLAN 网络
    pub async fn delete_no_vlan_network(&self, bridge_name: &str) -> Result<()> {
        info!("删除无 VLAN 网络，Bridge: {}", bridge_name);

        // 1. 从 Bridge 中移除 Provider 接口
        if self.interface_in_bridge(bridge_name, &self.provider_interface)? {
            info!("从 Bridge {} 移除接口 {}", bridge_name, self.provider_interface);
            self.remove_interface_from_bridge(bridge_name, &self.provider_interface)?;
        }

        // 2. 删除 Bridge（如果没有其他接口）
        if self.bridge_is_empty(bridge_name)? {
            info!("删除空 Bridge: {}", bridge_name);
            self.delete_bridge(bridge_name)?;
        } else {
            info!("Bridge {} 仍有其他接口，保留", bridge_name);
        }

        info!("无 VLAN 网络删除成功");
        Ok(())
    }

    /// 删除 VLAN 网络
    pub async fn delete_vlan_network(&self, vlan_id: u32, bridge_name: &str) -> Result<()> {
        info!("删除 VLAN {} 网络，Bridge: {}", vlan_id, bridge_name);

        let vlan_interface = format!("{}.{}", self.provider_interface, vlan_id);

        // 1. 从 Bridge 中移除 VLAN 子接口
        if self.interface_in_bridge(bridge_name, &vlan_interface)? {
            info!("从 Bridge {} 移除接口 {}", bridge_name, vlan_interface);
            self.remove_interface_from_bridge(bridge_name, &vlan_interface)?;
        }

        // 2. 删除 VLAN 子接口
        if self.interface_exists(&vlan_interface)? {
            info!("删除 VLAN 子接口: {}", vlan_interface);
            self.delete_interface(&vlan_interface)?;
        }

        // 3. 删除 Bridge（如果没有其他接口）
        if self.bridge_is_empty(bridge_name)? {
            info!("删除空 Bridge: {}", bridge_name);
            self.delete_bridge(bridge_name)?;
        } else {
            info!("Bridge {} 仍有其他接口，保留", bridge_name);
        }

        info!("VLAN {} 网络删除成功", vlan_id);
        Ok(())
    }

    /// 创建 Bridge
    fn create_bridge(&self, bridge_name: &str) -> Result<()> {
        let output = Command::new("ip")
            .args(["link", "add", "name", bridge_name, "type", "bridge"])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(common::Error::Internal(format!("创建 Bridge 失败: {}", stderr)));
        }

        Ok(())
    }

    /// 删除 Bridge
    fn delete_bridge(&self, bridge_name: &str) -> Result<()> {
        // 先关闭 Bridge
        let _ = Command::new("ip")
            .args(["link", "set", bridge_name, "down"])
            .output();

        let output = Command::new("ip")
            .args(["link", "delete", bridge_name])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(common::Error::Internal(format!("删除 Bridge 失败: {}", stderr)));
        }

        Ok(())
    }

    /// 检查 Bridge 是否存在
    pub async fn bridge_exists(&self, bridge_name: &str) -> bool {
        let output = match Command::new("ip")
            .args(["link", "show", bridge_name])
            .output()
        {
            Ok(output) => output,
            Err(_) => return false,
        };

        output.status.success()
    }
    
    /// 检查 Bridge 是否启动并可用
    pub async fn is_bridge_up(&self, bridge_name: &str) -> bool {
        let output = match Command::new("ip")
            .args(["link", "show", bridge_name])
            .output()
        {
            Ok(output) => output,
            Err(_) => return false,
        };

        if !output.status.success() {
            return false;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // 检查接口状态是否为 UP
        stdout.contains("state UP") || stdout.contains("UP")
    }

    /// 检查接口是否存在
    fn interface_exists(&self, interface: &str) -> Result<bool> {
        let output = Command::new("ip")
            .args(["link", "show", interface])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        Ok(output.status.success())
    }

    /// 创建 VLAN 子接口
    fn create_vlan_interface(&self, vlan_interface: &str, vlan_id: u32) -> Result<()> {
        let output = Command::new("ip")
            .args([
                "link",
                "add",
                "link",
                &self.provider_interface,
                "name",
                vlan_interface,
                "type",
                "vlan",
                "id",
                &vlan_id.to_string(),
            ])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(common::Error::Internal(format!("创建 VLAN 子接口失败: {}", stderr)));
        }

        Ok(())
    }

    /// 删除网络接口
    fn delete_interface(&self, interface: &str) -> Result<()> {
        let output = Command::new("ip")
            .args(["link", "delete", interface])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(common::Error::Internal(format!("删除接口失败: {}", stderr)));
        }

        Ok(())
    }

    /// 将接口添加到 Bridge
    fn add_interface_to_bridge(&self, bridge_name: &str, interface: &str) -> Result<()> {
        let output = Command::new("ip")
            .args(["link", "set", interface, "master", bridge_name])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(common::Error::Internal(format!("添加接口到 Bridge 失败: {}", stderr)));
        }

        Ok(())
    }

    /// 从 Bridge 移除接口
    fn remove_interface_from_bridge(&self, _bridge_name: &str, interface: &str) -> Result<()> {
        let output = Command::new("ip")
            .args(["link", "set", interface, "nomaster"])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(common::Error::Internal(format!("从 Bridge 移除接口失败: {}", stderr)));
        }

        Ok(())
    }

    /// 检查接口是否在 Bridge 中
    fn interface_in_bridge(&self, bridge_name: &str, interface: &str) -> Result<bool> {
        let output = Command::new("ip")
            .args(["link", "show", interface])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains(&format!("master {}", bridge_name)))
    }

    /// 检查 Bridge 是否为空（没有其他接口）
    fn bridge_is_empty(&self, bridge_name: &str) -> Result<bool> {
        let output = Command::new("ls")
            .arg(format!("/sys/class/net/{}/brif", bridge_name))
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            // Bridge 不存在或无法访问，视为空
            return Ok(true);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().is_empty())
    }

    /// 设置接口为 UP 状态
    fn set_interface_up(&self, interface: &str) -> Result<()> {
        let output = Command::new("ip")
            .args(["link", "set", interface, "up"])
            .output()
            .map_err(|e| common::Error::Internal(format!("执行命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("设置接口 {} 为 UP 失败: {}", interface, stderr);
            // 不返回错误，因为这可能不是致命问题
        }

        Ok(())
    }

    /// 生成 Bridge 名称（根据 VLAN ID）
    pub fn generate_bridge_name(vlan_id: Option<u32>) -> String {
        match vlan_id {
            Some(id) => format!("br-vlan{}", id),
            None => "br-default".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bridge_name() {
        assert_eq!(LinuxBridge::generate_bridge_name(Some(100)), "br-vlan100");
        assert_eq!(LinuxBridge::generate_bridge_name(Some(200)), "br-vlan200");
        assert_eq!(LinuxBridge::generate_bridge_name(None), "br-default");
    }
}


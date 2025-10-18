# 网络管理指南

## 概述

Easy VM Cloud 的网络管理系统基于 Linux Bridge 实现网络隔离，支持 VLAN 和无 VLAN 两种网络模式，满足不同场景的网络需求。

## 架构设计

### 网络模型

- **网络类型**：支持 Linux Bridge（当前实现）和 Open vSwitch（计划支持）
- **网络隔离**：支持 VLAN 和无 VLAN 两种模式
- **IP 地址管理（IPAM）**：内置简单的 IP 池管理，支持自动分配和释放

### Linux Bridge + VLAN 模式

每个 VLAN 对应一个独立的 Linux Bridge，通过以下方式实现：

1. **Provider 接口**：物理网卡（例如：`eth0`）作为上行接口
2. **VLAN 子接口**：在 Provider 接口上创建 VLAN 子接口（例如：`eth0.100`）
3. **Bridge**：为每个 VLAN 创建一个 Bridge（例如：`br-vlan100`）
4. **VM 接口**：VM 的虚拟网口（tap 设备）连接到对应的 Bridge

### Linux Bridge 无 VLAN 模式

当用户不指定 VLAN ID 时，系统将创建无 VLAN 的网络，直接使用 Provider 接口：

1. **Provider 接口**：物理网卡（例如：`eth0`）作为上行接口
2. **Bridge**：创建一个默认 Bridge（例如：`br-default`）
3. **直接连接**：Provider 接口直接连接到 Bridge
4. **VM 接口**：VM 的虚拟网口（tap 设备）连接到 Bridge

#### VLAN 模式拓扑图

```
             ┌───────────────────────────┐
             │      物理网络（上联交换机） │
             └───────────┬───────────────┘
                         │
                 ┌───────▼─────────┐
                 │ eth0 (Provider) │
                 └───────┬─────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
 ┌────────────┐   ┌────────────┐   ┌────────────┐
 │ eth0.100   │   │ eth0.200   │   │ eth0.300   │
 │ VLAN 100   │   │ VLAN 200   │   │ VLAN 300   │
 └─────┬──────┘   └─────┬──────┘   └─────┬──────┘
       │                │                │
       ▼                ▼                ▼
 ┌────────────┐   ┌────────────┐   ┌────────────┐
 │ br-vlan100 │   │ br-vlan200 │   │ br-vlan300 │
 │ (Bridge)   │   │ (Bridge)   │   │ (Bridge)   │
 └─────┬──────┘   └─────┬──────┘   └─────┬──────┘
       │                │                │
       ▼                ▼                ▼
 ┌────────────┐   ┌────────────┐   ┌────────────┐
 │ VM tap1    │   │ VM tap2    │   │ VM tap3    │
 │ (Guest NIC)│   │ (Guest NIC)│   │ (Guest NIC)│
 └────────────┘   └────────────┘   └────────────┘

```

#### 无 VLAN 模式拓扑图

```
             ┌───────────────────────────┐
             │      物理网络（上联交换机） │
             └───────────┬───────────────┘
                         │
                 ┌───────▼─────────┐
                 │ eth0 (Provider) │
                 └───────┬─────────┘
                         │
                         ▼
                 ┌────────────┐
                 │ br-default │
                 │ (Bridge)   │
                 └─────┬──────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
        ▼              ▼              ▼
 ┌────────────┐ ┌────────────┐ ┌────────────┐
 │ VM tap1    │ │ VM tap2    │ │ VM tap3    │
 │ (Guest NIC)│ │ (Guest NIC)│ │ (Guest NIC)│
 └────────────┘ └────────────┘ └────────────┘

```



### 权限要求

Agent 需要 root 权限或 CAP_NET_ADMIN 能力来管理网络接口：


## 故障排查

### 检查 Bridge 状态

```bash
# 列出所有 Bridge
ip link show type bridge

# 查看 Bridge 的接口
ls /sys/class/net/br-vlan100/brif/

# 查看 Bridge 详细信息
ip -d link show br-vlan100
```

### 检查 VLAN 子接口

```bash
# 列出所有 VLAN 接口
ip -d link show | grep vlan

# 查看特定 VLAN 接口
ip -d link show eth0.100
```

### 检查接口状态

```bash
# 确保接口为 UP 状态
ip link show br-vlan100
ip link show eth0.100

# 手动启动接口
sudo ip link set br-vlan100 up
sudo ip link set eth0.100 up
```

## 使用场景

### VLAN 模式适用场景
- 多租户环境，需要网络隔离
- 企业级部署，需要 VLAN 分段
- 与现有网络基础设施集成

### 无 VLAN 模式适用场景
- 单租户环境，不需要网络隔离
- 开发测试环境，简化网络配置
- 与不支持 VLAN 的网络设备集成

## 未来计划

- [ ] 支持 Open vSwitch（OVS）
- [ ] 支持 VXLAN 网络
- [ ] 支持外部 SDN 控制器集成
- [ ] 支持网络策略和安全组
- [ ] 支持 DHCP 服务
- [ ] 支持 IPv6


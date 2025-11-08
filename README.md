# Easy VM Cloud 虚拟机管理系统

设计一套可生产、可扩展、可维护的虚拟机管理（VM Management）系统，包含节点管理、网络管理、存储管理和虚拟机管理。

## 功能特性

### 虚拟机管理
- ✅ 虚拟机生命周期管理
- ✅ 冷迁移（关机状态下迁移）
- ✅ 热迁移（运行状态下在线迁移）

### 服务器节点管理
- ✅ 服务器信息展示
- ✅ 服务器配置

### 存储池管理
- ✅ 存储池管理
- ✅ 虚拟磁盘生命周期管理
- ✅ 快照支持
- ✅ NFS存储支持
- ✅ 共享文件系统支持
- 🚧 Ceph存储（计划中）
- 🚧 iSCSI存储（计划中）
- 🚧 LVM存储（计划中）

### 网络管理
- ✅ IP 地址池管理（IPAM）
- ✅ 网络隔离
- ✅ Linux Bridge + VLAN 网络模型
- 🚧 OpenVSwitch 网络模型（计划中）

### 可观测
- 🚧 观测指标（计划中）

## 项目概览

本项目采用前后端分离架构：
- **前端**: 使用 Angular 20 + ng-zorro-antd，提供现代化管理界面
- **Server (后端)**: 使用 Rust + Axum 框架，提供 RESTful API 服务
- **Agent (节点代理)**: 使用 Rust 实现，运行在每个物理/虚拟宿主机上，负责与本地虚拟化栈（libvirt/qemu/kvm、openvswitch、LVM等）交互
- **数据库**: PostgreSQL
- **通信协议**: Server ↔ 前端使用 REST API，Server ↔ Agent 使用 WebSocket 实现双向 RPC 调用

## 快速部署

一键部署 Easy VM Cloud：
适用于Ubuntu 24.04系统

```bash
curl -fsSL https://raw.githubusercontent.com/x0ken/easy-vm-cloud/main/scripts/install.sh | sudo bash
```

## 文档

- [快速开始指南](./docs/quick-start.md)
- [架构设计](./docs/architecture-design.md)
- [WebSocket RPC 协议](./docs/websocket-rpc-protocol.md)
- [网络管理指南](./docs/network-guide.md)
- [项目结构](./docs/project_structure.md)

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！

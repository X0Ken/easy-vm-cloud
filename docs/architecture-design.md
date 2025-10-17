# Web Admin 管理系统 — 架构设计

> 基于 Rust(Axum) 后端 + Angular 前端的轻量级虚拟机管理系统架构设计文档。

---

## 1 目标与范围

**目标**：设计一套可生产、可扩展、可维护的虚拟机管理（VM Management）系统，包含节点管理、网络管理、存储管理和虚拟机管理。

**范围**：
- 前端：Angular 20 + ng-zorro-antd 管理控制台
- 后端：Rust + Axum 提供 RESTful API
- 节点代理（Agent）：Rust 实现，运行在每个物理/虚拟宿主机上，负责与本地虚拟化栈（libvirt/qemu/kvm、openvswitch、LVM等）交互
- 元数据持久化：PostgreSQL
- 缓存：Redis
- 日志与观测：Prometheus + Grafana + Loki

---

## 2 高级架构概览

```
+--------------------+      HTTPS/REST       +--------------------+        WebSocket      +-------------+
|  Angular Admin UI  | <--------------------> |  Axum REST Backend | <------------------> | Node Agents |
+--------------------+                        +--------------------+                      +-------------+
          |                                            |                                      |
          |                                            |                                      |
          |                                            |                                      |
          |                                            |--(metrics)--> Prometheus             |
          v                                            v                                      v
     Browser / User                                 PostgreSQL                              libvirt, OVS,
                                                    Redis (cache)                          LVM, Ceph client
                                                    Object Storage (Backups)
```

说明：
- 前端通过 HTTPS / REST 与后端通信。
- 后端与节点代理之间采用 **WebSocket**，Agents通过WebSocket连接到server端，实现双向RPC调用。
- WebSocket RPC 使用 JSON 消息格式，支持请求/响应、通知和流式数据传输。
- 后端将状态与业务元数据存储在 PostgreSQL。缓存与短期协调用 Redis。
- 节点代理直接调用本地 libvirt/qemu/kvm、openvswitch、LVM、Ceph 等实现对 VM/网络/存储的操作。

---

## 3 关键组件详述

### 3.1 前端 (Angular 20)

**职责**：展示集群视图、节点/网络/存储/VM 管理界面、任务日志与告警、用户与角色管理。

**技术栈与结构**：
- Angular 20 + TypeScript
- UI 库：ng-zorro-antd
- 状态管理：NgRx（可选）或 Services + RxJS
- 路由：按模块划分（dashboard, nodes, vms, storage, network, auth, settings）
- 国际化：Angular i18n
- 登录：jwt

**典型页面**：
- Dashboard（集群健康、资源汇总）
- 节点列表 + 详情（CPU/内存/磁盘/网络）
- VM 列表 + 控制台（启动/停止/重启/控制台/迁移）
- 存储池管理（创建、扩容、快照、回滚）
- 网络拓扑（Bridge/OVS、端口、子网）
- 任务/作业中心（进度、日志）
- 审计/日志

---

### 3.2 后端 (Rust + Axum)

**职责**：提供 RESTful API，接收前端请求，执行业务逻辑、访问数据库、调度异步任务、与 Node Agent 协作。

**主要库/组件**：
- Axum：HTTP/REST 框架
- sea-orm：PostgreSQL 数据库访问
- serde + serde_json：序列化
- tokio：异步运行时
- tracing + slog：日志
- open-telemetry：分布式追踪

**接口风格**：RESTful JSON API，支持分页、过滤、排序。

**认证与鉴权**：
- 前端用户认证使用JWT
- 细粒度权限使用 RBAC（角色/策略表）

**业务模块**：
- Auth（用户、角色、Token）
- Nodes（注册、心跳、指标、操作）
- VM（CRUD、控制、迁移、快照）
- Storage（池、卷、快照、导入/导出）
- Network（bridge、ovs、subnet、IPAM）
- Tasks（作业队列、状态、重试）
- Audit（操作日志、审计）

**异步任务执行**：
- 在后端提交任务到 queue，工作消费者（可以是后端进程或独立 worker）负责处理任务并与 Agent 协调。
- 例如：VM 克隆 -> 将任务放入队列 -> worker 调用 Agent 的 RPC 控制接口 -> Agent 执行并上报进度流 -> 后端更新任务状态。

---

### 3.3 节点代理 (Agent，Rust)

**职责**：运行在每个宿主机上，直接与宿主机虚拟化栈交互，执行控制命令、收集指标并上报，管理 VM 的生命周期操作。

**实现细节**：
- Agent 通过 WebSocket 连接到 Server 的 `/ws/agent` 端点。
- Agent 启动时发送注册请求，Server 维护所有在线 Agent 的连接列表。
- 双向 RPC 调用：Server 可以向 Agent 发送请求（创建VM等），Agent 可以向 Server 发送通知（心跳、状态更新等）。
- 消息格式：JSON，包含消息类型（request/response/notification/stream）、方法名、负载和错误信息。
- 与本地 libvirt 交互可调用 `libvirt` 的 C API（通过 `libvirt` 的 FFI crate）。
- 网络：调用 `ovs-vsctl` 或 `ip` 命令管理桥接/OVS，或使用 openvswitch 的 OVSDB API。
- 存储：支持 LVM、QCOW2、Ceph RBD、NFS。通过命令行或 librbd 接口实现。
- 指标暴露：Prometheus exporter（/metrics）
- 心跳：Agent 每 30 秒通过 WebSocket 发送心跳通知，Server 90秒未收到心跳则标记节点离线。
- 自动重连：Agent 断线后每 5 秒尝试重新连接。

**安全**：
- 生产环境使用 WSS（WebSocket over TLS）加密传输
- Agent 连接时携带认证令牌
- Server 验证令牌后允许注册
- 最小权限运行（systemd unit, 限制能力）

---

## 4 数据模型（概要）

### 4.1 PostgreSQL 核心表（示例）

- `users` (id, username, password_hash, email, enabled, created_at)
- `roles` (id, name, description)
- `role_bindings` (user_id, role_id)
- `nodes` (id, hostname, ip, status, cpu_total, cpu_used, mem_total, mem_used, disk_total, disk_used, meta jsonb, last_heartbeat)
- `vms` (id, uuid, name, node_id, status, vcpu, memory_mb, disk_ids jsonb, network_interfaces jsonb, created_at)
- `volume_pools` (id, name, type, size_gb, meta jsonb)
- `volumes` (id, name, type, size_gb, pool_id, status, meta jsonb)
- `networks` (id, name, type, cidr, gateway, mtu, meta jsonb)
- `tasks` (id, type, payload jsonb, status, progress, created_by, created_at, updated_at)
- `audit_logs` (id, user_id, action, target_type, target_id, detail jsonb, timestamp)

---

## 5 API 设计（示例）

- `POST /api/auth/login` — 登录
- `GET /api/nodes` — 列表节点
- `GET /api/nodes/{id}` — 节点详情
- `POST /api/vms` — 创建 VM
- `POST /api/vms/{id}/start` — 启动 VM
- `POST /api/vms/{id}/migrate` — 迁移 VM（payload 包含目标 node_id）
- `GET /api/tasks/{id}` — 查询任务状态

**迁移流程（冷迁/热迁）示意**：
1. 前端发起迁移请求 -> 后端校验权限 & 资源
2. 后端写入 `tasks` 表并入队
3. worker 消费任务，向 source Agent 发起迁移请求（RPC stream）
4. source Agent 准备（快照或流式复制），与 target Agent 建立直接传输流（可能通过后端协调或直接 P2P）
5. 迁移进度上报 -> 后端更新任务
6. 完成 -> 更新 `vms.node_id` 和状态

---

## 6 网络模型

支持两种主流模型：
- Linux Bridge：简单、兼容性好，适用于小型部署
- Open vSwitch (OVS)：更强功能，支持 VLAN、VXLAN、SDN 控制器对接

**IPAM**：内置简单 IP 池管理（基于 `networks` 表与 `ip_allocations`），并支持与外部 SDN 控制器对接。

**SDN 对接**：提供插件接口（Webhook / RPC）以对接外部 SDN（如 ODL、ONOS、OVN）。

### Linux Bridge

通过配置文件指定的网卡作为网络的provider。使用vlan进行隔离，相同vlan作为同一个网络。

例如： vm创建（携带vlan100网络） -> Agent 检查是否存在代表vlan100的网桥，provider网卡的vlan100子接口是否在网桥中，将vm的网口插入到vlan100的网桥中。

### Open vSwitch (OVS)

略

---

## 7 存储模型

支持多种后端存储：
- 本地 LVM + QCOW2
- NFS
- Ceph RBD
- iSCSI

**存储抽象层**：实现驱动接口（trait）来适配不同存储后端。驱动负责创建卷、删除卷、克隆、快照、扩容、导入导出。

**数据平面**：实际数据操作由 Node Agent 执行，后端只下发指令并维护元数据。

### NFS

例如： volume创建 -> 将任务放入队列 -> worker 调用 Agent 的 RPC 接口 -> Agent 在对应的nfs目录中创建volume

### Ceph RBD

例如： volume创建 -> 将任务放入队列 -> worker 调用 Agent 的 RPC 接口 -> Agent 调用ceph rbd接口创建volume


---

## 8 可观测性与监控

- 指标：Agent 暴露 Prometheus `/metrics`；后端与 worker 也暴露指标
- 日志：后端与 Agent 使用 structured logging（JSON）并收集到 Loki
- 仪表盘：Grafana（集群资源面板、任务面板、历史趋势）
- 报警：基于 Alertmanager 配置阈值报警（节点离线、任务失败率、资源过载）

---

## 9 安全与合规

- 所有控制面通信使用 TLS / mTLS
- API 使用 JWT
- 审计日志记录所有管理员操作
- 最小权限原则（RBAC）
- 对敏感信息加密存储（例如：secret、证书）
- 定期备份 PostgreSQL、密钥材料与重要元数据



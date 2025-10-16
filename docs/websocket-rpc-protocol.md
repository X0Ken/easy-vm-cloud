# WebSocket RPC 协议设计

## 概述

本文档定义了 Server 与 Agent 之间通过 WebSocket 进行双向 RPC 调用的协议规范。

### 设计目标

- 支持双向 RPC 调用（Server ↔ Agent）
- 支持流式数据传输（如迁移进度）
- 简单易实现，基于 JSON 格式
- 支持请求/响应模式和通知模式
- 支持异步超时和错误处理
- 支持连接管理和自动重连

### 连接模型

```
Agent (Client) --[WebSocket]--> Server (WebSocket Server)
    |                                      |
    |<---- RPC Request/Response ---------->|
    |<---- Notification ------------------>|
    |<---- Stream Data ------------------->|
```

- Agent 主动连接到 Server
- Server 维护所有 Agent 连接列表（通过 `AgentConnectionManager`）
- 连接建立后，双方都可以发起 RPC 调用
- 支持心跳保活机制
- 支持自动重连（Agent 断线后每 5 秒重连）

## 消息格式

所有消息使用 JSON 格式，通过 WebSocket Text Frame 传输。

### 基础消息结构

```json
{
  "id": "string",           // 消息唯一ID（UUID格式，如 req-xxx, notif-xxx）
  "type": "string",         // 消息类型: request | response | notification | stream
  "method": "string",       // RPC 方法名（type=request/notification时必需）
  "payload": {},            // 消息负载（JSON对象）
  "error": null | {}        // 错误信息（仅response时可能有值）
}
```

**消息ID格式：**
- 请求消息：`req-{uuid}`
- 通知消息：`notif-{uuid}`
- 响应消息：与对应请求的ID相同
- 流式消息：与原始请求的ID相同

### 消息类型

#### Request（请求）

发起 RPC 调用，期望得到响应。

```json
{
  "id": "req-123e4567-e89b-12d3-a456-426614174000",
  "type": "request",
  "method": "create_vm",
  "payload": {
    "vm_id": "vm-001",
    "name": "test-vm",
    "vcpu": 2,
    "memory_mb": 2048
  }
}
```

#### Response（响应）

对 Request 的响应，`id` 必须与对应的 Request 相同。

成功响应：
```json
{
  "id": "req-123e4567-e89b-12d3-a456-426614174000",
  "type": "response",
  "payload": {
    "success": true,
    "vm_uuid": "uuid-..."
  },
  "error": null
}
```

失败响应：
```json
{
  "id": "req-123e4567-e89b-12d3-a456-426614174000",
  "type": "response",
  "payload": null,
  "error": {
    "code": "VM_CREATE_FAILED",
    "message": "创建虚拟机失败: 磁盘空间不足"
  }
}
```

#### Notification（通知）

单向消息，不需要响应。用于心跳、状态上报等。

```json
{
  "id": "notif-123e4567-e89b-12d3-a456-426614174000",
  "type": "notification",
  "method": "heartbeat",
  "payload": {
    "node_id": "node-001",
    "timestamp": 1234567890,
    "status": "healthy"
  }
}
```

#### Stream（流式数据）

用于传输流式数据，多个 stream 消息共享同一个 `id`（对应原始 request 的 id）。

```json
{
  "id": "req-123e4567-e89b-12d3-a456-426614174000",
  "type": "stream",
  "payload": {
    "stage": "transferring",
    "progress": 45.5,
    "message": "正在迁移虚拟机..."
  }
}
```

最后一个流消息带有 `completed` 标志：
```json
{
  "id": "req-123e4567-e89b-12d3-a456-426614174000",
  "type": "stream",
  "payload": {
    "stage": "completed",
    "progress": 100.0,
    "completed": true
  }
}
```

## 连接生命周期

### 连接建立

1. Agent 连接到 Server 的 WebSocket 端点：`ws://server:port/ws/agent`
2. 连接建立后，Agent 发送注册消息：
   ```json
   {
     "id": "reg-...",
     "type": "request",
     "method": "register",
     "payload": {
       "node_id": "node-001",
       "hostname": "host1",
       "ip_address": "192.168.1.100"
     }
   }
   ```
3. Server 响应注册结果并记录连接

### 心跳机制

- Agent 每 30 秒发送一次心跳通知
- Server 超过 90 秒未收到心跳，标记节点为离线
- Agent 可以通过心跳响应获取 Server 时间

### 连接关闭

- Agent 主动断开：发送 `unregister` 通知后关闭
- Server 主动断开：通知 Agent 后关闭连接
- 异常断开：Server 标记节点离线，等待重连

### 重连机制

- Agent 断线后每 5 秒尝试重连
- 重连成功后重新发送注册消息
- Server 更新节点状态为在线

## 错误处理

### 错误码示例

```rust
pub enum RpcErrorCode {
    // 通用错误
    InvalidRequest,      // 请求格式错误
    MethodNotFound,      // 方法不存在
}
```

### 错误响应格式

```json
{
  "code": "VM_NOT_FOUND",
  "message": "虚拟机不存在: vm-001",
  "details": {
    "vm_id": "vm-001"
  }
}
```

## 实现架构

### 核心组件

#### Common 模块 (`crates/common/src/ws_rpc/`)
- `message.rs` - RPC 消息定义和序列化
- `types.rs` - 所有 RPC 请求/响应类型定义
- `error.rs` - 错误码和错误处理
- `server.rs` - RPC 路由器（`RpcRouter`）
- `client.rs` - RPC 客户端连接管理（`WsRpcConnection`）

#### Agent 模块 (`crates/agent/src/ws/`)
- `client.rs` - WebSocket 客户端（`WsClient`）
- `handler.rs` - RPC 处理器注册表（`RpcHandlerRegistry`）

#### Server 模块 (`crates/server/src/ws/`)
- `agent_manager.rs` - Agent 连接管理器（`AgentConnectionManager`）
- `handler.rs` - WebSocket 连接处理器

### 连接管理

#### Agent 端
- `WsClient` 负责连接到 Server
- 支持自动重连（默认 5 秒间隔）
- 心跳机制（默认 30 秒间隔）
- 状态管理：`Disconnected` → `Connecting` → `Connected` → `Registered`

#### Server 端
- `AgentConnectionManager` 管理所有 Agent 连接
- 每个连接包含：节点信息、发送通道、心跳时间、待响应请求
- 支持并发处理多个 Agent 连接

## 安全性

### 认证

- 使用 TLS/WSS 加密传输（生产环境必需）
- Agent 连接时携带认证令牌（在 URL 参数或首次握手消息中）
- Server 验证令牌有效性后允许注册

### 授权

- Server 维护节点授权列表
- 只有授权的节点才能注册和调用敏感操作

## 性能考虑

### 并发处理

- Server 使用异步 I/O 处理多个 Agent 连接
- 每个连接独立的消息队列
- 支持并发处理多个 RPC 请求
- 使用 `tokio::sync::RwLock` 进行线程安全的并发访问

### 超时设置

- 默认 RPC 请求超时：30 秒
- 长时间操作（如迁移）：300 秒
- 心跳间隔：30 秒
- 心跳超时：90 秒
- Agent 重连间隔：5 秒

### 消息大小限制

- 单个消息最大：10MB
- 建议大文件传输使用专门的传输通道

### 内存管理

- 使用 `Arc<RwLock<HashMap>>` 管理待响应请求
- 连接断开时自动清理待处理请求
- 支持连接池和资源复用

## 实现示例

### 创建虚拟机流程

1. Server 发送请求：
   ```json
   {
     "id": "req-001",
     "type": "request",
     "method": "create_vm",
     "payload": { ... }
   }
   ```

2. Agent 处理并响应：
   ```json
   {
     "id": "req-001",
     "type": "response",
     "payload": {
       "success": true,
       "vm_uuid": "..."
     },
     "error": null
   }
   ```
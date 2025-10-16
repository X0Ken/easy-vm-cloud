# 项目结构说明

本文档描述了 Easy VM Cloud 项目的核心目录结构和文件组织方式。

## 📁 核心项目结构

```
easy-vm-cloud/                      # 项目根目录
├── README.md                       # 项目主文档
├── Cargo.toml                     # Rust Workspace 配置
│
├── docs/                          # 项目文档
│
├── crates/                        # Rust 核心模块
│   │
│   ├── common/                    # 公共库
│   │   ├── src/
│   │   │   ├── models/           # 共享数据模型
│   │   │   ├── ws_rpc/           # WebSocket RPC 框架
│   │   │   ├── errors.rs         # 错误处理
│   │   │   └── utils/            # 工具函数
│   │   └── Cargo.toml
│   │
│   ├── server/                    # 后端服务器
│   │   ├── src/
│   │   │   ├── main.rs           # 服务器入口
│   │   │   ├── api/              # REST API 接口目录
│   │   │   ├── services/         # 业务逻辑层
│   │   │   ├── ws/               # WebSocket 服务
│   │   │   ├── db/               # 数据库访问层
│   │   │   ├── auth/             # 认证授权
│   │   │   ├── config.rs         # 配置管理
│   │   │   ├── app_state.rs      # 应用状态
│   │   │   ├── middleware.rs     # 中间件
│   │   │   ├── extractors.rs     # 提取器
│   │   │   └── utils.rs          # 工具函数
│   │   └── Cargo.toml
│   │
│   └── agent/                     # 节点代理
│       ├── src/
│       │   ├── main.rs           # Agent 入口
│       │   ├── ws/               # WebSocket 客户端
│       │   │   ├── client.rs     # WebSocket 客户端
│       │   │   └── handler.rs    # RPC 请求处理器
│       │   ├── hypervisor/       # 虚拟化管理
│       │   │   └── manager.rs    # 虚拟化管理器
│       │   ├── storage/          # 存储管理
│       │   ├── network/          # 网络管理
│       │   ├── metrics/          # 指标收集
│       │   └── config.rs         # 配置管理
│       └── Cargo.toml
│
└── frontend/                      # Angular 前端
    ├── src/
    │   ├── app/
    │   │   ├── pages/            # 页面组件
    │   │   │   ├── login/        # 登录页面
    │   │   │   ├── vms/          # 虚拟机管理
    │   │   │   ├── network/      # 网络管理
    │   │   │   ├── storage/      # 存储管理
    │   │   │   ├── nodes/        # 节点管理
    │   │   │   ├── system/       # 系统管理
    │   │   │   ├── permissions/  # 权限管理
    │   │   │   ├── me/           # 个人中心
    │   │   │   └── welcome/      # 欢迎页面
    │   │   ├── services/         # 服务层
    │   │   ├── guards/           # 路由守卫
    │   │   ├── interceptors/     # HTTP 拦截器
    │   │   ├── models/           # 数据模型
    │   │   ├── config/           # 配置模块
    │   │   ├── shared/           # 共享模块
    │   │   ├── app.ts            # 根组件
    │   │   ├── app.routes.ts     # 路由配置
    │   │   ├── app.config.ts     # 应用配置
    │   │   ├── app.html          # 根模板
    │   │   ├── app.scss          # 根样式
    │   │   ├── app.spec.ts       # 根组件测试
    │   │   └── icons-provider.ts # 图标提供者
    │   ├── assets/               # 静态资源
    │   ├── environments/         # 环境配置
    │   └── main.ts               # 应用入口
    └── package.json
```


## 🚀 核心功能模块

### 1. Common 公共库

- 统一的错误处理 (`Error` 和 `Result` 类型)
- 共享数据模型 (节点状态、VM 状态、任务类型等)
- WebSocket RPC 框架（消息、错误、类型定义）
- 客户端和服务端辅助工具
- 工具函数 (ID 生成、格式化、验证等)

### 2. Server 后端

**REST API 接口目录**:
- `api/auth.rs` - 认证相关接口
- `api/nodes.rs` - 节点管理接口
- `api/vms.rs` - 虚拟机管理接口
- `api/storage.rs` - 存储管理接口
- `api/networks.rs` - 网络管理接口
- `api/user.rs` - 用户管理接口
- `api/role.rs` - 角色管理接口
- `api/permission.rs` - 权限管理接口
- `api/department.rs` - 部门管理接口

**WebSocket 服务**:
- `ws/` - WebSocket 服务目录
- 管理所有 Agent 的 WebSocket 连接
- 心跳监控和超时检测

**业务逻辑层**:
- `services/node_service.rs` - 节点管理服务
- `services/vm_service.rs` - 虚拟机管理服务
- `services/storage_service.rs` - 存储管理服务
- `services/network_service.rs` - 网络管理服务
- `services/user_service.rs` - 用户管理服务
- `services/task_service.rs` - 任务管理服务

### 3. Agent 节点代理

**WebSocket 客户端**:
- `ws/client.rs` - WebSocket 客户端
- `ws/handler.rs` - RPC 请求处理器
- 连接到 Server 的 `/ws/agent` 端点
- 自动注册和心跳上报
- 断线自动重连（5秒间隔）

**组件模块**:
- `hypervisor/manager.rs` - 虚拟化管理器
- `storage/` - 存储管理目录
- `network/` - 网络管理目录
- `metrics/` - 指标收集目录
- `ws/` - WebSocket 客户端目录

### 4. Frontend 前端

**核心页面目录**:
- `pages/login/` - 登录页面
- `pages/vms/` - 虚拟机管理
- `pages/network/` - 网络管理
- `pages/storage/` - 存储管理
- `pages/nodes/` - 节点管理
- `pages/system/` - 系统管理
- `pages/permissions/` - 权限管理
- `pages/me/` - 个人中心
- `pages/welcome/` - 欢迎页面

**核心服务目录**:
- `services/` - 服务层目录
- `guards/` - 路由守卫目录
- `interceptors/` - HTTP 拦截器目录
- `models/` - 数据模型目录
- `config/` - 配置模块目录
- `shared/` - 共享模块目录

## 📝 架构说明

### Rust Workspace 结构

项目使用 Cargo Workspace 管理多个相关的 Rust 项目：

- **`crates/common/`**: 共享库，包含通用数据结构、WebSocket RPC 框架、错误处理等
- **`crates/server/`**: 后端服务器，提供 REST API，管理业务逻辑
- **`crates/agent/`**: 节点代理，运行在宿主机上，执行实际的虚拟化操作

### 通信架构

- **前端 ↔ Server**: HTTPS/REST JSON API
- **Server ↔ Agent**: WebSocket（双向 RPC，JSON 消息格式）
  - Agent 主动连接到 Server
  - Server 维护所有在线 Agent 的连接
  - 支持请求/响应、通知和流式数据传输
  - 心跳机制：30秒间隔，90秒超时
- **Server ↔ PostgreSQL**: 数据持久化
- **Agent ↔ 虚拟化栈**: 本地调用（libvirt、OVS、LVM 等）

### WebSocket RPC 协议

详细协议规范请参考 `docs/websocket-rpc-protocol.md`，包括：
- 消息格式定义
- RPC 方法列表
- 连接生命周期
- 错误处理
- 安全性考虑

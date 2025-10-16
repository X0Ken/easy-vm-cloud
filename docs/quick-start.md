# Easy VM Cloud 快速开始指南

本指南将帮助您快速部署和运行 Easy VM Cloud 虚拟机管理系统。

## 环境要求

### 后端 (Server + Agent)
- Rust 1.70+
- PostgreSQL 12+
- Cargo
- libvirt-dev (Agent 需要，用于虚拟化管理)

### 前端
- Node.js 18+
- npm 或 yarn
- Angular CLI

## 快速部署步骤

### 1. 克隆项目

```bash
git clone <repository-url>
cd easy-vm-cloud
```

### 2. 环境配置

复制环境变量模板并配置：

```bash
cp env.example .env
```

编辑 `.env` 文件，配置数据库连接等参数：

```bash
# 数据库配置
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=easy_vm_cloud
POSTGRES_USER=postgres
POSTGRES_PASSWORD=your_password

# API配置
API_BASE_URL=http://localhost:3000
```

### 3. 数据库初始化

使用提供的脚本初始化数据库：

```bash
# 确保PostgreSQL服务运行
sudo systemctl start postgresql

# 执行数据库初始化脚本
chmod +x scripts/createdb.sh
./scripts/createdb.sh
```

该脚本会：
- 创建数据库（如果不存在）
- 执行数据库迁移脚本
- 初始化基础数据

### 4. 构建后端服务

#### 构建 Server (API服务)

```bash
# 进入server目录
cd crates/server

# 构建并运行
cargo build --release
cargo run --release
```

Server 将在 `http://localhost:3000` 启动。

#### 构建 Agent (节点代理)

```bash
# 进入agent目录
cd crates/agent

# 构建
cargo build --release

# 运行 (需要root权限，因为需要访问libvirt)
sudo ./target/release/agent
```

### 5. 构建前端

```bash
# 安装依赖
npm install

# 开发模式运行
npm start

# 或构建生产版本
npm run build
```

前端将在 `http://localhost:4200` 启动。

### 6. API 测试

项目提供了完整的API测试脚本：

```bash
# 给脚本执行权限
chmod +x scripts/apiclient.sh

# 登录系统
./scripts/apiclient.sh login --u admin -p admin123

# 查看所有可用命令
./scripts/apiclient.sh --help

# 测试基本功能
./scripts/apiclient.sh user list
./scripts/apiclient.sh node list
./scripts/apiclient.sh vm list
```

### 7. Docker 部署 (可选)

项目支持Docker部署：

```bash
# 使用docker-compose启动所有服务
docker-compose up -d

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f
```

## 默认账户

系统初始化后会创建默认超级管理员账户：

- **用户名**: admin
- **密码**: admin123
- **角色**: 超级管理员

## 访问系统

部署完成后，您可以通过以下方式访问系统：

- **前端界面**: http://localhost:4200
- **API服务**: http://localhost:3000

## 故障排除

### 常见问题

1. **数据库连接失败**
   - 检查PostgreSQL服务是否运行
   - 验证 `.env` 文件中的数据库配置

2. **Agent启动失败**
   - 确保以root权限运行
   - 检查libvirt服务是否运行
   - 验证libvirt-dev包是否安装

3. **前端构建失败**
   - 检查Node.js版本是否符合要求
   - 清除node_modules重新安装依赖

4. **API测试失败**
   - 确保Server服务正在运行
   - 检查API_BASE_URL配置是否正确

### 获取帮助

- 查看项目文档目录 `docs/`
- 提交 Issue 获取技术支持
- 参考API测试脚本了解具体功能用法

#!/bin/bash

# Easy VM Cloud 安装部署脚本
# 从 GitHub 下载最新版本并部署到本地

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置变量
GITHUB_REPO="x0ken/easy-vm-cloud"
INSTALL_DIR="/opt/easy-vm-cloud"
NODE_ID=""

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否为 root 用户
check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "请使用 root 权限运行此脚本"
        log_info "使用方法: sudo $0"
        exit 1
    fi
}

# 检查系统要求
check_system_requirements() {
    log_info "检查系统要求..."
    
    # 检查操作系统
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        log_info "检测到操作系统: $PRETTY_NAME"
    else
        log_warning "无法检测操作系统版本"
    fi
    
    # 检查必要工具
    local missing_tools=()
    
    if ! command -v curl &> /dev/null; then
        missing_tools+=("curl")
    fi
    
    if ! command -v wget &> /dev/null; then
        missing_tools+=("wget")
    fi
    
    if ! command -v tar &> /dev/null; then
        missing_tools+=("tar")
    fi
    
    if ! command -v systemctl &> /dev/null; then
        log_error "systemctl 未找到，此脚本需要 systemd 系统"
        exit 1
    fi
    
    if [ ${#missing_tools[@]} -ne 0 ]; then
        log_error "缺少必要工具: ${missing_tools[*]}"
        log_info "请先安装缺少的工具"
        exit 1
    fi
    
    log_success "系统要求检查完成"
}


# 检查 Docker
check_docker() {
    log_info "检查 Docker..."
    
    # 检查 Docker 是否已安装
    if ! command -v docker &> /dev/null; then
        log_error "Docker 未安装，请先安装 Docker"
        log_info "安装命令: curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh"
        log_info "或者: apt install docker.io"
        exit 1
    fi
    
    # 检查 Docker 服务是否运行
    if ! systemctl is-active --quiet docker; then
        log_error "Docker 服务未运行，请启动 Docker 服务"
        log_info "启动命令: systemctl start docker"
        log_info "启用命令: systemctl enable docker"
        exit 1
    fi
    
    log_success "Docker 检查完成"
}

# 安装和配置 Docker PostgreSQL
install_docker_postgresql() {
    log_info "安装和配置 Docker PostgreSQL..."
    
    # 检查 PostgreSQL 容器是否已存在
    if docker ps -a --format "table {{.Names}}" | grep -q "easy-vm-postgres"; then
        log_info "PostgreSQL 容器已存在"
        
        # 检查容器是否运行
        if docker ps --format "table {{.Names}}" | grep -q "easy-vm-postgres"; then
            log_info "PostgreSQL 容器正在运行"
        else
            log_info "启动 PostgreSQL 容器..."
            docker start easy-vm-postgres
        fi
    else
        log_info "创建 PostgreSQL 容器..."
        
        # 创建数据卷
        docker volume create easy-vm-postgres-data
        
        # 运行 PostgreSQL 容器
        docker run -d \
            --name easy-vm-postgres \
            --restart unless-stopped \
            -e POSTGRES_DB=easy_vm_cloud \
            -e POSTGRES_USER=easyvm \
            -e POSTGRES_PASSWORD=easyvm123 \
            -p 5432:5432 \
            -v easy-vm-postgres-data:/var/lib/postgresql/data \
            -v "$INSTALL_DIR/database/migrations:/docker-entrypoint-initdb.d:ro" \
            postgres:15-alpine
        
        # 等待容器启动
        log_info "等待 PostgreSQL 容器启动..."
        sleep 10
    fi
    
    # 检查容器状态
    if ! docker ps --format "table {{.Names}}" | grep -q "easy-vm-postgres"; then
        log_error "PostgreSQL 容器启动失败"
        docker logs easy-vm-postgres
        exit 1
    fi
    
    # 等待数据库就绪
    log_info "等待数据库就绪..."
    for i in {1..30}; do
        if docker exec easy-vm-postgres pg_isready -U easyvm -d easy_vm_cloud &> /dev/null; then
            log_success "PostgreSQL 数据库就绪"
            break
        fi
        sleep 2
    done
    
    log_success "Docker PostgreSQL 配置完成"
    log_info "数据库迁移文件已通过 Docker 自动导入"
}



# 下载并解压项目
download_and_extract() {
    log_info "下载 Easy VM Cloud 最新版本..."
    
    # 创建安装目录
    mkdir -p "$INSTALL_DIR"
    
    # 获取最新版本号
    log_info "获取最新版本信息..."
    local latest_release
    latest_release=$(curl -s "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | grep "tag_name" | cut -d '"' -f 4)
    
    if [ -z "$latest_release" ]; then
        log_error "无法获取最新版本信息，请检查网络连接或仓库地址"
        log_info "当前配置的仓库: $GITHUB_REPO"
        exit 1
    fi
    
    log_info "最新版本: $latest_release"
    
    # 构建下载URL
    local download_url="https://github.com/$GITHUB_REPO/releases/download/$latest_release/easy-vm-cloud-$latest_release.tar.gz"
    local temp_file="/tmp/easy-vm-cloud-$latest_release.tar.gz"
    
    # 下载文件
    log_info "正在下载: $download_url"
    if ! curl -L -o "$temp_file" "$download_url"; then
        log_error "下载失败，请检查网络连接"
        exit 1
    fi
    
    # 检查下载的文件
    if [ ! -f "$temp_file" ] || [ ! -s "$temp_file" ]; then
        log_error "下载的文件无效或为空"
        exit 1
    fi
    
    # 解压到安装目录
    log_info "解压项目文件..."
    if ! tar -xzf "$temp_file" -C "$INSTALL_DIR"; then
        log_error "解压失败"
        exit 1
    fi
    
    # 移动解压后的文件到正确位置
    local extracted_dir="$INSTALL_DIR/easy-vm-cloud-$latest_release"
    if [ -d "$extracted_dir" ]; then
        # 将解压后的内容移动到安装目录
        mv "$extracted_dir"/* "$INSTALL_DIR/"
        rmdir "$extracted_dir"
    fi
    
    # 设置权限
    if [ -f "$INSTALL_DIR/bin/server" ]; then
        chmod +x "$INSTALL_DIR/bin/server"
    fi
    if [ -f "$INSTALL_DIR/bin/agent" ]; then
        chmod +x "$INSTALL_DIR/bin/agent"
    fi
    
    # 清理临时文件
    rm -f "$temp_file"
    
    log_success "项目文件安装完成 (版本: $latest_release)"
}

# 创建 Caddy 配置文件
create_caddy_config() {
    log_info "创建 Caddy 配置文件..."
    
    # 创建 Caddyfile
    cat > "$INSTALL_DIR/Caddyfile" << EOF
# Easy VM Cloud Caddy 配置

# 前端静态文件服务
:8080 {
    root * /srv
    file_server
    
    # API 代理到后端
    handle /api/* {
        reverse_proxy localhost:3000
    }
    
    # WebSocket 代理
    handle /ws/* {
        reverse_proxy localhost:3000
    }
    
    # 健康检查
    handle /health {
        respond "OK" 200
    }
    
    # 日志
    log {
        output stdout
    }
}
EOF
    
    log_success "Caddy 配置文件创建完成"
}

# 启动 Caddy 容器
start_caddy_containers() {
    log_info "启动 Caddy 容器..."
    
    # 启动 Caddy 前端容器
    log_info "启动 Caddy 前端容器..."
    docker run -d \
        --name easy-vm-frontend \
        --restart unless-stopped \
        --network host \
        -v "$INSTALL_DIR/frontend/browser:/srv" \
        -v "$INSTALL_DIR/Caddyfile:/etc/caddy/Caddyfile" \
        caddy:alpine
    
    # 检查前端容器状态
    if docker ps --format "table {{.Names}}" | grep -q "easy-vm-frontend"; then
        log_success "Caddy 前端容器启动成功"
    else
        log_error "Caddy 前端容器启动失败"
        docker logs easy-vm-frontend
        exit 1
    fi
    
    log_success "Caddy 容器启动完成"
}

# 启动 Agent 服务
start_agent_service() {
    log_info "启动 Agent 服务..."
    systemctl enable easy-vm-cloud-agent
    systemctl start easy-vm-cloud-agent
    
    # 等待 Agent 服务启动
    sleep 3
    
    # 检查 Agent 服务状态
    if systemctl is-active --quiet easy-vm-cloud-agent; then
        log_success "Easy VM Cloud Agent 启动成功"
    else
        log_warning "Easy VM Cloud Agent 启动失败，但其他服务正常运行"
    fi
    
    log_success "systemd 服务启动完成"
}


# 创建 systemd 服务
create_systemd_services() {
    log_info "创建 systemd 服务..."
    
    # 创建后端服务
    cat > /etc/systemd/system/easy-vm-cloud-server.service << EOF
[Unit]
Description=Easy VM Cloud Server
After=network.target docker.service
Wants=docker.service

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/bin/server
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=DATABASE_URL=postgresql://easyvm:easyvm123@localhost:5432/easy_vm_cloud
Environment=JWT_SECRET=your-jwt-secret-key-change-in-production
Environment=JWT_EXPIRATION=24h

[Install]
WantedBy=multi-user.target
EOF


    # 生成随机节点ID
    NODE_ID=$(uuidgen)
    log_info "生成随机节点ID: $NODE_ID"
    log_success "节点ID已设置到systemd服务配置中"
    
    # 创建 Agent 服务（可选）
    cat > /etc/systemd/system/easy-vm-cloud-agent.service << EOF
[Unit]
Description=Easy VM Cloud Agent
After=network.target easy-vm-cloud-server.service
Wants=easy-vm-cloud-server.service

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/bin/agent
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=SERVER_WS_URL=ws://localhost:3000/ws/agent
Environment=NODE_ID=$NODE_ID
Environment=NODE_NAME=$(hostname)

[Install]
WantedBy=multi-user.target
EOF
    
    # 重新加载 systemd
    systemctl daemon-reload
    
    log_success "systemd 服务创建完成"
}

# 启动后端服务
start_server_service() {
    log_info "启动 Server 服务..."
    
    # 启动并启用 Server 服务
    systemctl enable easy-vm-cloud-server
    systemctl start easy-vm-cloud-server
    
    # 等待 Server 服务启动
    sleep 5
    
    # 检查 Server 服务状态
    if systemctl is-active --quiet easy-vm-cloud-server; then
        log_success "Easy VM Cloud Server 启动成功"
    else
        log_error "Easy VM Cloud Server 启动失败"
        systemctl status easy-vm-cloud-server
        exit 1
    fi
}

# 启动服务
start_services() {
    log_info "启动所有服务..."
    
    # 启动 Server 服务
    start_server_service
    
    # 启动 Caddy 容器
    start_caddy_containers
    
    # 启动 Agent 服务
    start_agent_service
    
    log_success "所有服务启动完成"
}

# 显示安装信息
show_installation_info() {
    echo ""
    echo "=========================================="
    echo "Easy VM Cloud 安装完成！"
    echo "=========================================="
    echo ""
    echo "安装目录: $INSTALL_DIR"
    echo ""
    echo "服务状态:"
    echo "- Easy VM Cloud Server: $(systemctl is-active easy-vm-cloud-server)"
    echo "- Easy VM Cloud Agent: $(systemctl is-active easy-vm-cloud-agent)"
    echo "- Easy VM Cloud Frontend (Caddy): $(docker ps --format 'table {{.Names}}' | grep -q easy-vm-frontend && echo 'running' || echo 'stopped')"
    echo "- PostgreSQL: $(docker ps --format 'table {{.Names}}' | grep -q easy-vm-postgres && echo 'running' || echo 'stopped')"
    echo ""
    echo "节点信息:"
    echo "- 节点ID: $NODE_ID"
    echo "- 节点名称: $(hostname)"
    echo ""
    echo "访问地址:"
    echo "- 前端界面: http://localhost:8080"
    echo ""
}

# 主函数
main() {
    log_info "开始安装 Easy VM Cloud..."
    
    check_root
    check_system_requirements
    check_docker
    download_and_extract
    create_caddy_config
    install_docker_postgresql
    create_systemd_services
    start_services
    show_installation_info
    
    log_success "安装完成！"
}

# 显示帮助信息
show_help() {
    echo "Easy VM Cloud 安装脚本"
    echo ""
    echo "使用方法:"
    echo "  $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help     显示此帮助信息"
    echo "  --install-dir  指定安装目录 (默认: $INSTALL_DIR)"
    echo ""
    echo "示例:"
    echo "  $0"
    echo "  $0 --install-dir /opt/custom"
    echo ""
}

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        *)
            log_error "未知参数: $1"
            show_help
            exit 1
            ;;
    esac
done

# 执行主函数
main "$@"

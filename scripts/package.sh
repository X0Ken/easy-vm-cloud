#!/bin/bash

# Easy VM Cloud 项目打包脚本
# 用于打包前端、server 和 agent 为一个压缩包

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# 获取项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PACKAGE_DIR="$PROJECT_ROOT/dist"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
PACKAGE_NAME="easy-vm-cloud-${TIMESTAMP}.tar.gz"

log_info "开始打包 Easy VM Cloud 项目..."
log_info "项目根目录: $PROJECT_ROOT"

# 清理旧的打包目录
if [ -d "$PACKAGE_DIR" ]; then
    log_info "清理旧的打包目录..."
    rm -rf "$PACKAGE_DIR"
fi

# 创建打包目录
mkdir -p "$PACKAGE_DIR"

# 检查必要工具
check_dependencies() {
    log_info "检查依赖工具..."
    
    # 检查 Node.js 和 npm
    if ! command -v node &> /dev/null; then
        log_error "Node.js 未安装，请先安装 Node.js"
        exit 1
    fi
    
    # 检查 Rust 和 Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo 未安装，请先安装 Rust"
        exit 1
    fi
    
    # 检查 tar 命令
    if ! command -v tar &> /dev/null; then
        log_error "tar 命令未找到"
        exit 1
    fi
    
    log_success "依赖检查完成"
}

# 构建前端
build_frontend() {
    log_info "构建前端项目..."
    
    cd "$PROJECT_ROOT/frontend"
    
    # 检查 package.json 是否存在
    if [ ! -f "package.json" ]; then
        log_error "前端 package.json 未找到"
        exit 1
    fi
    
    # 安装依赖
    log_info "安装前端依赖..."
    npm install
    
    # 构建生产版本
    log_info "构建前端生产版本..."
    npm run build
    
    # 复制构建结果到打包目录
    if [ -d "dist" ]; then
        cp -r dist/* "$PACKAGE_DIR/frontend/"
        log_success "前端构建完成"
    else
        log_error "前端构建失败，dist 目录不存在"
        exit 1
    fi
}

# 构建后端服务
build_backend() {
    log_info "构建后端服务..."
    
    cd "$PROJECT_ROOT"
    
    # 构建 server
    log_info "构建 server..."
    cargo build --release --bin server
    
    # 构建 agent
    log_info "构建 agent..."
    cargo build --release --bin agent
    
    # 复制可执行文件到打包目录
    mkdir -p "$PACKAGE_DIR/bin"
    
    # 查找可执行文件位置
    SERVER_BINARY=$(find target/release -name "server" -type f -executable 2>/dev/null | head -1)
    AGENT_BINARY=$(find target/release -name "agent" -type f -executable 2>/dev/null | head -1)
    
    if [ -n "$SERVER_BINARY" ] && [ -f "$SERVER_BINARY" ]; then
        cp "$SERVER_BINARY" "$PACKAGE_DIR/bin/"
        log_success "Server 二进制文件已复制"
    else
        log_error "Server 二进制文件未找到"
        exit 1
    fi
    
    if [ -n "$AGENT_BINARY" ] && [ -f "$AGENT_BINARY" ]; then
        cp "$AGENT_BINARY" "$PACKAGE_DIR/bin/"
        log_success "Agent 二进制文件已复制"
    else
        log_error "Agent 二进制文件未找到"
        exit 1
    fi
}

# 复制数据库相关文件
copy_database_files() {
    log_info "复制数据库相关文件..."
    
    # 创建数据库目录
    mkdir -p "$PACKAGE_DIR/database"
    
    # 复制SQL迁移文件
    if [ -d "$PROJECT_ROOT/crates/server/migrations" ]; then
        mkdir -p "$PACKAGE_DIR/database/migrations"
        cp -r "$PROJECT_ROOT/crates/server/migrations"/* "$PACKAGE_DIR/database/migrations/"
        log_success "SQL迁移文件已复制"
    else
        log_warning "SQL迁移文件目录未找到"
    fi
    
    log_success "数据库文件复制完成"
}

# 创建压缩包
create_archive() {
    log_info "创建压缩包..."
    
    cd "$PROJECT_ROOT"
    
    # 创建压缩包
    tar -czf "$PACKAGE_NAME" -C dist .
    
    # 获取文件大小
    PACKAGE_SIZE=$(du -h "$PACKAGE_NAME" | cut -f1)
    
    log_success "压缩包创建完成: $PACKAGE_NAME"
    log_success "文件大小: $PACKAGE_SIZE"
    log_success "压缩包位置: $PROJECT_ROOT/$PACKAGE_NAME"
}

# 清理临时文件
cleanup() {
    log_info "清理临时文件..."
    rm -rf "$PACKAGE_DIR"
    log_success "清理完成"
}

# 显示使用说明
show_usage() {
    echo ""
    echo "=========================================="
    echo "Easy VM Cloud 打包完成！"
    echo "=========================================="
    echo ""
    echo "压缩包文件: $PACKAGE_NAME"
    echo "文件大小: $PACKAGE_SIZE"
    echo ""
}

# 主函数
main() {
    log_info "开始打包 Easy VM Cloud 项目..."
    
    check_dependencies
    build_frontend
    build_backend
    copy_database_files
    create_archive
    cleanup
    show_usage
    
    log_success "打包完成！"
}

# 执行主函数
main "$@"

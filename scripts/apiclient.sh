#!/bin/bash

# Easy VM Cloud API 测试脚本
# 用法: ./api.sh login --u <username> -p <password>

# 默认配置
API_BASE_URL="${API_BASE_URL:-http://localhost:3000}"
TOKEN_FILE=".api_token"

# 颜色输出
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 打印帮助信息
print_help() {
    echo "用法: ./api.sh <command> [options]"
    echo ""
    echo "命令:"
    echo "  login           用户登录"
    echo "  user list       列出所有用户"
    echo "  vm list         列出所有虚拟机"
    echo "  vm create       创建虚拟机"
    echo "  vm get          获取虚拟机详情"
    echo "  vm update       更新虚拟机"
    echo "  vm delete       删除虚拟机"
    echo "  vm start        启动虚拟机"
    echo "  vm stop         停止虚拟机"
    echo "  vm restart      重启虚拟机"
    echo "  vm migrate      迁移虚拟机"
    echo "  vm attach       挂载存储卷"
    echo "  vm detach       卸载存储卷"
    echo "  vm volumes      列出虚拟机存储卷"
    echo "  node list       列出所有节点"
    echo "  node create     创建节点"
    echo "  node get        获取节点详情"
    echo "  node update     更新节点"
    echo "  node delete     删除节点"
    echo "  node heartbeat  更新节点心跳"
    echo "  node stats      获取节点统计信息"
    echo "  pool list       列出所有存储池"
    echo "  pool create     创建存储池"
    echo "  pool get        获取存储池详情"
    echo "  pool update     更新存储池"
    echo "  pool delete     删除存储池"
    echo "  volume list     列出所有存储卷"
    echo "  volume create   创建存储卷"
    echo "  volume get      获取存储卷详情"
    echo "  volume delete   删除存储卷"
    echo "  volume resize   调整存储卷大小"
    echo "  volume snapshot 创建存储卷快照"
    echo "  network list    列出所有网络"
    echo "  network create  创建网络"
    echo "  network get     获取网络详情"
    echo "  network update  更新网络"
    echo "  network delete  删除网络"
    echo "  network ips     列出网络IP分配"
    echo ""
    echo "登录选项:"
    echo "  --u, -u         用户名 (必需)"
    echo "  -p              密码 (必需)"
    echo ""
    echo "创建虚拟机选项:"
    echo "  --name          虚拟机名称 (必需)"
    echo "  --node-id       节点ID (必需)"
    echo "  --vcpu          CPU核心数 (必需)"
    echo "  --memory        内存大小(MB) (必需)"
    echo "  --disks         磁盘配置 (可选)"
    echo "                  格式: <volume_id>[,<device>[,<bootable>]]"
    echo "                  device默认自动分配(vda,vdb...), bootable默认false(首个默认true)"
    echo "                  多个磁盘用分号分隔"
    echo "                  例: vol-1  或  vol-1,vda  或  vol-1,vda,true  或  vol-1;vol-2"
    echo "  --networks      网络接口配置 (可选)"
    echo "                  格式: <network_id>[,<model>]"
    echo "                  model默认virtio"
    echo "                  注意: IP地址和MAC地址会自动分配，无需手动指定"
    echo "                  多个网络用分号分隔"
    echo "                  例: net-1  或  net-1,virtio  或  net-1,e1000"
    echo "  --metadata      元数据 (可选)"
    echo "                  格式: key1=value1,key2=value2"
    echo ""
    echo "更新虚拟机选项:"
    echo "  --id            虚拟机ID (必需)"
    echo "  --name          新名称 (可选)"
    echo "  --vcpu          CPU核心数 (可选)"
    echo "  --memory        内存大小(MB) (可选)"
    echo "  --disks         磁盘配置 (可选)"
    echo "  --networks      网络配置 (可选)"
    echo "  --metadata      元数据 (可选)"
    echo ""
    echo "虚拟机操作选项:"
    echo "  --id            虚拟机ID (必需)"
    echo "  --force         强制操作 (可选, 用于stop命令)"
    echo ""
    echo "迁移虚拟机选项:"
    echo "  --id            虚拟机ID (必需)"
    echo "  --target-node   目标节点ID (必需)"
    echo ""
    echo "挂载/卸载存储卷选项:"
    echo "  --id            虚拟机ID (必需)"
    echo "  --volume-id     存储卷ID (必需)"
    echo ""
    echo "创建存储卷选项:"
    echo "  --name          存储卷名称 (必需)"
    echo "  --pool-id       存储池ID (必需)"
    echo "  --size          大小(GB) (必需)"
    echo "  --type          卷类型 (必需，如: qcow2, raw)"
    echo "  --node-id       节点ID (可选)"
    echo ""
    echo "调整存储卷大小选项:"
    echo "  --id            存储卷ID (必需)"
    echo "  --size          新大小(GB) (必需)"
    echo ""
    echo "创建存储卷快照选项:"
    echo "  --id            存储卷ID (必需)"
    echo "  --name          快照名称 (可选)"
    echo "  --device        设备名 (挂载时可选, 默认自动分配)"
    echo "  --bootable      是否启动盘 (挂载时可选, 默认false)"
    echo ""
    echo "创建节点选项:"
    echo "  --hostname      主机名 (必需)"
    echo "  --ip            IP地址 (必需)"
    echo "  --hypervisor    虚拟化类型 (可选, 默认: kvm)"
    echo "  --version       虚拟化版本 (可选, 默认: 6.2.0)"
    echo ""
    echo "更新节点选项:"
    echo "  --id            节点ID (必需)"
    echo "  --hostname      主机名 (可选)"
    echo "  --status        状态 (可选: online/offline/maintenance)"
    echo ""
    echo "节点心跳选项:"
    echo "  --id            节点ID (必需)"
    echo "  --cpu-cores     CPU核心数 (可选)"
    echo "  --cpu-threads   CPU线程数 (可选)"
    echo "  --memory        总内存(字节) (可选)"
    echo "  --disk          总磁盘(字节) (可选)"
    echo ""
    echo "创建存储池选项:"
    echo "  --name          存储池名称 (必需)"
    echo "  --type          存储池类型 (必需: nfs/lvm/ceph/iscsi)"
    echo "  --config        配置JSON字符串 (必需)"
    echo "  --capacity      容量(GB) (可选)"
    echo ""
    echo "更新存储池选项:"
    echo "  --id            存储池ID (必需)"
    echo "  --name          新名称 (可选)"
    echo "  --status        新状态 (可选: active/inactive/error)"
    echo "  --capacity      容量(GB) (可选)"
    echo ""
    echo "创建存储卷选项:"
    echo "  --name          存储卷名称 (必需)"
    echo "  --pool-id       存储池ID (必需)"
    echo "  --size          大小(GB) (必需)"
    echo "  --type          卷类型 (必需: qcow2/raw)"
    echo "  --node-id       节点ID (可选)"
    echo ""
    echo "创建网络选项:"
    echo "  --name          网络名称 (必需)"
    echo "  --type          网络类型 (必需: bridge/ovs)"
    echo "  --cidr          CIDR地址范围 (必需, 例: 192.168.100.0/24)"
    echo "  --gateway       网关地址 (可选, 例: 192.168.100.1)"
    echo "  --vlan-id       VLAN ID (必需, 例: 100)"
    echo "  --mtu           MTU大小 (可选, 默认: 1500)"
    echo ""
    echo "更新网络选项:"
    echo "  --id            网络ID (必需)"
    echo "  --name          新名称 (可选)"
    echo "  --cidr          CIDR地址范围 (可选)"
    echo "  --gateway       网关地址 (可选)"
    echo "  --mtu           MTU大小 (可选)"
    echo ""
    echo "示例:"
    echo "  # 登录和用户管理"
    echo "  ./api.sh login --u admin -p admin123"
    echo "  ./api.sh user list"
    echo ""
    echo "  # 虚拟机管理"
    echo "  ./api.sh vm list"
    echo "  ./api.sh vm create --name test-vm --node-id node-001 --vcpu 2 --memory 2048 --disks vol-123 --networks net-uuid"
    echo "  ./api.sh vm get --id <vm-id>"
    echo "  ./api.sh vm update --id <vm-id> --name new-vm --vcpu 4"
    echo "  ./api.sh vm delete --id <vm-id>"
    echo "  ./api.sh vm start --id <vm-id>"
    echo "  ./api.sh vm stop --id <vm-id>"
    echo "  ./api.sh vm stop --id <vm-id> --force"
    echo "  ./api.sh vm restart --id <vm-id>"
    echo "  ./api.sh vm migrate --id <vm-id> --target-node <node-id>"
    echo "  ./api.sh vm attach --id <vm-id> --volume-id <vol-id>"
    echo "  ./api.sh vm detach --id <vm-id> --volume-id <vol-id>"
    echo "  ./api.sh vm volumes --id <vm-id>"
    echo ""
    echo "  # 节点管理"
    echo "  ./api.sh node list"
    echo "  ./api.sh node create --hostname node-01 --ip 192.168.1.100"
    echo "  ./api.sh node get --id <node-id>"
    echo "  ./api.sh node update --id <node-id> --hostname node-01-new --status maintenance"
    echo "  ./api.sh node delete --id <node-id>"
    echo "  ./api.sh node heartbeat --id <node-id> --cpu-cores 16 --cpu-threads 32"
    echo "  ./api.sh node stats"
    echo "  ./api.sh pool list"
    echo "  ./api.sh pool create --name nfs-pool1 --type nfs --config '{\"server\":\"192.168.1.10\",\"path\":\"/data/nfs\"}' --capacity 1000"
    echo "  ./api.sh pool get --id <pool-id>"
    echo "  ./api.sh pool update --id <pool-id> --status active --capacity 2000"
    echo "  ./api.sh pool delete --id <pool-id>"
    echo "  ./api.sh volume list"
    echo "  ./api.sh volume create --name vol1 --pool-id <pool-id> --node-id <node-id> --size 10 --type qcow2"
    echo ""
    echo "  # 网络管理"
    echo "  ./api.sh network list"
    echo "  ./api.sh network create --name vlan100-net --type bridge --cidr 192.168.100.0/24 --gateway 192.168.100.1 --vlan-id 100"
    echo "  ./api.sh network get --id <network-id>"
    echo "  ./api.sh network update --id <network-id> --name new-name --gateway 192.168.100.254"
    echo "  ./api.sh network delete --id <network-id>"
    echo "  ./api.sh network ips --id <network-id>"
    echo ""
    echo "环境变量:"
    echo "  API_BASE_URL    API服务器地址 (默认: http://localhost:3000)"
}

# 读取保存的token
get_token() {
    if [[ ! -f "$TOKEN_FILE" ]]; then
        echo -e "${RED}错误: 未找到token文件，请先登录${NC}"
        echo "使用命令: ./api.sh login --u <username> -p <password>"
        exit 1
    fi
    
    cat "$TOKEN_FILE"
}

# 登录函数
login() {
    local username=""
    local password=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --u|-u)
                username="$2"
                shift 2
                ;;
            -p)
                password="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$username" ]]; then
        echo -e "${RED}错误: 用户名不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$password" ]]; then
        echo -e "${RED}错误: 密码不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 发送登录请求
    echo -e "${YELLOW}正在登录...${NC}"
    echo "用户名: $username"
    echo "API地址: $API_BASE_URL/api/auth/login"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"password\":\"$password\"}" \
        "$API_BASE_URL/api/auth/login")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 登录成功!${NC}"
        echo ""
        echo "响应数据:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        
        # 保存token到文件
        token=$(echo "$body" | jq -r '.auth.token' 2>/dev/null)
        if [[ -n "$token" && "$token" != "null" ]]; then
            echo "$token" > "$TOKEN_FILE"
            echo -e "${GREEN}✓ Token已保存到 $TOKEN_FILE${NC}"
        fi
    else
        echo -e "${RED}✗ 登录失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 列出所有用户
user_list() {
    local token=$(get_token)
    
    echo -e "${YELLOW}正在获取用户列表...${NC}"
    echo "API地址: $API_BASE_URL/api/users"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/users")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        
        # 显示表头
        printf "%-6s | %-20s | %-30s | %-30s | %-8s\n" "ID" "用户名" "邮箱" "角色" "状态"
        printf '%*s\n' 110 '' | tr ' ' '='
        
        # 以表格形式显示用户列表
        echo "$body" | jq -r '.data[] | 
            [
                (.id | tostring),
                .username,
                .email,
                (.roles | join(", ")),
                (if .is_active then "启用" else "禁用" end)
            ] | @tsv' | while IFS=$'\t' read -r id username email roles status; do
            printf "%-6s | %-20s | %-30s | %-30s | %-8s\n" "$id" "$username" "$email" "$roles" "$status"
        done
        
        # 显示统计信息
        echo ""
        total=$(echo "$body" | jq -r '.pagination.total // 0' 2>/dev/null)
        echo -e "${GREEN}总计: $total 个用户${NC}"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 列出所有虚拟机
vm_list() {
    local token=$(get_token)
    
    echo -e "${YELLOW}正在获取虚拟机列表...${NC}"
    echo "API地址: $API_BASE_URL/api/vms"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/vms")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        
        # 显示表头
        printf "%-36s | %-20s | %-36s | %-10s | %-8s | %-12s | %-20s\n" "VM ID" "名称" "节点ID" "状态" "vCPU" "内存(MB)" "创建时间"
        printf '%*s\n' 160 '' | tr ' ' '='
        
        # 以表格形式显示虚拟机列表（显示完整UUID）
        echo "$body" | jq -r '.vms[] | 
            [
                .id,
                .name,
                (.node_id // "N/A"),
                .status,
                (.vcpu | tostring),
                (.memory_mb | tostring),
                .created_at[0:19]
            ] | @tsv' | while IFS=$'\t' read -r id name node_id status vcpu memory created_at; do
            printf "%-36s | %-20s | %-36s | %-10s | %-8s | %-12s | %-20s\n" "$id" "$name" "$node_id" "$status" "$vcpu" "$memory" "$created_at"
        done
        
        # 显示统计信息
        echo ""
        total=$(echo "$body" | jq -r '.total // 0' 2>/dev/null)
        echo -e "${GREEN}总计: $total 个虚拟机${NC}"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 创建虚拟机
vm_create() {
    local token=$(get_token)
    local name=""
    local node_id=""
    local vcpu=""
    local memory=""
    local disks=""
    local networks=""
    local metadata=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --name)
                name="$2"
                shift 2
                ;;
            --node-id)
                node_id="$2"
                shift 2
                ;;
            --vcpu)
                vcpu="$2"
                shift 2
                ;;
            --memory)
                memory="$2"
                shift 2
                ;;
            --disks)
                disks="$2"
                shift 2
                ;;
            --networks)
                networks="$2"
                shift 2
                ;;
            --metadata)
                metadata="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 虚拟机名称不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$node_id" ]]; then
        echo -e "${RED}错误: 节点ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$vcpu" ]]; then
        echo -e "${RED}错误: CPU核心数不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$memory" ]]; then
        echo -e "${RED}错误: 内存大小不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 解析disks参数（格式: vol-id[,device[,bootable]];vol-id2）
    local disks_json=""
    if [[ -n "$disks" ]]; then
        disks_json="["
        local first_disk=true
        local disk_index=0
        local device_names=("vda" "vdb" "vdc" "vdd" "vde" "vdf" "vdg" "vdh")
        
        IFS=';' read -ra DISK_ARRAY <<< "$disks"
        for disk_spec in "${DISK_ARRAY[@]}"; do
            IFS=',' read -r vol_id device bootable <<< "$disk_spec"
            
            # 去除空格
            vol_id=$(echo "$vol_id" | xargs)
            device=$(echo "$device" | xargs)
            bootable=$(echo "$bootable" | xargs)
            
            # 验证volume_id（必需）
            if [[ -z "$vol_id" ]]; then
                echo -e "${RED}错误: 磁盘配置格式错误，volume_id不能为空${NC}"
                echo -e "${YELLOW}格式: <volume_id>[,<device>[,<bootable>]]${NC}"
                exit 1
            fi
            
            # 设置默认值
            if [[ -z "$device" ]]; then
                device="${device_names[$disk_index]}"
            fi
            if [[ -z "$bootable" ]]; then
                # 第一个磁盘默认为启动盘
                if [[ $disk_index -eq 0 ]]; then
                    bootable="true"
                else
                    bootable="false"
                fi
            fi
            
            [[ "$first_disk" == false ]] && disks_json="$disks_json,"
            disks_json="$disks_json{\"volume_id\":\"$vol_id\",\"device\":\"$device\",\"bootable\":$bootable}"
            first_disk=false
            disk_index=$((disk_index + 1))
        done
        disks_json="$disks_json]"
    fi
    
    # 解析networks参数（格式: net-id[,model];net-id2）
    # 注意：IP和MAC地址由服务器自动分配，无需手动指定
    local networks_json=""
    if [[ -n "$networks" ]]; then
        networks_json="["
        local first_net=true
        IFS=';' read -ra NET_ARRAY <<< "$networks"
        for net_spec in "${NET_ARRAY[@]}"; do
            IFS=',' read -r net_id model <<< "$net_spec"
            
            # 去除空格
            net_id=$(echo "$net_id" | xargs)
            model=$(echo "$model" | xargs)
            
            # 验证network_id（必需）
            if [[ -z "$net_id" ]]; then
                echo -e "${RED}错误: 网络配置格式错误，network_id不能为空${NC}"
                echo -e "${YELLOW}格式: <network_id>[,<model>]${NC}"
                exit 1
            fi
            
            # 设置默认值
            if [[ -z "$model" ]]; then
                model="virtio"
            fi
            
            [[ "$first_net" == false ]] && networks_json="$networks_json,"
            networks_json="$networks_json{\"network_id\":\"$net_id\",\"model\":\"$model\"}"
            first_net=false
        done
        networks_json="$networks_json]"
    fi
    
    # 解析metadata参数（格式: key1=value1,key2=value2）
    local metadata_json=""
    if [[ -n "$metadata" ]]; then
        metadata_json="{"
        local first_meta=true
        IFS=',' read -ra META_ARRAY <<< "$metadata"
        for meta_spec in "${META_ARRAY[@]}"; do
            IFS='=' read -r key value <<< "$meta_spec"
            
            # 去除空格
            key=$(echo "$key" | xargs)
            value=$(echo "$value" | xargs)
            
            # 验证参数
            if [[ -z "$key" || -z "$value" ]]; then
                echo -e "${RED}错误: 元数据配置格式错误，应为 key1=value1,key2=value2${NC}"
                exit 1
            fi
            
            [[ "$first_meta" == false ]] && metadata_json="$metadata_json,"
            metadata_json="$metadata_json\"$key\":\"$value\""
            first_meta=false
        done
        metadata_json="$metadata_json}"
    fi
    
    # 构建JSON请求体
    local json_body=$(cat <<EOF
{
    "name": "$name",
    "node_id": "$node_id",
    "vcpu": $vcpu,
    "memory_mb": $memory
EOF
)
    
    # 添加可选字段
    if [[ -n "$disks_json" ]]; then
        json_body="$json_body,\n    \"disks\": $disks_json"
    fi
    
    if [[ -n "$networks_json" ]]; then
        json_body="$json_body,\n    \"networks\": $networks_json"
    fi
    
    if [[ -n "$metadata_json" ]]; then
        json_body="$json_body,\n    \"metadata\": $metadata_json"
    fi
    
    json_body="$json_body\n}"
    
    # 发送创建请求
    echo -e "${YELLOW}正在创建虚拟机...${NC}"
    echo "虚拟机名称: $name"
    echo "节点ID: $node_id"
    echo "CPU核心数: $vcpu"
    echo "内存大小: ${memory}MB"
    [[ -n "$disks" ]] && echo "磁盘配置: $disks"
    [[ -n "$networks" ]] && echo "网络配置: $networks"
    [[ -n "$metadata" ]] && echo "元数据: $metadata"
    echo "API地址: $API_BASE_URL/api/vms"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$(echo -e "$json_body")" \
        "$API_BASE_URL/api/vms")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "201" ]]; then
        echo -e "${GREEN}✓ 创建成功!${NC}"
        echo ""
        echo "虚拟机详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 创建失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 获取虚拟机详情
vm_get() {
    local token=$(get_token)
    local vm_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在获取虚拟机详情...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/vms/$vm_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        echo "虚拟机详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 更新虚拟机
vm_update() {
    local token=$(get_token)
    local vm_id=""
    local name=""
    local vcpu=""
    local memory=""
    local disks=""
    local networks=""
    local metadata=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            --name)
                name="$2"
                shift 2
                ;;
            --vcpu)
                vcpu="$2"
                shift 2
                ;;
            --memory)
                memory="$2"
                shift 2
                ;;
            --disks)
                disks="$2"
                shift 2
                ;;
            --networks)
                networks="$2"
                shift 2
                ;;
            --metadata)
                metadata="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$name" && -z "$vcpu" && -z "$memory" && -z "$disks" && -z "$networks" && -z "$metadata" ]]; then
        echo -e "${RED}错误: 至少需要提供一个更新参数${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body="{"
    local first=true
    
    if [[ -n "$name" ]]; then
        json_body="$json_body\"name\": \"$name\""
        first=false
    fi
    
    if [[ -n "$vcpu" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"vcpu\": $vcpu"
        first=false
    fi
    
    if [[ -n "$memory" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"memory_mb\": $memory"
        first=false
    fi
    
    # TODO: 如果需要，可以添加disks, networks, metadata的解析逻辑
    
    json_body="$json_body}"
    
    # 发送更新请求
    echo -e "${YELLOW}正在更新虚拟机...${NC}"
    echo "虚拟机ID: $vm_id"
    [[ -n "$name" ]] && echo "新名称: $name"
    [[ -n "$vcpu" ]] && echo "新CPU核心数: $vcpu"
    [[ -n "$memory" ]] && echo "新内存大小: ${memory}MB"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X PUT \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/vms/$vm_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 更新成功!${NC}"
        echo ""
        echo "虚拟机详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 更新失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 删除虚拟机
vm_delete() {
    local token=$(get_token)
    local vm_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在删除虚拟机...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X DELETE \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/vms/$vm_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "204" ]]; then
        echo -e "${GREEN}✓ 删除成功!${NC}"
        echo ""
        if [[ -n "$body" ]]; then
            echo "$body" | jq '.' 2>/dev/null || echo "$body"
        fi
    else
        echo -e "${RED}✗ 删除失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 启动虚拟机
vm_start() {
    local token=$(get_token)
    local vm_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在启动虚拟机...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id/start"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/vms/$vm_id/start")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 启动成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 启动失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 停止虚拟机
vm_stop() {
    local token=$(get_token)
    local vm_id=""
    local force="false"
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            --force)
                force="true"
                shift
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在停止虚拟机...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "强制停止: $force"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id/stop"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{\"force\": $force}" \
        "$API_BASE_URL/api/vms/$vm_id/stop")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 停止成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 停止失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 重启虚拟机
vm_restart() {
    local token=$(get_token)
    local vm_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在重启虚拟机...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id/restart"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/vms/$vm_id/restart")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 重启成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 重启失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 迁移虚拟机
vm_migrate() {
    local token=$(get_token)
    local vm_id=""
    local target_node=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            --target-node)
                target_node="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$target_node" ]]; then
        echo -e "${RED}错误: 目标节点ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在迁移虚拟机...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "目标节点: $target_node"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id/migrate"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{\"target_node_id\": \"$target_node\"}" \
        "$API_BASE_URL/api/vms/$vm_id/migrate")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 迁移成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 迁移失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 挂载存储卷
vm_attach() {
    local token=$(get_token)
    local vm_id=""
    local volume_id=""
    local device=""
    local bootable="false"
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            --volume-id)
                volume_id="$2"
                shift 2
                ;;
            --device)
                device="$2"
                shift 2
                ;;
            --bootable)
                bootable="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$volume_id" ]]; then
        echo -e "${RED}错误: 存储卷ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body="{\"volume_id\": \"$volume_id\""
    
    if [[ -n "$device" ]]; then
        json_body="$json_body, \"device\": \"$device\""
    fi
    
    json_body="$json_body, \"bootable\": $bootable}"
    
    echo -e "${YELLOW}正在挂载存储卷...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "存储卷ID: $volume_id"
    [[ -n "$device" ]] && echo "设备名: $device"
    echo "启动盘: $bootable"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id/volumes/attach"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/vms/$vm_id/volumes/attach")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 挂载成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 挂载失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 卸载存储卷
vm_detach() {
    local token=$(get_token)
    local vm_id=""
    local volume_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            --volume-id)
                volume_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$volume_id" ]]; then
        echo -e "${RED}错误: 存储卷ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在卸载存储卷...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "存储卷ID: $volume_id"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id/volumes/detach"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{\"volume_id\": \"$volume_id\"}" \
        "$API_BASE_URL/api/vms/$vm_id/volumes/detach")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 卸载成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 卸载失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 列出虚拟机存储卷
vm_volumes() {
    local token=$(get_token)
    local vm_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                vm_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$vm_id" ]]; then
        echo -e "${RED}错误: 虚拟机ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在获取虚拟机存储卷列表...${NC}"
    echo "虚拟机ID: $vm_id"
    echo "API地址: $API_BASE_URL/api/vms/$vm_id/volumes"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/vms/$vm_id/volumes")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        
        # 显示表头
        printf "%-36s | %-10s | %-10s\n" "存储卷ID" "设备名" "启动盘"
        printf '%*s\n' 60 '' | tr ' ' '='
        
        # 以表格形式显示存储卷列表
        echo "$body" | jq -r '.[] | 
            [
                .volume_id,
                .device,
                (if .bootable then "是" else "否" end)
            ] | @tsv' | while IFS=$'\t' read -r vol_id device bootable; do
            printf "%-36s | %-10s | %-10s\n" "$vol_id" "$device" "$bootable"
        done
        
        # 显示统计信息
        echo ""
        total=$(echo "$body" | jq '. | length' 2>/dev/null)
        echo -e "${GREEN}总计: $total 个存储卷${NC}"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# ========== 节点管理功能 ==========

# 列出所有节点
node_list() {
    local token=$(get_token)
    
    echo -e "${YELLOW}正在获取节点列表...${NC}"
    echo "API地址: $API_BASE_URL/api/nodes"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/nodes?page=1&page_size=50")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        
        # 显示表头
        printf "%-36s | %-20s | %-15s | %-20s | %-10s | %-12s\n" "ID" "主机名" "IP地址" "gRPC地址" "状态" "虚拟化"
        printf '%*s\n' 100 '' | tr ' ' '='
        
        # 以表格形式显示节点列表
        echo "$body" | jq -r '.nodes[] | 
            [
                .id,
                .hostname,
                .ip_address,
                (.grpc_address // "N/A"),
                .status,
                (.hypervisor_type // "N/A")
            ] | @tsv' | while IFS=$'\t' read -r id hostname ip grpc status hypervisor; do
            printf "%-36s | %-20s | %-15s | %-20s | %-10s | %-12s\n" "$id" "$hostname" "$ip" "$grpc" "$status" "$hypervisor"
        done
        
        # 显示统计信息
        echo ""
        total=$(echo "$body" | jq -r '.total // 0' 2>/dev/null)
        echo -e "${GREEN}总计: $total 个节点${NC}"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 创建节点
node_create() {
    local token=$(get_token)
    local hostname=""
    local ip=""
    local hypervisor="kvm"
    local version="6.2.0"
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --hostname)
                hostname="$2"
                shift 2
                ;;
            --ip)
                ip="$2"
                shift 2
                ;;
            --hypervisor)
                hypervisor="$2"
                shift 2
                ;;
            --version)
                version="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$hostname" ]]; then
        echo -e "${RED}错误: 主机名不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$ip" ]]; then
        echo -e "${RED}错误: IP地址不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body=$(cat <<EOF
{
    "hostname": "$hostname",
    "ip_address": "$ip",
    "hypervisor_type": "$hypervisor",
    "hypervisor_version": "$version"
}
EOF
)
    
    # 发送创建请求
    echo -e "${YELLOW}正在创建节点...${NC}"
    echo "主机名: $hostname"
    echo "IP地址: $ip"
    echo "虚拟化类型: $hypervisor"
    echo "虚拟化版本: $version"
    echo "API地址: $API_BASE_URL/api/nodes"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/nodes")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
        echo -e "${GREEN}✓ 创建成功!${NC}"
        echo ""
        echo "节点详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 创建失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 获取节点详情
node_get() {
    local token=$(get_token)
    local node_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                node_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$node_id" ]]; then
        echo -e "${RED}错误: 节点ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在获取节点详情...${NC}"
    echo "节点ID: $node_id"
    echo "API地址: $API_BASE_URL/api/nodes/$node_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/nodes/$node_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        echo "节点详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 更新节点
node_update() {
    local token=$(get_token)
    local node_id=""
    local hostname=""
    local status=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                node_id="$2"
                shift 2
                ;;
            --hostname)
                hostname="$2"
                shift 2
                ;;
            --status)
                status="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$node_id" ]]; then
        echo -e "${RED}错误: 节点ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$hostname" && -z "$status" ]]; then
        echo -e "${RED}错误: 至少需要提供一个更新参数 (--hostname 或 --status)${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body="{"
    local first=true
    
    if [[ -n "$hostname" ]]; then
        json_body="$json_body\"hostname\": \"$hostname\""
        first=false
    fi
    
    if [[ -n "$status" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"status\": \"$status\""
    fi
    
    json_body="$json_body}"
    
    # 发送更新请求
    echo -e "${YELLOW}正在更新节点...${NC}"
    echo "节点ID: $node_id"
    [[ -n "$hostname" ]] && echo "新主机名: $hostname"
    [[ -n "$status" ]] && echo "新状态: $status"
    echo "API地址: $API_BASE_URL/api/nodes/$node_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X PUT \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/nodes/$node_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 更新成功!${NC}"
        echo ""
        echo "节点详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 更新失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 删除节点
node_delete() {
    local token=$(get_token)
    local node_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                node_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$node_id" ]]; then
        echo -e "${RED}错误: 节点ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在删除节点...${NC}"
    echo "节点ID: $node_id"
    echo "API地址: $API_BASE_URL/api/nodes/$node_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X DELETE \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/nodes/$node_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 删除成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 删除失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 更新节点心跳
node_heartbeat() {
    local token=$(get_token)
    local node_id=""
    local cpu_cores=""
    local cpu_threads=""
    local memory=""
    local disk=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                node_id="$2"
                shift 2
                ;;
            --cpu-cores)
                cpu_cores="$2"
                shift 2
                ;;
            --cpu-threads)
                cpu_threads="$2"
                shift 2
                ;;
            --memory)
                memory="$2"
                shift 2
                ;;
            --disk)
                disk="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$node_id" ]]; then
        echo -e "${RED}错误: 节点ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body="{"
    local first=true
    
    if [[ -n "$cpu_cores" ]]; then
        json_body="$json_body\"cpu_cores\": $cpu_cores"
        first=false
    fi
    
    if [[ -n "$cpu_threads" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"cpu_threads\": $cpu_threads"
        first=false
    fi
    
    if [[ -n "$memory" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"memory_total\": $memory"
        first=false
    fi
    
    if [[ -n "$disk" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"disk_total\": $disk"
        first=false
    fi
    
    # 添加默认的虚拟化信息
    if [[ "$first" == false ]]; then
        json_body="$json_body,"
    fi
    json_body="$json_body\"hypervisor_type\": \"kvm\", \"hypervisor_version\": \"6.2.0\"}"
    
    # 发送心跳请求
    echo -e "${YELLOW}正在更新节点心跳...${NC}"
    echo "节点ID: $node_id"
    [[ -n "$cpu_cores" ]] && echo "CPU核心数: $cpu_cores"
    [[ -n "$cpu_threads" ]] && echo "CPU线程数: $cpu_threads"
    [[ -n "$memory" ]] && echo "总内存: $memory 字节"
    [[ -n "$disk" ]] && echo "总磁盘: $disk 字节"
    echo "API地址: $API_BASE_URL/api/nodes/$node_id/heartbeat"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/nodes/$node_id/heartbeat")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 心跳更新成功!${NC}"
        echo ""
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 心跳更新失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 获取节点统计信息
node_stats() {
    local token=$(get_token)
    
    echo -e "${YELLOW}正在获取节点统计信息...${NC}"
    echo "API地址: $API_BASE_URL/api/nodes/stats"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/nodes/stats")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        echo "统计信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# ========== 存储池管理功能 ==========

# 列出所有存储池
pool_list() {
    local token=$(get_token)
    
    echo -e "${YELLOW}正在获取存储池列表...${NC}"
    echo "API地址: $API_BASE_URL/api/storage/pools"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/storage/pools?page=1&page_size=50")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        
        # 显示表头
        printf "%-36s | %-20s | %-10s | %-12s | %-12s | %-12s | %-20s\n" "ID" "名称" "类型" "状态" "容量(GB)" "已用(GB)" "创建时间"
        printf '%*s\n' 150 '' | tr ' ' '='
        
        # 以表格形式显示存储池列表
        echo "$body" | jq -r '.pools[]? // .data[]? // empty | 
            [
                .id,
                .name,
                .pool_type,
                .status,
                (.capacity_gb // 0 | tostring),
                (.allocated_gb // 0 | tostring),
                .created_at[0:19]
            ] | @tsv' | while IFS=$'\t' read -r id name type status capacity allocated created_at; do
            printf "%-36s | %-20s | %-10s | %-12s | %-12s | %-12s | %-20s\n" "$id" "$name" "$type" "$status" "$capacity" "$allocated" "$created_at"
        done
        
        # 显示统计信息
        echo ""
        total=$(echo "$body" | jq -r '.total // (.pools | length) // 0' 2>/dev/null)
        echo -e "${GREEN}总计: $total 个存储池${NC}"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 创建存储池
pool_create() {
    local token=$(get_token)
    local name=""
    local pool_type=""
    local config=""
    local capacity=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --name)
                name="$2"
                shift 2
                ;;
            --type)
                pool_type="$2"
                shift 2
                ;;
            --config)
                config="$2"
                shift 2
                ;;
            --capacity)
                capacity="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 存储池名称不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$pool_type" ]]; then
        echo -e "${RED}错误: 存储池类型不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$config" ]]; then
        echo -e "${RED}错误: 配置JSON不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body=$(cat <<EOF
{
    "name": "$name",
    "pool_type": "$pool_type",
    "config": $config
EOF
)
    
    # 如果提供了capacity，则添加到JSON中
    if [[ -n "$capacity" ]]; then
        json_body="$json_body,\n    \"capacity_gb\": $capacity"
    fi
    
    json_body="$json_body\n}"
    
    # 发送创建请求
    echo -e "${YELLOW}正在创建存储池...${NC}"
    echo "存储池名称: $name"
    echo "存储池类型: $pool_type"
    echo "配置: $config"
    [[ -n "$capacity" ]] && echo "容量: ${capacity}GB"
    echo "API地址: $API_BASE_URL/api/v1/storage/pools"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$(echo -e "$json_body")" \
        "$API_BASE_URL/api/storage/pools")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
        echo -e "${GREEN}✓ 创建成功!${NC}"
        echo ""
        echo "存储池详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 创建失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 获取存储池详情
pool_get() {
    local token=$(get_token)
    local pool_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                pool_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$pool_id" ]]; then
        echo -e "${RED}错误: 存储池ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在获取存储池详情...${NC}"
    echo "存储池ID: $pool_id"
    echo "API地址: $API_BASE_URL/api/storage/pools/$pool_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/storage/pools/$pool_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        echo "存储池详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 更新存储池
pool_update() {
    local token=$(get_token)
    local pool_id=""
    local name=""
    local status=""
    local capacity=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                pool_id="$2"
                shift 2
                ;;
            --name)
                name="$2"
                shift 2
                ;;
            --status)
                status="$2"
                shift 2
                ;;
            --capacity)
                capacity="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$pool_id" ]]; then
        echo -e "${RED}错误: 存储池ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$name" && -z "$status" && -z "$capacity" ]]; then
        echo -e "${RED}错误: 至少需要提供一个更新参数 (--name, --status 或 --capacity)${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body="{"
    local first=true
    
    if [[ -n "$name" ]]; then
        json_body="$json_body\"name\": \"$name\""
        first=false
    fi
    
    if [[ -n "$status" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"status\": \"$status\""
        first=false
    fi
    
    if [[ -n "$capacity" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"capacity_gb\": $capacity"
    fi
    
    json_body="$json_body}"
    
    # 发送更新请求
    echo -e "${YELLOW}正在更新存储池...${NC}"
    echo "存储池ID: $pool_id"
    [[ -n "$name" ]] && echo "新名称: $name"
    [[ -n "$status" ]] && echo "新状态: $status"
    [[ -n "$capacity" ]] && echo "新容量: ${capacity}GB"
    echo "API地址: $API_BASE_URL/api/storage/pools/$pool_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X PUT \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/storage/pools/$pool_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 更新成功!${NC}"
        echo ""
        echo "存储池详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 更新失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 删除存储池
pool_delete() {
    local token=$(get_token)
    local pool_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                pool_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$pool_id" ]]; then
        echo -e "${RED}错误: 存储池ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在删除存储池...${NC}"
    echo "存储池ID: $pool_id"
    echo "API地址: $API_BASE_URL/api/storage/pools/$pool_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X DELETE \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/storage/pools/$pool_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "204" ]]; then
        echo -e "${GREEN}✓ 删除成功!${NC}"
        echo ""
        if [[ -n "$body" ]]; then
            echo "$body" | jq '.' 2>/dev/null || echo "$body"
        fi
    else
        echo -e "${RED}✗ 删除失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# ========== 存储卷管理功能 ==========

# 列出所有存储卷
volume_list() {
    local token=$(get_token)
    
    echo -e "${YELLOW}正在获取存储卷列表...${NC}"
    echo "API地址: $API_BASE_URL/api/storage/volumes"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/storage/volumes?page=1&page_size=50")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        
        # 显示表头
        printf "%-36s | %-20s | %-36s | %-10s | %-8s | %-12s | %-20s\n" "ID" "名称" "存储池ID" "类型" "大小(GB)" "状态" "创建时间"
        printf '%*s\n' 150 '' | tr ' ' '='
        
        # 以表格形式显示存储卷列表
        echo "$body" | jq -r '.volumes[]? // .data[]? // empty | 
            [
                .id,
                .name,
                .pool_id,
                .volume_type,
                (.size_gb | tostring),
                .status,
                .created_at[0:19]
            ] | @tsv' | while IFS=$'\t' read -r id name pool_id type size status created_at; do
            printf "%-36s | %-20s | %-36s | %-10s | %-8s | %-12s | %-20s\n" "$id" "$name" "$pool_id" "$type" "$size" "$status" "$created_at"
        done
        
        # 显示统计信息
        echo ""
        total=$(echo "$body" | jq -r '.total // (.volumes | length) // 0' 2>/dev/null)
        echo -e "${GREEN}总计: $total 个存储卷${NC}"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 创建存储卷
volume_create() {
    local token=$(get_token)
    local name=""
    local pool_id=""
    local size=""
    local volume_type=""
    local node_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --name)
                name="$2"
                shift 2
                ;;
            --pool-id)
                pool_id="$2"
                shift 2
                ;;
            --size)
                size="$2"
                shift 2
                ;;
            --type)
                volume_type="$2"
                shift 2
                ;;
            --node-id)
                node_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 存储卷名称不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$pool_id" ]]; then
        echo -e "${RED}错误: 存储池ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$size" ]]; then
        echo -e "${RED}错误: 大小不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$volume_type" ]]; then
        echo -e "${RED}错误: 卷类型不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body=$(cat <<EOF
{
    "name": "$name",
    "pool_id": "$pool_id",
    "size_gb": $size,
    "volume_type": "$volume_type"
EOF
)
    
    # 如果提供了node_id，则添加到JSON中
    if [[ -n "$node_id" ]]; then
        json_body="$json_body,\n    \"node_id\": \"$node_id\""
    fi
    
    json_body="$json_body\n}"
    
    # 发送创建请求
    echo -e "${YELLOW}正在创建存储卷...${NC}"
    echo "存储卷名称: $name"
    echo "存储池ID: $pool_id"
    echo "大小: ${size}GB"
    echo "卷类型: $volume_type"
    [[ -n "$node_id" ]] && echo "节点ID: $node_id"
    echo "API地址: $API_BASE_URL/api/storage/volumes"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$(echo -e "$json_body")" \
        "$API_BASE_URL/api/storage/volumes")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
        echo -e "${GREEN}✓ 创建成功!${NC}"
        echo ""
        echo "存储卷详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 创建失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 获取存储卷详情
volume_get() {
    local token=$(get_token)
    local volume_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                volume_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$volume_id" ]]; then
        echo -e "${RED}错误: 存储卷ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在获取存储卷详情...${NC}"
    echo "存储卷ID: $volume_id"
    echo "API地址: $API_BASE_URL/api/storage/volumes/$volume_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/storage/volumes/$volume_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        echo "$body" | jq '.'
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 删除存储卷
volume_delete() {
    local token=$(get_token)
    local volume_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                volume_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$volume_id" ]]; then
        echo -e "${RED}错误: 存储卷ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    echo -e "${YELLOW}正在删除存储卷...${NC}"
    echo "存储卷ID: $volume_id"
    echo "API地址: $API_BASE_URL/api/storage/volumes/$volume_id"
    echo ""
    
    # 确认删除
    read -p "确认删除此存储卷? (yes/no): " confirm
    if [[ "$confirm" != "yes" ]]; then
        echo -e "${YELLOW}取消删除${NC}"
        exit 0
    fi
    
    response=$(curl -s -w "\n%{http_code}" -X DELETE \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$API_BASE_URL/api/storage/volumes/$volume_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "204" ]]; then
        echo -e "${GREEN}✓ 删除成功!${NC}"
        [[ -n "$body" ]] && echo "$body" | jq '.' 2>/dev/null
    else
        echo -e "${RED}✗ 删除失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 调整存储卷大小
volume_resize() {
    local token=$(get_token)
    local volume_id=""
    local new_size=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                volume_id="$2"
                shift 2
                ;;
            --size)
                new_size="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$volume_id" ]]; then
        echo -e "${RED}错误: 存储卷ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$new_size" ]]; then
        echo -e "${RED}错误: 新大小不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body=$(cat <<EOF
{
    "new_size_gb": $new_size
}
EOF
)
    
    echo -e "${YELLOW}正在调整存储卷大小...${NC}"
    echo "存储卷ID: $volume_id"
    echo "新大小: ${new_size}GB"
    echo "API地址: $API_BASE_URL/api/storage/volumes/$volume_id/resize"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/storage/volumes/$volume_id/resize")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 调整大小成功!${NC}"
        echo ""
        echo "$body" | jq '.'
    else
        echo -e "${RED}✗ 调整大小失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 创建存储卷快照
volume_snapshot() {
    local token=$(get_token)
    local volume_id=""
    local snapshot_name=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                volume_id="$2"
                shift 2
                ;;
            --name)
                snapshot_name="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$volume_id" ]]; then
        echo -e "${RED}错误: 存储卷ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 如果未提供快照名称，自动生成
    if [[ -z "$snapshot_name" ]]; then
        snapshot_name="snapshot-$(date +%Y%m%d-%H%M%S)"
    fi
    
    # 构建JSON请求体
    local json_body=$(cat <<EOF
{
    "snapshot_name": "$snapshot_name"
}
EOF
)
    
    echo -e "${YELLOW}正在创建存储卷快照...${NC}"
    echo "存储卷ID: $volume_id"
    echo "快照名称: $snapshot_name"
    echo "API地址: $API_BASE_URL/api/storage/volumes/$volume_id/snapshot"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/storage/volumes/$volume_id/snapshot")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
        echo -e "${GREEN}✓ 快照创建成功!${NC}"
        echo ""
        echo "$body" | jq '.'
    else
        echo -e "${RED}✗ 快照创建失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# ==================== 网络管理函数 ====================

# 列出所有网络
network_list() {
    local token=$(get_token)
    
    echo -e "${YELLOW}正在获取网络列表...${NC}"
    echo "API地址: $API_BASE_URL/api/networks"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        "$API_BASE_URL/api/networks")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        
        # 显示表头
        printf "%-36s | %-20s | %-10s | %-18s | %-15s | %-8s | %-10s | %-20s\n" "网络ID" "名称" "类型" "CIDR" "网关" "VLAN ID" "状态" "创建时间"
        printf '%*s\n' 160 '' | tr ' ' '='
        
        # 以表格形式显示网络列表（显示完整UUID）
        echo "$body" | jq -r '.networks[] | 
            [
                .id,
                .name,
                .network_type,
                .cidr,
                (.gateway // "N/A"),
                (.vlan_id | tostring),
                .status,
                .created_at[0:19]
            ] | @tsv' | while IFS=$'\t' read -r id name net_type cidr gateway vlan_id status created_at; do
            printf "%-36s | %-20s | %-10s | %-18s | %-15s | %-8s | %-10s | %-20s\n" "$id" "$name" "$net_type" "$cidr" "$gateway" "$vlan_id" "$status" "$created_at"
        done
        
        # 显示统计信息
        echo ""
        total=$(echo "$body" | jq -r '.total // 0' 2>/dev/null)
        [[ -z "$total" || "$total" == "null" ]] && total=$(echo "$body" | jq -r '.networks | length' 2>/dev/null)
        echo -e "${GREEN}总计: $total 个网络${NC}"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 创建网络
network_create() {
    local token=$(get_token)
    local name=""
    local network_type=""
    local cidr=""
    local gateway=""
    local vlan_id=""
    local mtu="1500"
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --name)
                name="$2"
                shift 2
                ;;
            --type)
                network_type="$2"
                shift 2
                ;;
            --cidr)
                cidr="$2"
                shift 2
                ;;
            --gateway)
                gateway="$2"
                shift 2
                ;;
            --vlan-id)
                vlan_id="$2"
                shift 2
                ;;
            --mtu)
                mtu="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证必需参数
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 网络名称不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$network_type" ]]; then
        echo -e "${RED}错误: 网络类型不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$cidr" ]]; then
        echo -e "${RED}错误: CIDR不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$vlan_id" ]]; then
        echo -e "${RED}错误: VLAN ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body=$(cat <<EOF
{
    "name": "$name",
    "network_type": "$network_type",
    "cidr": "$cidr",
    "vlan_id": $vlan_id,
    "mtu": $mtu
EOF
)
    
    if [[ -n "$gateway" ]]; then
        json_body="$json_body,\n    \"gateway\": \"$gateway\""
    fi
    
    json_body="$json_body\n}"
    
    # 发送创建请求
    echo -e "${YELLOW}正在创建网络...${NC}"
    echo "网络名称: $name"
    echo "网络类型: $network_type"
    echo "CIDR: $cidr"
    [[ -n "$gateway" ]] && echo "网关: $gateway"
    echo "VLAN ID: $vlan_id"
    echo "MTU: $mtu"
    echo "API地址: $API_BASE_URL/api/networks"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$(echo -e "$json_body")" \
        "$API_BASE_URL/api/networks")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
        echo -e "${GREEN}✓ 创建成功!${NC}"
        echo ""
        echo "网络详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        echo ""
        echo -e "${GREEN}提示: IP池已自动初始化${NC}"
    else
        echo -e "${RED}✗ 创建失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 获取网络详情
network_get() {
    local token=$(get_token)
    local network_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                network_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$network_id" ]]; then
        echo -e "${RED}错误: 网络ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 发送请求
    echo -e "${YELLOW}正在获取网络详情...${NC}"
    echo "网络ID: $network_id"
    echo "API地址: $API_BASE_URL/api/networks/$network_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        "$API_BASE_URL/api/networks/$network_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        echo "网络详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 更新网络
network_update() {
    local token=$(get_token)
    local network_id=""
    local name=""
    local cidr=""
    local gateway=""
    local mtu=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                network_id="$2"
                shift 2
                ;;
            --name)
                name="$2"
                shift 2
                ;;
            --cidr)
                cidr="$2"
                shift 2
                ;;
            --gateway)
                gateway="$2"
                shift 2
                ;;
            --mtu)
                mtu="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$network_id" ]]; then
        echo -e "${RED}错误: 网络ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    if [[ -z "$name" && -z "$cidr" && -z "$gateway" && -z "$mtu" ]]; then
        echo -e "${RED}错误: 至少需要提供一个更新参数${NC}"
        print_help
        exit 1
    fi
    
    # 构建JSON请求体
    local json_body="{"
    local first=true
    
    if [[ -n "$name" ]]; then
        json_body="$json_body\"name\": \"$name\""
        first=false
    fi
    
    if [[ -n "$cidr" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"cidr\": \"$cidr\""
        first=false
    fi
    
    if [[ -n "$gateway" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"gateway\": \"$gateway\""
        first=false
    fi
    
    if [[ -n "$mtu" ]]; then
        [[ "$first" == false ]] && json_body="$json_body,"
        json_body="$json_body\"mtu\": $mtu"
    fi
    
    json_body="$json_body}"
    
    # 发送更新请求
    echo -e "${YELLOW}正在更新网络...${NC}"
    echo "网络ID: $network_id"
    [[ -n "$name" ]] && echo "新名称: $name"
    [[ -n "$cidr" ]] && echo "新CIDR: $cidr"
    [[ -n "$gateway" ]] && echo "新网关: $gateway"
    [[ -n "$mtu" ]] && echo "新MTU: $mtu"
    echo "API地址: $API_BASE_URL/api/networks/$network_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X PUT \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$json_body" \
        "$API_BASE_URL/api/networks/$network_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 更新成功!${NC}"
        echo ""
        echo "网络详情:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 更新失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 删除网络
network_delete() {
    local token=$(get_token)
    local network_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                network_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$network_id" ]]; then
        echo -e "${RED}错误: 网络ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 发送删除请求
    echo -e "${YELLOW}正在删除网络...${NC}"
    echo "网络ID: $network_id"
    echo "API地址: $API_BASE_URL/api/networks/$network_id"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X DELETE \
        -H "Authorization: Bearer $token" \
        "$API_BASE_URL/api/networks/$network_id")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" || "$http_code" == "204" ]]; then
        echo -e "${GREEN}✓ 删除成功!${NC}"
    else
        echo -e "${RED}✗ 删除失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 列出网络的IP分配
network_ips() {
    local token=$(get_token)
    local network_id=""
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --id)
                network_id="$2"
                shift 2
                ;;
            *)
                echo -e "${RED}错误: 未知参数 '$1'${NC}"
                print_help
                exit 1
                ;;
        esac
    done
    
    # 验证参数
    if [[ -z "$network_id" ]]; then
        echo -e "${RED}错误: 网络ID不能为空${NC}"
        print_help
        exit 1
    fi
    
    # 发送请求
    echo -e "${YELLOW}正在获取IP分配列表...${NC}"
    echo "网络ID: $network_id"
    echo "API地址: $API_BASE_URL/api/networks/$network_id/ips"
    echo ""
    
    response=$(curl -s -w "\n%{http_code}" -X GET \
        -H "Authorization: Bearer $token" \
        "$API_BASE_URL/api/networks/$network_id/ips")
    
    # 分离响应体和状态码
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    # 检查响应状态
    if [[ "$http_code" == "200" ]]; then
        echo -e "${GREEN}✓ 获取成功!${NC}"
        echo ""
        echo "IP分配列表:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ 获取失败! (HTTP $http_code)${NC}"
        echo ""
        echo "错误信息:"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        exit 1
    fi
}

# 主程序
main() {
    if [[ $# -eq 0 ]]; then
        print_help
        exit 0
    fi
    
    command=$1
    shift
    
    case $command in
        login)
            login "$@"
            ;;
        user)
            if [[ "$1" == "list" ]]; then
                shift
                user_list "$@"
            else
                echo -e "${RED}错误: 未知的user子命令 '$1'${NC}"
                print_help
                exit 1
            fi
            ;;
        vm)
            if [[ "$1" == "list" ]]; then
                shift
                vm_list "$@"
            elif [[ "$1" == "create" ]]; then
                shift
                vm_create "$@"
            elif [[ "$1" == "get" ]]; then
                shift
                vm_get "$@"
            elif [[ "$1" == "update" ]]; then
                shift
                vm_update "$@"
            elif [[ "$1" == "delete" ]]; then
                shift
                vm_delete "$@"
            elif [[ "$1" == "start" ]]; then
                shift
                vm_start "$@"
            elif [[ "$1" == "stop" ]]; then
                shift
                vm_stop "$@"
            elif [[ "$1" == "restart" ]]; then
                shift
                vm_restart "$@"
            elif [[ "$1" == "migrate" ]]; then
                shift
                vm_migrate "$@"
            elif [[ "$1" == "attach" ]]; then
                shift
                vm_attach "$@"
            elif [[ "$1" == "detach" ]]; then
                shift
                vm_detach "$@"
            elif [[ "$1" == "volumes" ]]; then
                shift
                vm_volumes "$@"
            else
                echo -e "${RED}错误: 未知的vm子命令 '$1'${NC}"
                print_help
                exit 1
            fi
            ;;
        node)
            if [[ "$1" == "list" ]]; then
                shift
                node_list "$@"
            elif [[ "$1" == "create" ]]; then
                shift
                node_create "$@"
            elif [[ "$1" == "get" ]]; then
                shift
                node_get "$@"
            elif [[ "$1" == "update" ]]; then
                shift
                node_update "$@"
            elif [[ "$1" == "delete" ]]; then
                shift
                node_delete "$@"
            elif [[ "$1" == "heartbeat" ]]; then
                shift
                node_heartbeat "$@"
            elif [[ "$1" == "stats" ]]; then
                shift
                node_stats "$@"
            else
                echo -e "${RED}错误: 未知的node子命令 '$1'${NC}"
                print_help
                exit 1
            fi
            ;;
        pool)
            if [[ "$1" == "list" ]]; then
                shift
                pool_list "$@"
            elif [[ "$1" == "create" ]]; then
                shift
                pool_create "$@"
            elif [[ "$1" == "get" ]]; then
                shift
                pool_get "$@"
            elif [[ "$1" == "update" ]]; then
                shift
                pool_update "$@"
            elif [[ "$1" == "delete" ]]; then
                shift
                pool_delete "$@"
            else
                echo -e "${RED}错误: 未知的pool子命令 '$1'${NC}"
                print_help
                exit 1
            fi
            ;;
        volume)
            if [[ "$1" == "list" ]]; then
                shift
                volume_list "$@"
            elif [[ "$1" == "create" ]]; then
                shift
                volume_create "$@"
            elif [[ "$1" == "get" ]]; then
                shift
                volume_get "$@"
            elif [[ "$1" == "delete" ]]; then
                shift
                volume_delete "$@"
            elif [[ "$1" == "resize" ]]; then
                shift
                volume_resize "$@"
            elif [[ "$1" == "snapshot" ]]; then
                shift
                volume_snapshot "$@"
            else
                echo -e "${RED}错误: 未知的volume子命令 '$1'${NC}"
                print_help
                exit 1
            fi
            ;;
        network)
            if [[ "$1" == "list" ]]; then
                shift
                network_list "$@"
            elif [[ "$1" == "create" ]]; then
                shift
                network_create "$@"
            elif [[ "$1" == "get" ]]; then
                shift
                network_get "$@"
            elif [[ "$1" == "update" ]]; then
                shift
                network_update "$@"
            elif [[ "$1" == "delete" ]]; then
                shift
                network_delete "$@"
            elif [[ "$1" == "ips" ]]; then
                shift
                network_ips "$@"
            else
                echo -e "${RED}错误: 未知的network子命令 '$1'${NC}"
                print_help
                exit 1
            fi
            ;;
        help|--help|-h)
            print_help
            ;;
        *)
            echo -e "${RED}错误: 未知命令 '$command'${NC}"
            print_help
            exit 1
            ;;
    esac
}

main "$@"


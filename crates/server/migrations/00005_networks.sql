-- 网络表
CREATE TABLE IF NOT EXISTS networks (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,  -- bridge, ovs, macvlan
    cidr VARCHAR(50),
    gateway VARCHAR(45),
    mtu INTEGER DEFAULT 1500,
    vlan_id INTEGER,
    
    -- 元数据
    metadata JSONB,
    
    -- 时间戳
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- IP 分配表
CREATE TABLE IF NOT EXISTS ip_allocations (
    id VARCHAR(36) PRIMARY KEY,
    network_id VARCHAR(36) NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    ip_address VARCHAR(45) NOT NULL,
    mac_address VARCHAR(17),
    vm_id VARCHAR(36) REFERENCES vms(id) ON DELETE SET NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'available',  -- available, allocated, reserved
    
    allocated_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_networks_type ON networks(type);
CREATE INDEX IF NOT EXISTS idx_ip_allocations_network_id ON ip_allocations(network_id);
CREATE INDEX IF NOT EXISTS idx_ip_allocations_ip_address ON ip_allocations(ip_address);
CREATE INDEX IF NOT EXISTS idx_ip_allocations_vm_id ON ip_allocations(vm_id);
CREATE INDEX IF NOT EXISTS idx_ip_allocations_status ON ip_allocations(status);


-- 节点表
CREATE TABLE IF NOT EXISTS nodes (
    id VARCHAR(36) PRIMARY KEY,
    hostname VARCHAR(255) NOT NULL,
    ip_address VARCHAR(45) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'offline',  -- online, offline, maintenance, error
    hypervisor_type VARCHAR(50),  -- kvm, qemu, xen
    hypervisor_version VARCHAR(50),
    
    -- 资源信息
    cpu_cores INTEGER,
    cpu_threads INTEGER,
    memory_total BIGINT,  -- bytes
    disk_total BIGINT,  -- bytes
    
    -- 元数据
    metadata JSONB,
    
    -- 时间戳
    last_heartbeat TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);
CREATE INDEX IF NOT EXISTS idx_nodes_ip_address ON nodes(ip_address);
CREATE INDEX IF NOT EXISTS idx_nodes_last_heartbeat ON nodes(last_heartbeat);


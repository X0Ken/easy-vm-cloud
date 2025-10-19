-- 虚拟机表
CREATE TABLE IF NOT EXISTS vms (
    id VARCHAR(36) PRIMARY KEY,
    uuid VARCHAR(36) UNIQUE,  -- libvirt UUID
    name VARCHAR(255) NOT NULL,
    node_id VARCHAR(36) REFERENCES nodes(id) ON DELETE SET NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'stopped',  -- running, stopped, paused, migrating, error
    
    -- 配置信息
    vcpu INTEGER NOT NULL,
    memory_mb BIGINT NOT NULL,
    os_type VARCHAR(20) DEFAULT 'linux',  -- 操作系统类型: linux, windows
    
    -- 磁盘和网络配置 (JSON)
    disk_ids JSONB,
    network_interfaces JSONB,
    
    -- 元数据
    metadata JSONB,
    
    -- 时间戳
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    stopped_at TIMESTAMP WITH TIME ZONE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_vms_node_id ON vms(node_id);
CREATE INDEX IF NOT EXISTS idx_vms_status ON vms(status);
CREATE INDEX IF NOT EXISTS idx_vms_name ON vms(name);
CREATE INDEX IF NOT EXISTS idx_vms_uuid ON vms(uuid);
CREATE INDEX IF NOT EXISTS idx_vms_os_type ON vms(os_type);


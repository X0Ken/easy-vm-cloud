-- 存储池表
CREATE TABLE IF NOT EXISTS storage_pools (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    type VARCHAR(50) NOT NULL,  -- nfs, lvm, ceph, iscsi
    status VARCHAR(50) NOT NULL DEFAULT 'active',  -- active, inactive, error
    
    -- 存储池配置（根据类型不同，配置不同）
    -- NFS: {server: "192.168.1.100", export_path: "/mnt/nfs/vm-storage"}
    -- Ceph: {monitors: ["192.168.1.10:6789"], pool: "vms", user: "admin"}
    -- LVM: {vg_name: "vg_vm"}
    config JSONB NOT NULL,
    
    -- 容量信息
    capacity_gb BIGINT,
    allocated_gb BIGINT DEFAULT 0,
    available_gb BIGINT,
    
    -- 关联信息
    node_id VARCHAR(36) REFERENCES nodes(id) ON DELETE SET NULL,
    
    -- 元数据
    metadata JSONB,
    
    -- 时间戳
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 存储卷表
CREATE TABLE IF NOT EXISTS volumes (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,  -- lvm, qcow2, raw, ceph, nfs
    size_gb BIGINT NOT NULL,
    pool_id VARCHAR(36) NOT NULL REFERENCES storage_pools(id) ON DELETE RESTRICT,
    path TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'available',  -- available, in-use, creating, deleting, error
    
    -- 关联信息
    vm_id VARCHAR(36) REFERENCES vms(id) ON DELETE SET NULL,
    
    -- 元数据
    metadata JSONB,
    
    -- 时间戳
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 快照表
CREATE TABLE IF NOT EXISTS snapshots (
    id VARCHAR(36) PRIMARY KEY,
    volume_id VARCHAR(36) NOT NULL REFERENCES volumes(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    size_gb BIGINT,
    
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_storage_pools_type ON storage_pools(type);
CREATE INDEX IF NOT EXISTS idx_storage_pools_status ON storage_pools(status);
CREATE INDEX IF NOT EXISTS idx_storage_pools_node_id ON storage_pools(node_id);
CREATE INDEX IF NOT EXISTS idx_volumes_pool_id ON volumes(pool_id);
CREATE INDEX IF NOT EXISTS idx_volumes_vm_id ON volumes(vm_id);
CREATE INDEX IF NOT EXISTS idx_volumes_type ON volumes(type);
CREATE INDEX IF NOT EXISTS idx_volumes_status ON volumes(status);
CREATE INDEX IF NOT EXISTS idx_snapshots_volume_id ON snapshots(volume_id);


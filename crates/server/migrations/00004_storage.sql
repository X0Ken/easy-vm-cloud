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
    name VARCHAR(255) NOT NULL,
    volume_id VARCHAR(36) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'creating',
    size_gb BIGINT,
    snapshot_tag VARCHAR(255),
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_snapshot_volume FOREIGN KEY (volume_id)
        REFERENCES volumes(id) ON DELETE CASCADE
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
CREATE INDEX idx_snapshots_volume_id ON snapshots(volume_id);
CREATE INDEX idx_snapshots_status ON snapshots(status);
CREATE INDEX idx_snapshots_created_at ON snapshots(created_at DESC);

-- 添加注释
COMMENT ON TABLE snapshots IS '存储卷快照表';
COMMENT ON COLUMN snapshots.id IS '快照唯一标识';
COMMENT ON COLUMN snapshots.name IS '快照名称';
COMMENT ON COLUMN snapshots.volume_id IS '关联的存储卷ID';
COMMENT ON COLUMN snapshots.status IS '快照状态: creating-创建中, available-可用, deleting-删除中, error-错误';
COMMENT ON COLUMN snapshots.size_gb IS '快照大小(GB)';
COMMENT ON COLUMN snapshots.snapshot_tag IS 'qemu/libvirt中的实际快照标签';
COMMENT ON COLUMN snapshots.description IS '快照描述';
COMMENT ON COLUMN snapshots.metadata IS '快照元数据(JSON格式)';
COMMENT ON COLUMN snapshots.created_at IS '创建时间';
COMMENT ON COLUMN snapshots.updated_at IS '更新时间';

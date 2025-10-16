-- 任务表
CREATE TABLE IF NOT EXISTS tasks (
    id VARCHAR(36) PRIMARY KEY,
    task_type VARCHAR(50) NOT NULL,  -- create_vm, delete_vm, migrate_vm, etc.
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, running, completed, failed, cancelled
    progress INTEGER DEFAULT 0,  -- 0-100
    
    -- 任务数据
    payload JSONB NOT NULL,
    result JSONB,
    error_message TEXT,
    
    -- 关联信息
    target_type VARCHAR(50),  -- vm, node, volume, network
    target_id VARCHAR(36),
    node_id VARCHAR(36) REFERENCES nodes(id) ON DELETE SET NULL,
    
    -- 重试信息
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    
    -- 用户信息
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    
    -- 时间戳
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_type ON tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_tasks_target ON tasks(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_tasks_created_by ON tasks(created_by);
CREATE INDEX IF NOT EXISTS idx_tasks_node_id ON tasks(node_id);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);


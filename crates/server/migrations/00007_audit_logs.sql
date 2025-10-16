-- 审计日志表
CREATE TABLE IF NOT EXISTS audit_logs (
    id VARCHAR(36) PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    username VARCHAR(255),
    action VARCHAR(100) NOT NULL,  -- login, create, update, delete, etc.
    target_type VARCHAR(50),  -- vm, node, volume, network, user
    target_id VARCHAR(36),
    target_name VARCHAR(255),
    
    -- 详细信息
    detail JSONB,
    ip_address VARCHAR(45),
    user_agent TEXT,
    
    -- 结果
    success BOOLEAN NOT NULL DEFAULT TRUE,
    error_message TEXT,
    
    -- 时间戳
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_target ON audit_logs(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_ip_address ON audit_logs(ip_address);


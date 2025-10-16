#!/bin/bash

source .env
# 从环境变量获取数据库配置
export PGPASSWORD=$POSTGRES_PASSWORD

# 创建数据库（如果不存在）
psql -h localhost -U $POSTGRES_USER -t -c "SELECT 1 FROM pg_database WHERE datname = '$POSTGRES_DB'" | grep -q 1 || psql -h localhost -U $POSTGRES_USER -c "CREATE DATABASE $POSTGRES_DB"

echo "数据库 $POSTGRES_DB 已就绪"

# 运行迁移脚本
MIGRATIONS_DIR="crates/server/migrations"
echo "开始执行数据库迁移..."

for migration in $(ls $MIGRATIONS_DIR/*.sql | sort); do
    echo "执行迁移: $(basename $migration)"
    psql -h localhost -U $POSTGRES_USER -d $POSTGRES_DB -f $migration
done

echo "✅ 数据库迁移完成"


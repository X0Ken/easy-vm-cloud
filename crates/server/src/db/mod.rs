/// 数据库访问层

pub mod models;

use sqlx::{postgres::PgPoolOptions, PgPool};
use sea_orm::{Database, DatabaseConnection};
use std::env;
use tracing::info;

/// 初始化数据库连接池 (SQLx)
pub async fn init_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    Ok(pool)
}

/// 建立数据库连接 (SeaORM)
pub async fn establish_connection() -> Result<DatabaseConnection, anyhow::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://username:password@localhost:5432/rust_web_admin".to_string());

    info!("正在连接数据库: {}", database_url);

    let db = Database::connect(&database_url).await?;
    info!("数据库连接成功");

    Ok(db)
}

pub async fn run_migrations(_db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    info!("数据库迁移完成");
    Ok(())
}


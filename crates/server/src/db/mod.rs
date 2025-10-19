/// 数据库访问层

pub mod models;

use sea_orm::{Database, DatabaseConnection};
use tracing::info;

/// 建立数据库连接 (SeaORM)
pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection, anyhow::Error> {
    info!("正在连接数据库: {}", database_url);

    let db = Database::connect(database_url).await?;
    info!("数据库连接成功");

    Ok(db)
}


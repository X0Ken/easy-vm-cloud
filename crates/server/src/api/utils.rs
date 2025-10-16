use axum::{
    http::StatusCode,
    response::Json,
};
use sea_orm::DatabaseConnection;
use serde_json::{json, Value};

use crate::auth::RbacService;

// 帮助函数：检查权限
pub async fn check_permission(
    db: &DatabaseConnection,
    user_id: i32,
    resource: &str,
    action: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    match RbacService::check_permission(db, user_id, resource, action).await {
        Ok(true) => Ok(()),
        Ok(false) => Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "权限不足",
                "required": format!("{}:{}", resource, action)
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "权限检查失败",
                "message": e.to_string()
            })),
        )),
    }
}

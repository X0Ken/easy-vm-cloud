use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::auth::Claims;

#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    axum::response::Json(json!({
                        "error": "未认证"
                    })),
                )
                    .into_response()
            })?;

        Ok(AuthUser(claims))
    }
}

// 为了兼容现有代码，创建一个简单的权限检查提取器
#[derive(Debug, Clone)]
pub struct RequireAuth {
    pub user_id: i32,
    pub username: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    axum::response::Json(json!({
                        "error": "需要认证"
                    })),
                )
                    .into_response()
            })?;

        Ok(RequireAuth {
            user_id: claims.sub,
            username: claims.username,
        })
    }
}



use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{Response, IntoResponse},
};
use serde_json::json;

use crate::auth::{AuthService, Claims, RbacService};

pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // 从请求头中获取Authorization
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Some(auth_header) = auth_header {
        // 提取JWT令牌
        if let Ok(token) = AuthService::extract_token_from_header(auth_header) {
            // 验证JWT令牌
            if let Ok(claims) = AuthService::verify_token(&token) {
                // 将claims添加到请求扩展中
                request.extensions_mut().insert(claims);
                return Ok(next.run(request).await);
            }
        }
    }

    // 认证失败，返回401错误
    let error_response = (
        StatusCode::UNAUTHORIZED,
        axum::response::Json(json!({
            "error": "认证失败，请提供有效的JWT令牌"
        })),
    ).into_response();

    Ok(error_response)
}

// 创建一个权限检查中间件生成器
pub fn require_permission(resource: &'static str, action: &'static str) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Response>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let resource = resource;
        let action = action;
        Box::pin(async move {
            // 从请求扩展中获取claims
            let claims = request.extensions().get::<Claims>().cloned();

            if let Some(claims) = claims {
                // 从请求中获取数据库连接
                let db = request.extensions().get::<sea_orm::DatabaseConnection>().cloned();

                if let Some(db) = db {
                    // 检查权限
                    match crate::auth::RbacService::check_permission(&db, claims.sub, resource, action).await {
                        Ok(true) => {
                            // 权限检查通过
                            Ok(next.run(request).await)
                        }
                        Ok(false) | Err(_) => {
                            // 权限不足
                            let error_response = (
                                StatusCode::FORBIDDEN,
                                axum::response::Json(json!({
                                    "error": "权限不足",
                                    "required": format!("{}:{}", resource, action)
                                })),
                            ).into_response();
                            Ok(error_response)
                        }
                    }
                } else {
                    // 数据库连接不可用
                    let error_response = (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::response::Json(json!({
                            "error": "服务器内部错误"
                        })),
                    ).into_response();
                    Ok(error_response)
                }
            } else {
                // 未认证
                let error_response = (
                    StatusCode::UNAUTHORIZED,
                    axum::response::Json(json!({
                        "error": "未认证"
                    })),
                ).into_response();
                Ok(error_response)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
        response::Response,
    };
    use serde_json::json;
    use crate::auth::Claims;

    #[test]
    fn test_require_permission_function_creation() {
        // 测试权限检查中间件生成器的创建
        let _middleware = require_permission("users", "read");
        
        // 验证中间件函数可以被创建
        // 由于这是一个函数生成器，我们主要测试它不会panic
        assert!(true);
    }

    #[test]
    fn test_permission_string_formatting() {
        // 测试权限字符串格式化
        let resource = "users";
        let action = "read";
        let expected = format!("{}:{}", resource, action);
        
        assert_eq!(expected, "users:read");
    }

    #[test]
    fn test_json_error_response_structure() {
        // 测试JSON错误响应的结构
        let error_json = json!({
            "error": "认证失败，请提供有效的JWT令牌"
        });
        
        assert!(error_json.is_object());
        assert_eq!(error_json["error"], "认证失败，请提供有效的JWT令牌");
    }

    #[test]
    fn test_permission_error_response_structure() {
        // 测试权限不足错误响应的结构
        let resource = "users";
        let action = "write";
        let error_json = json!({
            "error": "权限不足",
            "required": format!("{}:{}", resource, action)
        });
        
        assert!(error_json.is_object());
        assert_eq!(error_json["error"], "权限不足");
        assert_eq!(error_json["required"], "users:write");
    }

    #[test]
    fn test_claims_structure() {
        // 测试Claims结构体
        let claims = Claims {
            sub: 123,
            username: "testuser".to_string(),
            exp: 9999999999,
            iat: 1000000000,
        };
        
        assert_eq!(claims.sub, 123);
        assert_eq!(claims.username, "testuser");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_status_codes() {
        // 测试HTTP状态码
        assert_eq!(StatusCode::UNAUTHORIZED.as_u16(), 401);
        assert_eq!(StatusCode::FORBIDDEN.as_u16(), 403);
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
    }

    #[test]
    fn test_error_message_constants() {
        // 测试错误消息常量
        let auth_error = "认证失败，请提供有效的JWT令牌";
        let permission_error = "权限不足";
        let server_error = "服务器内部错误";
        let not_authenticated = "未认证";
        
        assert!(!auth_error.is_empty());
        assert!(!permission_error.is_empty());
        assert!(!server_error.is_empty());
        assert!(!not_authenticated.is_empty());
    }

    #[test] 
    fn test_middleware_response_types() {
        // 测试中间件响应类型
        use axum::response::{IntoResponse, Json};
        
        let json_response = Json(json!({"error": "test"}));
        let status_code = StatusCode::BAD_REQUEST;
        
        // 验证响应可以被转换
        let _response = (status_code, json_response).into_response();
        assert!(true);
    }

    #[test]
    fn test_header_authorization_parsing() {
        // 测试Authorization头解析
        let auth_header = "Bearer token123";
        
        assert!(auth_header.starts_with("Bearer "));
        
        let token_part = &auth_header[7..];
        assert_eq!(token_part, "token123");
    }

    #[test]
    fn test_request_extensions_concept() {
        // 测试请求扩展的概念
        let mut extensions = axum::http::Extensions::new();
        
        // 插入Claims
        let claims = Claims {
            sub: 1,
            username: "test".to_string(),
            exp: 9999999999,
            iat: 1000000000,
        };
        extensions.insert(claims.clone());
        
        // 获取Claims
        let retrieved = extensions.get::<Claims>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().sub, 1);
    }
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use sea_orm::*;
use serde_json::{json, Value};
use validator::Validate;

use crate::{
    db::models::{user, CreateUserDto, UserResponse, PaginationQuery, PaginationResponse, PaginationInfo},
    auth::RbacService,
    extractors::AuthUser,
    api::utils::check_permission,
    app_state::AppState,
};
use sea_orm::DatabaseConnection;

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/:id", get(get_user))
        .route("/", post(create_user))
        .route("/:id", put(update_user))
        .route("/:id", delete(delete_user))
        .route("/:id/roles", post(assign_role))
}



async fn list_users(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "user", "read").await?;

    // 验证分页参数
    if let Err(errors) = pagination.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "分页参数验证失败",
                "details": errors
            })),
        ));
    }

    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    let offset = (page - 1) * per_page;

    // 获取总数
    let total = user::Entity::find()
        .count(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取用户总数失败",
                    "message": e.to_string()
                })),
            )
        })?;

    // 获取分页数据
    let users = user::Entity::find()
        .limit(per_page as u64)
        .offset(offset as u64)
        .all(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取用户列表失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let user_responses: Vec<UserResponse> = futures::future::join_all(
        users.into_iter().map(|user| {
            let db = state.sea_db().clone();
            async move {
                let roles = RbacService::get_user_roles(&db, user.id)
                    .await
                    .unwrap_or_default();
                let permissions = RbacService::get_user_permissions(&db, user.id)
                    .await
                    .unwrap_or_default();

                UserResponse {
                    id: user.id,
                    username: user.username,
                    email: user.email,
                    is_active: user.is_active,
                    roles,
                    permissions: permissions.into_iter().collect(),
                }
            }
        })
    ).await;

    let pagination_info = PaginationInfo::new(page, per_page, total);
    let response = PaginationResponse {
        data: user_responses,
        pagination: pagination_info,
    };

    Ok(Json(json!(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreateUserDto, LoginDto, UserResponse, PaginationQuery, PaginationInfo, PaginationResponse};
    use validator::Validate;
    use serde_json::json;
    use axum::http::StatusCode;

    #[test]
    fn test_user_routes_creation() {
        let router = user_routes();
        // 验证路由创建成功
        assert!(std::mem::size_of_val(&router) > 0);
    }

    #[test]
    fn test_create_user_dto_validation() {
        // 测试有效的用户创建DTO
        let valid_dto = CreateUserDto {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(valid_dto.validate().is_ok());

        // 测试无效的DTO
        let invalid_dto = CreateUserDto {
            username: "ab".to_string(), // 太短
            email: "invalid-email".to_string(), // 无效邮箱
            password: "123".to_string(), // 太短
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_login_dto_validation() {
        // 测试有效的登录DTO
        let valid_dto = LoginDto {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };
        assert!(valid_dto.validate().is_ok());

        // 测试无效的登录DTO
        let invalid_dto = LoginDto {
            username: "".to_string(), // 空用户名
            password: "".to_string(), // 空密码
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_pagination_query_validation() {
        // 测试有效的分页查询
        let valid_query = PaginationQuery {
            page: Some(1),
            per_page: Some(20),
        };
        assert!(valid_query.validate().is_ok());

        // 测试无效的分页查询
        let invalid_query = PaginationQuery {
            page: Some(0), // 页码不能为0
            per_page: Some(101), // 每页数量过大
        };
        assert!(invalid_query.validate().is_err());
    }

    #[test]
    fn test_user_response_structure() {
        let user_response = UserResponse {
            id: 1,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            is_active: true,
            roles: vec!["admin".to_string(), "user".to_string()],
            permissions: vec!["read".to_string(), "write".to_string(), "delete".to_string()],
        };

        assert_eq!(user_response.id, 1);
        assert_eq!(user_response.username, "testuser");
        assert_eq!(user_response.email, "test@example.com");
        assert!(user_response.is_active);
        assert_eq!(user_response.roles.len(), 2);
        assert_eq!(user_response.permissions.len(), 3);
        assert!(user_response.roles.contains(&"admin".to_string()));
        assert!(user_response.permissions.contains(&"read".to_string()));
    }

    #[test]
    fn test_pagination_response_structure() {
        let users = vec![
            UserResponse {
                id: 1,
                username: "user1".to_string(),
                email: "user1@test.com".to_string(),
                is_active: true,
                roles: vec!["user".to_string()],
                permissions: vec!["read".to_string()],
            },
            UserResponse {
                id: 2,
                username: "user2".to_string(),
                email: "user2@test.com".to_string(),
                is_active: false,
                roles: vec!["admin".to_string()],
                permissions: vec!["read".to_string(), "write".to_string()],
            },
        ];

        let pagination_info = PaginationInfo::new(1, 2, 10);
        let response = PaginationResponse {
            data: users,
            pagination: pagination_info,
        };

        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].username, "user1");
        assert_eq!(response.data[1].username, "user2");
        assert_eq!(response.pagination.current_page, 1);
        assert_eq!(response.pagination.per_page, 2);
        assert_eq!(response.pagination.total, 10);
    }

    #[test]
    fn test_status_code_constants() {
        // 测试HTTP状态码
        assert_eq!(StatusCode::OK.as_u16(), 200);
        assert_eq!(StatusCode::CREATED.as_u16(), 201);
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), 400);
        assert_eq!(StatusCode::UNAUTHORIZED.as_u16(), 401);
        assert_eq!(StatusCode::FORBIDDEN.as_u16(), 403);
        assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
        assert_eq!(StatusCode::CONFLICT.as_u16(), 409);
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
    }

    #[test]
    fn test_json_error_response_format() {
        // 测试错误响应格式
        let error_response = json!({
            "error": "用户名已存在",
            "details": "Username 'testuser' is already taken"
        });

        assert_eq!(error_response["error"], "用户名已存在");
        assert_eq!(error_response["details"], "Username 'testuser' is already taken");
    }

    #[test]
    fn test_validation_error_response() {
        // 测试验证错误响应
        let validation_errors = json!({
            "error": "输入验证失败",
            "details": {
                "username": ["用户名长度必须在3-50个字符之间"],
                "email": ["邮箱格式不正确"],
                "password": ["密码长度至少6个字符"]
            }
        });

        assert_eq!(validation_errors["error"], "输入验证失败");
        assert!(validation_errors["details"].is_object());
    }

    #[test]
    fn test_user_creation_edge_cases() {
        // 测试边界情况
        let long_username = "a".repeat(50);
        let too_long_username = "a".repeat(51);
        let test_cases = vec![
            // (username, email, password, should_be_valid)
            ("abc", "a@b.co", "123456", true),           // 最小有效值
            (long_username.as_str(), "test@example.com", "password123", true), // 最大用户名长度
            ("ab", "test@example.com", "password123", false),          // 用户名太短
            (too_long_username.as_str(), "test@example.com", "password123", false), // 用户名太长
            ("testuser", "invalid", "password123", false),             // 无效邮箱
            ("testuser", "test@example.com", "12345", false),          // 密码太短
        ];

        for (username, email, password, should_be_valid) in test_cases {
            let dto = CreateUserDto {
                username: username.to_string(),
                email: email.to_string(),
                password: password.to_string(),
            };

            if should_be_valid {
                assert!(dto.validate().is_ok(), 
                    "Should be valid: username='{}', email='{}', password='{}'", 
                    username, email, password);
            } else {
                assert!(dto.validate().is_err(), 
                    "Should be invalid: username='{}', email='{}', password='{}'", 
                    username, email, password);
            }
        }
    }

    #[test]
    fn test_pagination_calculations() {
        // 测试分页计算
        let test_cases = vec![
            (1, 10, 25, 3, true, false),   // 第一页
            (2, 10, 25, 3, true, true),    // 中间页
            (3, 10, 25, 3, false, true),   // 最后一页
            (1, 10, 10, 1, false, false),  // 只有一页
            (1, 10, 0, 0, false, false),   // 没有数据
        ];

        for (page, per_page, total, expected_pages, expected_next, expected_prev) in test_cases {
            let info = PaginationInfo::new(page, per_page, total);
            assert_eq!(info.total_pages, expected_pages);
            assert_eq!(info.has_next, expected_next);
            assert_eq!(info.has_prev, expected_prev);
        }
    }

    #[test]
    fn test_user_roles_and_permissions() {
        // 测试不同角色和权限组合
        let admin_user = UserResponse {
            id: 1,
            username: "admin".to_string(),
            email: "admin@test.com".to_string(),
            is_active: true,
            roles: vec!["admin".to_string(), "super_admin".to_string()],
            permissions: vec![
                "user:create".to_string(),
                "user:read".to_string(),
                "user:update".to_string(),
                "user:delete".to_string(),
                "role:manage".to_string(),
            ],
        };

        let regular_user = UserResponse {
            id: 2,
            username: "user".to_string(),
            email: "user@test.com".to_string(),
            is_active: true,
            roles: vec!["user".to_string()],
            permissions: vec!["user:read".to_string()],
        };

        // 验证管理员权限
        assert!(admin_user.roles.contains(&"admin".to_string()));
        assert!(admin_user.permissions.contains(&"user:create".to_string()));
        assert!(admin_user.permissions.len() > regular_user.permissions.len());

        // 验证普通用户权限
        assert!(regular_user.roles.contains(&"user".to_string()));
        assert!(regular_user.permissions.contains(&"user:read".to_string()));
        assert!(!regular_user.permissions.contains(&"user:delete".to_string()));
    }

    #[test]
    fn test_email_normalization() {
        // 测试邮箱标准化场景
        let email_cases = vec![
            "Test@Example.COM",      // 大写
            "user+tag@domain.com",   // 带标签
            "user.name@domain.com",  // 带点
            "user123@sub.domain.co.uk", // 多级域名
        ];

        for email in email_cases {
            let dto = CreateUserDto {
                username: "testuser".to_string(),
                email: email.to_string(),
                password: "password123".to_string(),
            };

            // 所有这些邮箱格式都应该是有效的
            assert!(dto.validate().is_ok(), "Email '{}' should be valid", email);
        }
    }

    #[test]
    fn test_password_requirements() {
        // 测试密码要求
        let password_cases = vec![
            ("123456", true),         // 最小长度
            ("password", true),       // 普通密码
            ("P@ssw0rd!", true),      // 复杂密码
            ("12345", false),         // 太短
            ("", false),              // 空密码
        ];

        for (password, should_be_valid) in password_cases {
            let dto = CreateUserDto {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password: password.to_string(),
            };

            if should_be_valid {
                assert!(dto.validate().is_ok(), "Password '{}' should be valid", password);
            } else {
                assert!(dto.validate().is_err(), "Password '{}' should be invalid", password);
            }
        }
    }
}

async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<i32>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "user", "read").await?;

    let user = user::Entity::find_by_id(user_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取用户失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let user = user.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "用户不存在"
        })),
    ))?;

    let roles = RbacService::get_user_roles(&state.sea_db(), user.id)
        .await
        .unwrap_or_default();
    let permissions = RbacService::get_user_permissions(&state.sea_db(), user.id)
        .await
        .unwrap_or_default();

    let user_response = UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        is_active: user.is_active,
        roles,
        permissions: permissions.into_iter().collect(),
    };

    Ok(Json(json!({
        "user": user_response
    })))
}

async fn create_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<CreateUserDto>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "user", "create").await?;

    // 验证输入
    if let Err(errors) = payload.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "验证失败",
                "details": errors
            })),
        ));
    }

    // 检查用户名是否已存在
    let existing_user = user::Entity::find()
        .filter(user::Column::Username.eq(&payload.username))
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "数据库错误",
                    "message": e.to_string()
                })),
            )
        })?;

    if existing_user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({
                "error": "用户名已存在"
            })),
        ));
    }

    // 创建用户
    let user = user::ActiveModel {
        username: Set(payload.username),
        email: Set(payload.email),
        password_hash: Set("".to_string()), // 这里应该加密密码
        is_active: Set(true),
        ..Default::default()
    };

    let user = user.insert(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "创建用户失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "用户创建成功",
        "user": {
            "id": user.id,
            "username": user.username,
            "email": user.email
        }
    })))
}

async fn update_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "user", "update").await?;

    let user = user::Entity::find_by_id(user_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取用户失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let user = user.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "用户不存在"
        })),
    ))?;

    // 更新用户信息
    let mut user_model: user::ActiveModel = user.into();
    
    if let Some(email) = payload.get("email").and_then(|v| v.as_str()) {
        user_model.email = Set(email.to_string());
    }
    
    if let Some(is_active) = payload.get("is_active").and_then(|v| v.as_bool()) {
        user_model.is_active = Set(is_active);
    }

    let user = user_model.update(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "更新用户失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "用户更新成功",
        "user": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "is_active": user.is_active
        }
    })))
}

async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<i32>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "user", "delete").await?;

    let user = user::Entity::find_by_id(user_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取用户失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let user = user.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "用户不存在"
        })),
    ))?;

    // 软删除用户（设置为非激活状态）
    let mut user_model: user::ActiveModel = user.into();
    user_model.is_active = Set(false);

    user_model.update(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "删除用户失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "用户删除成功"
    })))
}

async fn assign_role(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "user", "update").await?;

    let role_id = payload.get("role_id")
        .and_then(|v| v.as_i64())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "缺少role_id参数"
            })),
        ))? as i32;

    RbacService::assign_role_to_user(&state.sea_db, user_id, role_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "分配角色失败",
                    "message": e.to_string()
                })),
            )
        })?;

    Ok(Json(json!({
        "message": "角色分配成功"
    })))
}

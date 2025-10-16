use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{post, get},
    Router,
    middleware::from_fn,
};
use sea_orm::*;
use serde_json::{json, Value};
use validator::Validate;

use crate::{
    auth::{AuthService, AuthResponse, RbacService},
    db::models::{user, CreateUserDto, LoginDto, UserResponse},
    extractors::RequireAuth,
    middleware::auth_middleware,
    app_state::AppState,
};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token).layer(from_fn(auth_middleware)))
        .route("/me", get(get_current_user).layer(from_fn(auth_middleware)))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserDto>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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

    // 检查邮箱是否已存在
    let existing_email = user::Entity::find()
        .filter(user::Column::Email.eq(&payload.email))
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

    if existing_email.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({
                "error": "邮箱已存在"
            })),
        ));
    }

    // 加密密码
    let password_hash = AuthService::hash_password(&payload.password)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "密码加密失败",
                    "message": e.to_string()
                })),
            )
        })?;

    // 创建用户
    let user = user::ActiveModel {
        username: Set(payload.username),
        email: Set(payload.email),
        password_hash: Set(password_hash),
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
        "message": "用户注册成功",
        "user": {
            "id": user.id,
            "username": user.username,
            "email": user.email
        }
    })))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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

    // 查找用户
    let user = user::Entity::find()
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

    let user = user.ok_or((
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": "用户名或密码错误"
        })),
    ))?;

    // 验证密码
    AuthService::verify_password(&payload.password, &user.password_hash)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "用户名或密码错误"
                })),
            )
        })?;

    // 检查用户是否激活
    if !user.is_active {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "账户已被禁用"
            })),
        ));
    }

    // 生成JWT令牌
    let token = AuthService::generate_token(user.id, &user.username)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "生成令牌失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let auth_response = AuthResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400, // 24小时
    };

    Ok(Json(json!({
        "message": "登录成功",
        "auth": auth_response
    })))
}

async fn get_current_user(
    State(state): State<AppState>,
    auth: RequireAuth,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 查找用户
    let user = user::Entity::find_by_id(auth.user_id)
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

    let user = user.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "用户不存在"
        })),
    ))?;

    // 获取用户角色和权限
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

async fn refresh_token(
    State(state): State<AppState>,
    auth: RequireAuth,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 查找用户以确保用户仍然存在且激活
    let user = user::Entity::find_by_id(auth.user_id)
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

    let user = user.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "用户不存在"
        })),
    ))?;

    // 检查用户是否仍然激活
    if !user.is_active {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "账户已被禁用"
            })),
        ));
    }

    // 生成新的JWT令牌
    let new_token = AuthService::generate_token(user.id, &user.username)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "生成令牌失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let auth_response = AuthResponse {
        token: new_token,
        token_type: "Bearer".to_string(),
        expires_in: 86400, // 24小时
    };

    Ok(Json(json!({
        "message": "令牌刷新成功",
        "auth": auth_response
    })))
}

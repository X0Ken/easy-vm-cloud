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
    db::models::{role, CreateRoleDto, RoleResponse, PaginationQuery, PaginationResponse, PaginationInfo},
    auth::RbacService,
    extractors::AuthUser,
    api::utils::check_permission,
    app_state::AppState,
};

pub fn role_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_roles))
        .route("/:id", get(get_role))
        .route("/", post(create_role))
        .route("/:id", put(update_role))
        .route("/:id", delete(delete_role))
        .route("/:id/permissions", post(assign_permission))
        .route("/:id/permissions", delete(remove_permission))
}




async fn list_roles(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "role", "read").await?;

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
    let total = role::Entity::find()
        .count(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取角色总数失败",
                    "message": e.to_string()
                })),
            )
        })?;

    // 获取分页数据
    let roles = role::Entity::find()
        .limit(per_page as u64)
        .offset(offset as u64)
        .all(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取角色列表失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let role_responses: Vec<RoleResponse> = futures::future::join_all(
        roles.into_iter().map(|role| {
            let db = state.sea_db().clone();
            async move {
                let permissions = RbacService::get_role_permissions(&db, role.id)
                    .await
                    .unwrap_or_default();

                RoleResponse {
                    id: role.id,
                    name: role.name,
                    description: role.description,
                    is_active: role.is_active,
                    permissions: permissions.into_iter().collect(),
                }
            }
        })
    ).await;

    let pagination_info = PaginationInfo::new(page, per_page, total);
    let response = PaginationResponse {
        data: role_responses,
        pagination: pagination_info,
    };

    Ok(Json(json!(response)))
}

async fn get_role(
    State(state): State<AppState>,
    Path(role_id): Path<i32>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "role", "read").await?;

    let role = role::Entity::find_by_id(role_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取角色失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let role = role.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "角色不存在"
        })),
    ))?;

    let permissions = RbacService::get_role_permissions(&state.sea_db(), role.id)
        .await
        .unwrap_or_default();

    let role_response = RoleResponse {
        id: role.id,
        name: role.name,
        description: role.description,
        is_active: role.is_active,
        permissions: permissions.into_iter().collect(),
    };

    Ok(Json(json!({
        "role": role_response
    })))
}

async fn create_role(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<CreateRoleDto>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "role", "create").await?;

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

    // 检查角色名是否已存在
    let existing_role = role::Entity::find()
        .filter(role::Column::Name.eq(&payload.name))
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

    if existing_role.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({
                "error": "角色名已存在"
            })),
        ));
    }

    // 创建角色
    let role = role::ActiveModel {
        name: Set(payload.name),
        description: Set(payload.description),
        is_active: Set(true),
        ..Default::default()
    };

    let role = role.insert(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "创建角色失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "角色创建成功",
        "role": {
            "id": role.id,
            "name": role.name,
            "description": role.description
        }
    })))
}

async fn update_role(
    State(state): State<AppState>,
    Path(role_id): Path<i32>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "role", "update").await?;

    let role = role::Entity::find_by_id(role_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取角色失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let role = role.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "角色不存在"
        })),
    ))?;

    // 更新角色信息
    let mut role_model: role::ActiveModel = role.into();
    
    if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
        role_model.name = Set(name.to_string());
    }
    
    if let Some(description) = payload.get("description").and_then(|v| v.as_str()) {
        role_model.description = Set(Some(description.to_string()));
    }
    
    if let Some(is_active) = payload.get("is_active").and_then(|v| v.as_bool()) {
        role_model.is_active = Set(is_active);
    }

    let role = role_model.update(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "更新角色失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "角色更新成功",
        "role": {
            "id": role.id,
            "name": role.name,
            "description": role.description,
            "is_active": role.is_active
        }
    })))
}

async fn delete_role(
    State(state): State<AppState>,
    Path(role_id): Path<i32>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "role", "delete").await?;

    let role = role::Entity::find_by_id(role_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取角色失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let role = role.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "角色不存在"
        })),
    ))?;

    // 软删除角色（设置为非激活状态）
    let mut role_model: role::ActiveModel = role.into();
    role_model.is_active = Set(false);

    role_model.update(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "删除角色失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "角色删除成功"
    })))
}

async fn assign_permission(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(role_id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "role", "update").await?;

    let permission_id = payload.get("permission_id")
        .and_then(|v| v.as_i64())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "缺少permission_id参数"
            })),
        ))? as i32;

    RbacService::assign_permission_to_role(&state.sea_db(), role_id, permission_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "分配权限失败",
                    "message": e.to_string()
                })),
            )
        })?;

    Ok(Json(json!({
        "message": "权限分配成功"
    })))
}

async fn remove_permission(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(role_id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "role", "update").await?;

    let permission_id = payload.get("permission_id")
        .and_then(|v| v.as_i64())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "缺少permission_id参数"
            })),
        ))? as i32;

    RbacService::remove_permission_from_role(&state.sea_db(), role_id, permission_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "移除权限失败",
                    "message": e.to_string()
                })),
            )
        })?;

    Ok(Json(json!({
        "message": "权限移除成功"
    })))
}

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
    db::models::{permission, CreatePermissionDto, PermissionResponse, PaginationQuery, PaginationResponse, PaginationInfo},
    extractors::AuthUser,
    api::utils::check_permission,
    app_state::AppState,
};

pub fn permission_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_permissions))
        .route("/:id", get(get_permission))
        .route("/", post(create_permission))
        .route("/:id", put(update_permission))
        .route("/:id", delete(delete_permission))
}

async fn list_permissions(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "permission", "read").await?;

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
    let total = permission::Entity::find()
        .count(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取权限总数失败",
                    "message": e.to_string()
                })),
            )
        })?;

    // 获取分页数据
    let permissions = permission::Entity::find()
        .limit(per_page as u64)
        .offset(offset as u64)
        .all(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取权限列表失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let permission_responses: Vec<PermissionResponse> = permissions
        .into_iter()
        .map(|p| PermissionResponse {
            id: p.id,
            name: p.name,
            description: p.description,
            resource: p.resource,
            action: p.action,
            is_active: p.is_active,
        })
        .collect();

    let pagination_info = PaginationInfo::new(page, per_page, total);
    let response = PaginationResponse {
        data: permission_responses,
        pagination: pagination_info,
    };

    Ok(Json(json!(response)))
}

async fn get_permission(
    State(state): State<AppState>,
    Path(permission_id): Path<i32>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "permission", "read").await?;

    let permission = permission::Entity::find_by_id(permission_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取权限失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let permission = permission.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "权限不存在"
        })),
    ))?;

    let permission_response = PermissionResponse {
        id: permission.id,
        name: permission.name,
        description: permission.description,
        resource: permission.resource,
        action: permission.action,
        is_active: permission.is_active,
    };

    Ok(Json(json!({
        "permission": permission_response
    })))
}

async fn create_permission(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<CreatePermissionDto>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "permission", "create").await?;

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

    // 检查权限名是否已存在
    let existing_permission = permission::Entity::find()
        .filter(permission::Column::Name.eq(&payload.name))
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

    if existing_permission.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({
                "error": "权限名已存在"
            })),
        ));
    }

    // 创建权限
    let permission = permission::ActiveModel {
        name: Set(payload.name),
        description: Set(payload.description),
        resource: Set(payload.resource),
        action: Set(payload.action),
        is_active: Set(true),
        ..Default::default()
    };

    let permission = permission.insert(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "创建权限失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "权限创建成功",
        "permission": {
            "id": permission.id,
            "name": permission.name,
            "description": permission.description,
            "resource": permission.resource,
            "action": permission.action
        }
    })))
}

async fn update_permission(
    State(state): State<AppState>,
    Path(permission_id): Path<i32>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "permission", "update").await?;

    let permission = permission::Entity::find_by_id(permission_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取权限失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let permission = permission.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "权限不存在"
        })),
    ))?;

    // 更新权限信息
    let mut permission_model: permission::ActiveModel = permission.into();
    
    if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
        permission_model.name = Set(name.to_string());
    }
    
    if let Some(description) = payload.get("description").and_then(|v| v.as_str()) {
        permission_model.description = Set(Some(description.to_string()));
    }
    
    if let Some(resource) = payload.get("resource").and_then(|v| v.as_str()) {
        permission_model.resource = Set(resource.to_string());
    }
    
    if let Some(action) = payload.get("action").and_then(|v| v.as_str()) {
        permission_model.action = Set(action.to_string());
    }
    
    if let Some(is_active) = payload.get("is_active").and_then(|v| v.as_bool()) {
        permission_model.is_active = Set(is_active);
    }

    let permission = permission_model.update(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "更新权限失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "权限更新成功",
        "permission": {
            "id": permission.id,
            "name": permission.name,
            "description": permission.description,
            "resource": permission.resource,
            "action": permission.action,
            "is_active": permission.is_active
        }
    })))
}

async fn delete_permission(
    State(state): State<AppState>,
    Path(permission_id): Path<i32>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 检查权限
    check_permission(&state.sea_db(), claims.sub, "permission", "delete").await?;

    let permission = permission::Entity::find_by_id(permission_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "获取权限失败",
                    "message": e.to_string()
                })),
            )
        })?;

    let permission = permission.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "权限不存在"
        })),
    ))?;

    // 软删除权限（设置为非激活状态）
    let mut permission_model: permission::ActiveModel = permission.into();
    permission_model.is_active = Set(false);

    permission_model.update(&state.sea_db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "删除权限失败",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "message": "权限删除成功"
    })))
}

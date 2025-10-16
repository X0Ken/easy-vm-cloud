use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    extractors::AuthUser,
    db::models::user_department::*,
    services::UserDepartmentService,
    utils::ApiResponse,
    app_state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct BatchAssignDto {
    pub user_ids: Vec<i32>,
    pub department_id: i32,
    pub position: Option<String>,
}

pub fn user_department_routes() -> Router<AppState> {
    Router::new()
        .route("/assign", post(assign_user_to_department))
        .route("/batch-assign", post(batch_assign_users))
        .route("/:id", get(get_user_department))
        .route("/:id", put(update_user_department))
        .route("/:id", delete(remove_user_from_department))
        .route("/user/:user_id", get(get_user_departments))
        .route("/department/:department_id", get(get_department_users))
        .route("/user/:user_id/primary", get(get_user_primary_department))
}

/// 为用户分配部门
async fn assign_user_to_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Json(dto): Json<CreateUserDepartmentDto>,
) -> Result<Json<ApiResponse<Model>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 验证输入
    if let Err(e) = dto.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("输入验证失败: {}", e))),
        ));
    }

    match UserDepartmentService::assign_user_to_department(&state.sea_db(), dto).await {
        Ok(user_department) => Ok(Json(ApiResponse::success(user_department))),
        Err(e) => {
            tracing::error!("分配用户到部门失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("分配用户到部门失败: {}", e))),
            ))
        }
    }
}

/// 批量分配用户到部门
async fn batch_assign_users(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Json(dto): Json<BatchAssignDto>,
) -> Result<Json<ApiResponse<Vec<Model>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match UserDepartmentService::batch_assign_users_to_department(&state.sea_db(), 
        dto.user_ids,
        dto.department_id,
        dto.position,
    ).await {
        Ok(results) => Ok(Json(ApiResponse::success(results))),
        Err(e) => {
            tracing::error!("批量分配用户到部门失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("批量分配用户到部门失败: {}", e))),
            ))
        }
    }
}

/// 获取用户部门关联详情
async fn get_user_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Option<UserDepartmentDto>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match UserDepartmentService::get_user_department(&state.sea_db(), id).await {
        Ok(user_department) => Ok(Json(ApiResponse::success(user_department))),
        Err(e) => {
            tracing::error!("获取用户部门关联详情失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取用户部门关联详情失败")),
            ))
        }
    }
}

/// 更新用户部门信息
async fn update_user_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<i32>,
    Json(dto): Json<UpdateUserDepartmentDto>,
) -> Result<Json<ApiResponse<Model>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 验证输入
    if let Err(e) = dto.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("输入验证失败: {}", e))),
        ));
    }

    match UserDepartmentService::update_user_department(&state.sea_db(), id, dto).await {
        Ok(user_department) => Ok(Json(ApiResponse::success(user_department))),
        Err(e) => {
            tracing::error!("更新用户部门信息失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("更新用户部门信息失败: {}", e))),
            ))
        }
    }
}

/// 移除用户部门关联
async fn remove_user_from_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<()>>)> {
    match UserDepartmentService::remove_user_from_department(&state.sea_db(), id).await {
        Ok(success) => {
            if success {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("用户部门关联不存在")),
                ))
            }
        }
        Err(e) => {
            tracing::error!("移除用户部门关联失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("移除用户部门关联失败: {}", e))),
            ))
        }
    }
}

/// 获取用户的所有部门
async fn get_user_departments(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(user_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<UserDepartmentDto>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match UserDepartmentService::get_user_departments(&state.sea_db(), user_id).await {
        Ok(user_departments) => Ok(Json(ApiResponse::success(user_departments))),
        Err(e) => {
            tracing::error!("获取用户部门列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取用户部门列表失败")),
            ))
        }
    }
}

/// 获取部门的所有用户
async fn get_department_users(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(department_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<UserDepartmentDto>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match UserDepartmentService::get_department_users(&state.sea_db(), department_id).await {
        Ok(users) => Ok(Json(ApiResponse::success(users))),
        Err(e) => {
            tracing::error!("获取部门用户列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取部门用户列表失败")),
            ))
        }
    }
}

/// 获取用户的主要部门
async fn get_user_primary_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(user_id): Path<i32>,
) -> Result<Json<ApiResponse<Option<UserDepartmentDto>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match UserDepartmentService::get_user_primary_department(&state.sea_db(), user_id).await {
        Ok(primary_department) => Ok(Json(ApiResponse::success(primary_department))),
        Err(e) => {
            tracing::error!("获取用户主要部门失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取用户主要部门失败")),
            ))
        }
    }
}

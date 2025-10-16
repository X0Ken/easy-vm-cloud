/// 存储管理接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::db::models::storage_pool::{CreateStoragePoolDto, UpdateStoragePoolDto, StoragePoolListResponse, StoragePoolResponse};
use crate::db::models::volume::{CreateVolumeDto, UpdateVolumeDto, ResizeVolumeDto, VolumeListResponse, VolumeResponse};
use crate::services::storage_service::StorageService;

/// API 错误响应
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse {
            error: status.canonical_reason().unwrap_or("Unknown").to_string(),
            message,
        });

        (status, body).into_response()
    }
}

#[derive(Debug)]
enum ApiError {
    NotFound(String),
    BadRequest(String),
    Conflict(String),
    Internal(String),
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

/// 存储池查询参数
#[derive(Debug, Deserialize)]
pub struct ListStoragePoolsQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub pool_type: Option<String>,
    pub status: Option<String>,
}

/// 存储卷查询参数
#[derive(Debug, Deserialize)]
pub struct ListVolumesQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub pool_id: Option<String>,
    pub node_id: Option<String>,
    pub status: Option<String>,
}

/// 创建快照请求
#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    pub snapshot_name: String,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

/// 创建路由
pub fn routes() -> Router<AppState> {
    Router::new()
        // 存储池路由
        .route("/pools", post(create_storage_pool))
        .route("/pools", get(list_storage_pools))
        .route("/pools/:pool_id", get(get_storage_pool))
        .route("/pools/:pool_id", put(update_storage_pool))
        .route("/pools/:pool_id", delete(delete_storage_pool))
        
        // 存储卷路由
        .route("/volumes", post(create_volume))
        .route("/volumes", get(list_volumes))
        .route("/volumes/:volume_id", get(get_volume))
        .route("/volumes/:volume_id", put(update_volume))
        .route("/volumes/:volume_id", delete(delete_volume))
        .route("/volumes/:volume_id/resize", post(resize_volume))
        .route("/volumes/:volume_id/snapshot", post(create_snapshot))
}

// ==================== 存储池接口 ====================

/// 创建存储池
async fn create_storage_pool(
    State(state): State<AppState>,
    Json(dto): Json<CreateStoragePoolDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let pool = service.create_storage_pool(dto).await?;
    Ok((StatusCode::CREATED, Json(pool)))
}

/// 获取存储池列表
async fn list_storage_pools(
    State(state): State<AppState>,
    Query(query): Query<ListStoragePoolsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let response = service.list_storage_pools(
        query.page,
        query.page_size,
        query.pool_type,
        query.status,
    ).await?;
    Ok(Json(response))
}

/// 获取单个存储池
async fn get_storage_pool(
    State(state): State<AppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let pool = service.get_storage_pool(&pool_id).await?;
    Ok(Json(pool))
}

/// 更新存储池
async fn update_storage_pool(
    State(state): State<AppState>,
    Path(pool_id): Path<String>,
    Json(dto): Json<UpdateStoragePoolDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let pool = service.update_storage_pool(&pool_id, dto).await?;
    Ok(Json(pool))
}

/// 删除存储池
async fn delete_storage_pool(
    State(state): State<AppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    service.delete_storage_pool(&pool_id).await
        .map_err(|err| {
            if err.to_string().contains("存储池下还有存储卷，无法删除") {
                ApiError::Conflict(err.to_string())
            } else {
                ApiError::from(err)
            }
        })?;
    Ok(StatusCode::NO_CONTENT)
}

// ==================== 存储卷接口 ====================

/// 创建存储卷
async fn create_volume(
    State(state): State<AppState>,
    Json(dto): Json<CreateVolumeDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let volume = service.create_volume(dto).await?;
    Ok((StatusCode::CREATED, Json(volume)))
}

/// 获取存储卷列表
async fn list_volumes(
    State(state): State<AppState>,
    Query(query): Query<ListVolumesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let response = service.list_volumes(
        query.page,
        query.page_size,
        query.pool_id,
        query.node_id,
        query.status,
    ).await?;
    Ok(Json(response))
}

/// 获取单个存储卷
async fn get_volume(
    State(state): State<AppState>,
    Path(volume_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let volume = service.get_volume(&volume_id).await?;
    Ok(Json(volume))
}

/// 更新存储卷
async fn update_volume(
    State(state): State<AppState>,
    Path(volume_id): Path<String>,
    Json(dto): Json<UpdateVolumeDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let volume = service.update_volume(&volume_id, dto).await?;
    Ok(Json(volume))
}

/// 删除存储卷
async fn delete_volume(
    State(state): State<AppState>,
    Path(volume_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    service.delete_volume(&volume_id).await
        .map_err(|err| {
            if err.to_string().contains("存储卷正在被虚拟机使用，无法删除") {
                ApiError::Conflict(err.to_string())
            } else {
                ApiError::from(err)
            }
        })?;
    Ok(StatusCode::NO_CONTENT)
}

/// 调整存储卷大小
async fn resize_volume(
    State(state): State<AppState>,
    Path(volume_id): Path<String>,
    Json(dto): Json<ResizeVolumeDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let volume = service.resize_volume(&volume_id, dto).await?;
    Ok(Json(volume))
}

/// 创建快照
async fn create_snapshot(
    State(state): State<AppState>,
    Path(volume_id): Path<String>,
    Json(req): Json<CreateSnapshotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let service = StorageService::new(state);
    let snapshot_id = service.create_snapshot(&volume_id, req.snapshot_name).await?;
    
    #[derive(Serialize)]
    struct SnapshotResponse {
        snapshot_id: String,
    }
    
    Ok(Json(SnapshotResponse { snapshot_id }))
}


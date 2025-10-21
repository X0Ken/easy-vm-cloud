/// 快照管理接口
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::db::models::snapshot::{CreateSnapshotDto, UpdateSnapshotDto};
use crate::services::snapshot_service::SnapshotService;

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

/// 快照查询参数
#[derive(Debug, Deserialize)]
pub struct ListSnapshotsQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub volume_id: Option<String>,
    pub status: Option<String>,
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
        .route("/snapshots", post(create_snapshot))
        .route("/snapshots", get(list_snapshots))
        .route("/snapshots/:snapshot_id", get(get_snapshot))
        .route("/snapshots/:snapshot_id", put(update_snapshot))
        .route("/snapshots/:snapshot_id", delete(delete_snapshot))
        .route("/snapshots/:snapshot_id/restore", post(restore_snapshot))
}

// ==================== 快照接口 ====================

/// 创建快照
async fn create_snapshot(
    State(state): State<AppState>,
    Json(dto): Json<CreateSnapshotDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = SnapshotService::new(state);
    let snapshot = service.create_snapshot(dto).await?;
    Ok((StatusCode::CREATED, Json(snapshot)))
}

/// 获取快照列表
async fn list_snapshots(
    State(state): State<AppState>,
    Query(query): Query<ListSnapshotsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let service = SnapshotService::new(state);
    let response = service
        .list_snapshots(query.page, query.page_size, query.volume_id, query.status)
        .await?;
    Ok(Json(response))
}

/// 获取单个快照
async fn get_snapshot(
    State(state): State<AppState>,
    Path(snapshot_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = SnapshotService::new(state);
    let snapshot = service.get_snapshot(&snapshot_id).await?;
    Ok(Json(snapshot))
}

/// 删除快照
async fn delete_snapshot(
    State(state): State<AppState>,
    Path(snapshot_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = SnapshotService::new(state);
    service.delete_snapshot(&snapshot_id).await.map_err(|err| {
        if err.to_string().contains("快照不存在") {
            ApiError::NotFound(err.to_string())
        } else {
            ApiError::from(err)
        }
    })?;
    Ok(StatusCode::NO_CONTENT)
}

/// 恢复快照
async fn restore_snapshot(
    State(state): State<AppState>,
    Path(snapshot_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = SnapshotService::new(state);
    service
        .restore_snapshot(&snapshot_id)
        .await
        .map_err(|err| {
            if err.to_string().contains("需要先停止虚拟机") {
                ApiError::Conflict(err.to_string())
            } else if err.to_string().contains("不存在") {
                ApiError::NotFound(err.to_string())
            } else {
                ApiError::from(err)
            }
        })?;

    #[derive(Serialize)]
    struct RestoreResponse {
        message: String,
    }

    Ok(Json(RestoreResponse {
        message: "快照恢复成功".to_string(),
    }))
}

/// 更新快照
async fn update_snapshot(
    State(state): State<AppState>,
    Path(snapshot_id): Path<String>,
    Json(dto): Json<UpdateSnapshotDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = SnapshotService::new(state);
    let snapshot = service
        .update_snapshot(&snapshot_id, dto)
        .await
        .map_err(|err| {
            if err.to_string().contains("不存在") {
                ApiError::NotFound(err.to_string())
            } else {
                ApiError::from(err)
            }
        })?;
    Ok(Json(snapshot))
}

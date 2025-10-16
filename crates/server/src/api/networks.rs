/// 网络管理接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::db::models::network::{CreateNetworkDto, UpdateNetworkDto, NetworkListResponse, NetworkResponse};
use crate::db::models::ip_allocation::{IpAllocationListResponse, IpAllocationResponse};
use crate::services::network_service::NetworkService;

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
    Internal(String),
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

/// 网络查询参数
#[derive(Debug, Deserialize)]
pub struct ListNetworksQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub network_type: Option<String>,
}

/// IP 分配查询参数
#[derive(Debug, Deserialize)]
pub struct ListIpAllocationsQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub status: Option<String>,
}

/// 分配 IP 请求
#[derive(Debug, Deserialize)]
pub struct AllocateIpRequest {
    pub vm_id: String,
}

/// 释放 IP 请求
#[derive(Debug, Deserialize)]
pub struct ReleaseIpRequest {
    pub vm_id: String,
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
        // 网络路由
        .route("/", post(create_network))
        .route("/", get(list_networks))
        .route("/:network_id", get(get_network))
        .route("/:network_id", put(update_network))
        .route("/:network_id", delete(delete_network))
        
        // IP 分配路由
        .route("/:network_id/ips", get(list_ip_allocations))
}

// ==================== 网络接口 ====================

/// 创建网络
async fn create_network(
    State(state): State<AppState>,
    Json(dto): Json<CreateNetworkDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = NetworkService::new(state);
    let network = service.create_network(dto).await?;
    Ok((StatusCode::CREATED, Json(network)))
}

/// 获取网络列表
async fn list_networks(
    State(state): State<AppState>,
    Query(query): Query<ListNetworksQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let service = NetworkService::new(state);
    let response = service.list_networks(
        query.page,
        query.page_size,
        query.network_type,
    ).await?;
    Ok(Json(response))
}

/// 获取单个网络
async fn get_network(
    State(state): State<AppState>,
    Path(network_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = NetworkService::new(state);
    let network = service.get_network(&network_id).await?;
    Ok(Json(network))
}

/// 更新网络
async fn update_network(
    State(state): State<AppState>,
    Path(network_id): Path<String>,
    Json(dto): Json<UpdateNetworkDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = NetworkService::new(state);
    let network = service.update_network(&network_id, dto).await?;
    Ok(Json(network))
}

/// 删除网络
async fn delete_network(
    State(state): State<AppState>,
    Path(network_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let service = NetworkService::new(state);
    service.delete_network(&network_id).await?;
    Ok((StatusCode::NO_CONTENT, ()))
}

// ==================== IP 分配接口 ====================

/// 列出网络的 IP 分配
async fn list_ip_allocations(
    State(state): State<AppState>,
    Path(network_id): Path<String>,
    Query(query): Query<ListIpAllocationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let service = NetworkService::new(state);
    let response = service.list_ip_allocations(
        &network_id,
        query.page,
        query.page_size,
        query.status,
    ).await?;
    Ok(Json(response))
}

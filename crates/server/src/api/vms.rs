/// 虚拟机管理接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::db::models::vm::{CreateVmDto, UpdateVmDto, VmListResponse, VmResponse, AttachVolumeDto, DetachVolumeDto, VmDiskResponse};
use crate::services::vm_service::VmService;

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

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound("资源不存在".to_string()),
            _ => ApiError::Internal(err.to_string()),
        }
    }
}

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct ListVmsQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub node_id: Option<String>,
    pub status: Option<String>,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

/// 迁移请求
#[derive(Debug, Deserialize)]
pub struct MigrateVmRequest {
    pub target_node_id: String,
    #[serde(default)]
    pub live: bool,
}

/// 停止虚拟机请求
#[derive(Debug, Deserialize)]
pub struct StopVmRequest {
    #[serde(default)]
    pub force: bool,
}

/// VM 路由
pub fn vm_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_vms).post(create_vm))
        .route("/:id", get(get_vm).put(update_vm).delete(delete_vm))
        .route("/:id/start", post(start_vm))
        .route("/:id/stop", post(stop_vm))
        .route("/:id/restart", post(restart_vm))
        .route("/:id/migrate", post(migrate_vm))
        .route("/:id/volumes", get(list_vm_volumes))
        .route("/:id/volumes/attach", post(attach_volume))
        .route("/:id/volumes/detach", post(detach_volume))
        .route("/:id/networks", get(get_vm_networks))
}

/// 获取虚拟机列表
///
/// GET /api/vms?page=1&page_size=20&node_id=xxx&status=running
pub async fn list_vms(
    State(state): State<AppState>,
    Query(query): Query<ListVmsQuery>,
) -> Result<Json<VmListResponse>, ApiError> {
    let service = VmService::new(state.clone());

    let result = service
        .list_vms(query.page, query.page_size, query.node_id, query.status)
        .await?;

    Ok(Json(result))
}

/// 创建虚拟机
///
/// POST /api/vms
/// Body: CreateVmDto
pub async fn create_vm(
    State(state): State<AppState>,
    Json(dto): Json<CreateVmDto>,
) -> Result<(StatusCode, Json<VmResponse>), ApiError> {
    // 验证参数
    if dto.name.is_empty() {
        return Err(ApiError::BadRequest("虚拟机名称不能为空".to_string()));
    }

    if dto.vcpu == 0 {
        return Err(ApiError::BadRequest("CPU 核心数必须大于 0".to_string()));
    }

    if dto.memory_mb == 0 {
        return Err(ApiError::BadRequest("内存大小必须大于 0".to_string()));
    }

    // 验证操作系统类型
    if let Some(ref os_type) = dto.os_type {
        if os_type != "linux" && os_type != "windows" {
            return Err(ApiError::BadRequest("操作系统类型必须是 'linux' 或 'windows'".to_string()));
        }
    }

    let service = VmService::new(state.clone());
    let result = service.create_vm(dto).await?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// 获取单个虚拟机详情
///
/// GET /api/vms/:id
pub async fn get_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<VmResponse>, ApiError> {
    let service = VmService::new(state.clone());

    let result = service
        .get_vm(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("虚拟机 {} 不存在", id)))?;

    Ok(Json(result))
}

/// 更新虚拟机
///
/// PUT /api/vms/:id
/// Body: UpdateVmDto
pub async fn update_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateVmDto>,
) -> Result<Json<VmResponse>, ApiError> {
    let service = VmService::new(state.clone());
    let result = service.update_vm(&id, dto).await?;

    Ok(Json(result))
}

/// 删除虚拟机
///
/// DELETE /api/vms/:id
pub async fn delete_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let service = VmService::new(state.clone());
    service.delete_vm(&id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// 启动虚拟机
///
/// POST /api/vms/:id/start
pub async fn start_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = VmService::new(state.clone());
    service.start_vm(&id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "虚拟机启动成功"
    })))
}

/// 停止虚拟机
///
/// POST /api/vms/:id/stop
/// Body: { "force": false }
pub async fn stop_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<StopVmRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = VmService::new(state.clone());
    service.stop_vm(&id, req.force).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "虚拟机停止成功"
    })))
}

/// 重启虚拟机
///
/// POST /api/vms/:id/restart
pub async fn restart_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = VmService::new(state.clone());
    service.restart_vm(&id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "虚拟机重启成功"
    })))
}

/// 迁移虚拟机
///
/// POST /api/vms/:id/migrate
/// Body: MigrateVmRequest
pub async fn migrate_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<MigrateVmRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.target_node_id.is_empty() {
        return Err(ApiError::BadRequest("目标节点 ID 不能为空".to_string()));
    }

    let service = VmService::new(state.clone());
    service
        .migrate_vm(&id, &req.target_node_id, req.live)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "虚拟机迁移已开始"
    })))
}

/// 附加存储卷到虚拟机
///
/// POST /api/vms/:id/volumes/attach
/// Body: AttachVolumeDto
pub async fn attach_volume(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<AttachVolumeDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if dto.volume_id.is_empty() {
        return Err(ApiError::BadRequest("存储卷 ID 不能为空".to_string()));
    }


    let service = VmService::new(state.clone());
    service.attach_volume(&id, dto).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "存储卷附加成功"
    })))
}

/// 从虚拟机分离存储卷
///
/// POST /api/vms/:id/volumes/detach
/// Body: DetachVolumeDto
pub async fn detach_volume(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<DetachVolumeDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if dto.volume_id.is_empty() {
        return Err(ApiError::BadRequest("存储卷 ID 不能为空".to_string()));
    }

    let service = VmService::new(state.clone());
    service.detach_volume(&id, dto).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "存储卷分离成功"
    })))
}

/// 获取虚拟机的所有存储卷
///
/// GET /api/vms/:id/volumes
pub async fn list_vm_volumes(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<VmDiskResponse>>, ApiError> {
    let service = VmService::new(state.clone());
    let volumes = service.list_vm_volumes(&id).await?;

    Ok(Json(volumes))
}


/// 获取虚拟机网络信息
///
/// GET /api/vms/:id/networks
pub async fn get_vm_networks(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let service = VmService::new(state.clone());
    let networks = service.list_vm_networks(&id).await?;

    Ok(Json(networks))
}

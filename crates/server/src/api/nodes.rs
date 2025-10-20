/// 节点管理接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json,
    Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    app_state::AppState, 
    services::node_service::NodeService,
    db::models::node::{CreateNodeDto, UpdateNodeDto, NodeResponse, NodeListResponse, NodeStatsResponse},
};

/// 节点路由
pub fn node_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_nodes).post(create_node))
        .route("/stats", get(get_stats))
        .route("/:id", get(get_node).put(update_node).delete(delete_node))
}

/// 分页查询参数
#[derive(Debug, Deserialize)]
pub struct ListNodesQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub status: Option<String>,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

/// 通用响应
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

/// 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

/// 创建节点
pub async fn create_node(
    State(state): State<AppState>,
    Json(dto): Json<CreateNodeDto>,
) -> Result<Json<NodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 验证输入
    if let Err(e) = dto.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: format!("验证失败: {}", e),
            }),
        ));
    }

    let service = NodeService::new(state);
    match service.create_node(dto).await {
        Ok(node) => Ok(Json(node)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("创建节点失败: {}", e),
            }),
        )),
    }
}

/// 获取节点列表
pub async fn list_nodes(
    State(state): State<AppState>,
    Query(query): Query<ListNodesQuery>,
) -> Result<Json<NodeListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = NodeService::new(state);
    match service.list_nodes(query.page, query.page_size, query.status).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("获取节点列表失败: {}", e),
            }),
        )),
    }
}

/// 获取单个节点详情
pub async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<NodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = NodeService::new(state);
    match service.get_node(&id).await {
        Ok(node) => Ok(Json(node)),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("节点不存在: {}", e),
            }),
        )),
    }
}

/// 更新节点
pub async fn update_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateNodeDto>,
) -> Result<Json<NodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 验证输入
    if let Err(e) = dto.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: format!("验证失败: {}", e),
            }),
        ));
    }

    let service = NodeService::new(state);
    match service.update_node(&id, dto).await {
        Ok(node) => Ok(Json(node)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("更新节点失败: {}", e),
            }),
        )),
    }
}

/// 删除节点
pub async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = NodeService::new(state);
    match service.delete_node(&id).await {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            message: "节点删除成功".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("删除节点失败: {}", e),
            }),
        )),
    }
}

/// 获取节点统计信息
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<NodeStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = NodeService::new(state);
    match service.get_stats().await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("获取统计信息失败: {}", e),
            }),
        )),
    }
}


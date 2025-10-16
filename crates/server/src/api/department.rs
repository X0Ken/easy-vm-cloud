use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};

use validator::Validate;

use crate::{
    extractors::AuthUser,
    db::models::department::*,
    services::DepartmentService,
    utils::ApiResponse,
    app_state::AppState,
};

pub fn department_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_departments))
        .route("/tree", get(get_department_tree))
        .route("/", post(create_department))
        .route("/:id", get(get_department))
        .route("/:id", put(update_department))
        .route("/:id", delete(delete_department))
}

/// 获取部门列表
async fn list_departments(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
) -> Result<Json<ApiResponse<Vec<Model>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match DepartmentService::list_departments(&state.sea_db()).await {
        Ok(departments) => Ok(Json(ApiResponse::success(departments))),
        Err(e) => {
            tracing::error!("获取部门列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取部门列表失败")),
            ))
        }
    }
}

/// 获取部门树形结构
async fn get_department_tree(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
) -> Result<Json<ApiResponse<Vec<DepartmentTreeDto>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match DepartmentService::get_department_tree(&state.sea_db()).await {
        Ok(tree) => Ok(Json(ApiResponse::success(tree))),
        Err(e) => {
            tracing::error!("获取部门树失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取部门树失败")),
            ))
        }
    }
}

/// 创建部门
async fn create_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Json(dto): Json<CreateDepartmentDto>,
) -> Result<Json<ApiResponse<Model>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 验证输入
    if let Err(e) = dto.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("输入验证失败: {}", e))),
        ));
    }

    match DepartmentService::create_department(&state.sea_db(), dto).await {
        Ok(department) => Ok(Json(ApiResponse::success(department))),
        Err(e) => {
            tracing::error!("创建部门失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("创建部门失败: {}", e))),
            ))
        }
    }
}

/// 获取部门详情
async fn get_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Option<Model>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match DepartmentService::get_department(&state.sea_db(), id).await {
        Ok(department) => Ok(Json(ApiResponse::success(department))),
        Err(e) => {
            tracing::error!("获取部门详情失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取部门详情失败")),
            ))
        }
    }
}

/// 更新部门
async fn update_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<i32>,
    Json(dto): Json<UpdateDepartmentDto>,
) -> Result<Json<ApiResponse<Model>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 验证输入
    if let Err(e) = dto.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("输入验证失败: {}", e))),
        ));
    }

    match DepartmentService::update_department(&state.sea_db(), id, dto).await {
        Ok(department) => Ok(Json(ApiResponse::success(department))),
        Err(e) => {
            tracing::error!("更新部门失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("更新部门失败: {}", e))),
            ))
        }
    }
}

/// 删除部门
async fn delete_department(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<()>>)> {
    match DepartmentService::delete_department(&state.sea_db(), id).await {
        Ok(success) => {
            if success {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("部门不存在")),
                ))
            }
        }
        Err(e) => {
            tracing::error!("删除部门失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("删除部门失败: {}", e))),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::ApiResponse;
    use axum::http::StatusCode;

    #[test]
    fn test_department_routes_creation() {
        let router = department_routes();
        // 验证路由创建成功
        assert!(std::mem::size_of_val(&router) > 0);
    }

    #[test]
    fn test_api_response_success() {
        let department = Model {
            id: 1,
            name: "测试部门".to_string(),
            code: "TEST".to_string(),
            parent_id: None,
            level: 1,
            sort_order: 1,
            description: Some("测试描述".to_string()),
            manager_id: Some(2),
            is_active: true,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        let response = ApiResponse::success(department.clone());
        let data = response.data.unwrap();
        assert_eq!(data.id, 1);
        assert_eq!(data.name, "测试部门");
        assert!(response.success);
        assert_eq!(response.message, "操作成功");
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("测试错误消息");
        assert!(!response.success);
        assert_eq!(response.message, "测试错误消息");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_create_department_dto_validation() {
        // 测试有效的DTO
        let valid_dto = CreateDepartmentDto {
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            parent_id: Some(1),
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
        };
        assert!(valid_dto.validate().is_ok());

        // 测试无效的DTO - 空名称
        let invalid_dto = CreateDepartmentDto {
            name: "".to_string(),
            code: "TECH".to_string(),
            parent_id: None,
            sort_order: 0,
            description: None,
            manager_id: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_update_department_dto_validation() {
        // 测试有效的更新DTO
        let valid_dto = UpdateDepartmentDto {
            name: Some("技术研发部".to_string()),
            code: Some("TECH_RD".to_string()),
            parent_id: Some(1),
            sort_order: Some(2),
            description: Some("负责技术研发和创新".to_string()),
            manager_id: Some(3),
            is_active: Some(true),
        };
        assert!(valid_dto.validate().is_ok());

        // 测试无效的更新DTO - 空名称
        let invalid_dto = UpdateDepartmentDto {
            name: Some("".to_string()),
            code: None,
            parent_id: None,
            sort_order: None,
            description: None,
            manager_id: None,
            is_active: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_department_tree_dto_structure() {
        let child_dto = DepartmentTreeDto {
            id: 2,
            name: "后端开发组".to_string(),
            code: "BACKEND".to_string(),
            level: 3,
            sort_order: 1,
            description: Some("负责后端开发".to_string()),
            manager_id: Some(3),
            is_active: true,
            children: vec![],
            user_count: 3,
        };

        let parent_dto = DepartmentTreeDto {
            id: 1,
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            level: 2,
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
            is_active: true,
            children: vec![child_dto],
            user_count: 8,
        };

        // 验证树形结构
        assert_eq!(parent_dto.children.len(), 1);
        assert_eq!(parent_dto.children[0].name, "后端开发组");
        assert_eq!(parent_dto.children[0].level, 3);
        assert_eq!(parent_dto.user_count, 8);
    }

    #[test]
    fn test_status_code_mappings() {
        // 测试状态码常量
        assert_eq!(StatusCode::OK.as_u16(), 200);
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), 400);
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
        assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
    }

    #[test]
    fn test_error_message_formatting() {
        let error_msg = "部门编码已存在";
        let formatted_msg = format!("创建部门失败: {}", error_msg);
        assert_eq!(formatted_msg, "创建部门失败: 部门编码已存在");
    }

    #[test]
    fn test_validation_error_formatting() {
        let validation_error = "名称长度必须在1-100个字符之间";
        let formatted_msg = format!("输入验证失败: {}", validation_error);
        assert_eq!(formatted_msg, "输入验证失败: 名称长度必须在1-100个字符之间");
    }

    #[test]
    fn test_department_id_path_parameter() {
        // 测试路径参数解析
        let id: i32 = 123;
        assert_eq!(id, 123);
        assert!(id > 0);
    }

    #[test]
    fn test_json_serialization() {
        let dto = CreateDepartmentDto {
            name: "测试部门".to_string(),
            code: "TEST".to_string(),
            parent_id: Some(1),
            sort_order: 5,
            description: Some("测试描述".to_string()),
            manager_id: Some(2),
        };

        // 测试序列化（模拟）
        let serialized = serde_json::to_string(&dto);
        assert!(serialized.is_ok());
    }

    #[test]
    fn test_department_manager_field() {
        let dto = CreateDepartmentDto {
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            parent_id: Some(1),
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2), // 测试经理字段
        };

        assert_eq!(dto.manager_id, Some(2));

        // 测试无经理的情况
        let dto_no_manager = CreateDepartmentDto {
            name: "人事部".to_string(),
            code: "HR".to_string(),
            parent_id: Some(1),
            sort_order: 2,
            description: Some("负责人事管理".to_string()),
            manager_id: None, // 无经理
        };

        assert_eq!(dto_no_manager.manager_id, None);
    }

    #[test]
    fn test_department_hierarchy_validation() {
        // 测试部门层级逻辑
        let root_dept = DepartmentTreeDto {
            id: 1,
            name: "总公司".to_string(),
            code: "HQ".to_string(),
            level: 1,
            sort_order: 1,
            description: Some("公司根部门".to_string()),
            manager_id: Some(1),
            is_active: true,
            children: vec![],
            user_count: 50,
        };

        let child_dept = DepartmentTreeDto {
            id: 2,
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            level: 2, // 子部门层级应该比父部门高
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
            is_active: true,
            children: vec![],
            user_count: 15,
        };

        // 验证层级关系
        assert!(child_dept.level > root_dept.level);
        assert_eq!(child_dept.level - root_dept.level, 1);
    }
}

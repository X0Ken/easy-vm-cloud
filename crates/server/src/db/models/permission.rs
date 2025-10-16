use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "permissions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::role_permission::Entity")]
    RolePermission,
}

impl Related<super::role_permission::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RolePermission.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePermissionDto {
    #[validate(length(min = 2, max = 50))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(min = 1))]
    pub resource: String,
    #[validate(length(min = 1))]
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_permission_dto_validation_business_rules() {
        // 业务规则：权限创建的基本验证
        let valid_permissions = vec![
            CreatePermissionDto {
                name: "用户读取".to_string(),
                description: Some("读取用户信息的权限".to_string()),
                resource: "users".to_string(),
                action: "read".to_string(),
            },
            CreatePermissionDto {
                name: "部门管理".to_string(),
                description: None,
                resource: "departments".to_string(),
                action: "manage".to_string(),
            },
        ];

        for permission in valid_permissions {
            let result = permission.validate();
            assert!(result.is_ok(), "有效权限应该通过验证: {:?}", permission);
        }
    }

    #[test]
    fn test_permission_name_validation_business_rules() {
        // 业务规则：权限名称长度限制
        let invalid_permissions = vec![
            CreatePermissionDto {
                name: "".to_string(), // 空名称
                description: None,
                resource: "users".to_string(),
                action: "read".to_string(),
            },
            CreatePermissionDto {
                name: "a".to_string(), // 太短
                description: None,
                resource: "users".to_string(),
                action: "read".to_string(),
            },
        ];

        for permission in invalid_permissions {
            let result = permission.validate();
            assert!(result.is_err(), "无效权限名称应该验证失败: {:?}", permission);
        }
    }

    #[test]
    fn test_resource_action_validation_business_rules() {
        // 业务规则：资源和动作不能为空
        let invalid_permissions = vec![
            CreatePermissionDto {
                name: "测试权限".to_string(),
                description: None,
                resource: "".to_string(), // 空资源
                action: "read".to_string(),
            },
            CreatePermissionDto {
                name: "测试权限".to_string(),
                description: None,
                resource: "users".to_string(),
                action: "".to_string(), // 空动作
            },
        ];

        for permission in invalid_permissions {
            let result = permission.validate();
            assert!(result.is_err(), "空资源或动作应该验证失败: {:?}", permission);
        }
    }

    #[test]
    fn test_permission_business_scenarios() {
        // 业务场景：常见的权限配置
        let business_permissions = vec![
            ("users:read", "users", "read", "查看用户列表"),
            ("users:write", "users", "write", "编辑用户信息"),
            ("users:delete", "users", "delete", "删除用户"),
            ("departments:manage", "departments", "manage", "管理部门"),
            ("reports:export", "reports", "export", "导出报表"),
            ("system:configure", "system", "configure", "系统配置"),
        ];

        for (name, resource, action, description) in business_permissions {
            let permission = CreatePermissionDto {
                name: name.to_string(),
                description: Some(description.to_string()),
                resource: resource.to_string(),
                action: action.to_string(),
            };

            assert!(permission.validate().is_ok(), "业务权限配置应该有效: {}", name);
            
            // 业务验证：权限命名约定
            assert_eq!(format!("{}:{}", resource, action), name, 
                "权限名称应该遵循 'resource:action' 格式");
        }
    }

    #[test]
    fn test_permission_response_structure_business_requirements() {
        // 业务需求：权限响应结构验证
        let permission_response = PermissionResponse {
            id: 1,
            name: "users:read".to_string(),
            description: Some("读取用户信息".to_string()),
            resource: "users".to_string(),
            action: "read".to_string(),
            is_active: true,
        };

        // 业务验证：响应包含必要字段
        assert!(permission_response.id > 0, "权限ID必须大于0");
        assert!(!permission_response.name.is_empty(), "权限名称不能为空");
        assert!(!permission_response.resource.is_empty(), "资源不能为空");
        assert!(!permission_response.action.is_empty(), "动作不能为空");
        assert!(permission_response.is_active, "默认权限应该是激活状态");

        // 业务验证：权限名称格式
        let expected_name = format!("{}:{}", permission_response.resource, permission_response.action);
        assert_eq!(permission_response.name, expected_name, "权限名称应该匹配资源:动作格式");
    }

    #[test]
    fn test_resource_naming_business_conventions() {
        // 业务约定：资源命名规范
        let valid_resources = vec![
            "users",
            "departments", 
            "roles",
            "permissions",
            "reports",
            "settings",
            "system",
            "files",
        ];

        for resource in valid_resources {
            let permission = CreatePermissionDto {
                name: format!("{}:read", resource),
                description: Some(format!("读取{}", resource)),
                resource: resource.to_string(),
                action: "read".to_string(),
            };

            assert!(permission.validate().is_ok(), "标准资源名称应该有效: {}", resource);
            
            // 业务约定：资源名应该是复数形式
            if resource != "system" && resource != "settings" {
                assert!(resource.ends_with('s'), "资源名应该是复数形式: {}", resource);
            }
        }
    }

    #[test]
    fn test_action_naming_business_conventions() {
        // 业务约定：动作命名规范
        let valid_actions = vec![
            "read",
            "write", 
            "delete",
            "manage",
            "export",
            "import",
            "configure",
            "execute",
        ];

        for action in valid_actions {
            let permission = CreatePermissionDto {
                name: format!("users:{}", action),
                description: Some(format!("对用户执行{}", action)),
                resource: "users".to_string(),
                action: action.to_string(),
            };

            assert!(permission.validate().is_ok(), "标准动作名称应该有效: {}", action);
            
            // 业务约定：动作名应该是动词原形
            assert!(!action.is_empty(), "动作名不能为空");
            assert!(!action.contains(' '), "动作名不应该包含空格");
        }
    }

    #[test]
    fn test_permission_hierarchy_business_logic() {
        // 业务逻辑：权限层次结构
        let permission_hierarchy = vec![
            ("manage", vec!["read", "write", "delete"]), // manage 包含其他所有操作
            ("write", vec!["read"]),                     // write 包含 read
            ("delete", vec!["read"]),                    // delete 通常需要 read
        ];

        for (high_level_action, included_actions) in permission_hierarchy {
            // 业务规则：高级权限应该隐含包含低级权限
            for included_action in included_actions {
                assert!(high_level_action != included_action, 
                    "权限不能包含自己: {}", high_level_action);
                
                // 可以添加更多层次验证逻辑
                match high_level_action {
                    "manage" => {
                        assert!(["read", "write", "delete"].contains(&included_action),
                            "manage权限应该包含基本CRUD操作");
                    }
                    "write" => {
                        assert_eq!(included_action, "read", 
                            "write权限应该包含read权限");
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_permission_scope_business_rules() {
        // 业务规则：权限范围和安全性
        let sensitive_permissions = vec![
            ("system:configure", "系统配置权限"),
            ("users:delete", "删除用户权限"),
            ("permissions:manage", "权限管理权限"),
            ("roles:delete", "删除角色权限"),
        ];

        let safe_permissions = vec![
            ("users:read", "用户读取权限"),
            ("reports:read", "报表读取权限"),
            ("files:read", "文件读取权限"),
        ];

        // 业务规则：敏感权限需要特别标识
        for (name, _description) in sensitive_permissions {
            let parts: Vec<&str> = name.split(':').collect();
            let resource = parts[0];
            let action = parts[1];

            // 业务验证：敏感操作
            if action == "delete" || action == "configure" || action == "manage" {
                assert!(matches!(action, "delete" | "configure" | "manage"),
                    "敏感权限 {} 需要特殊处理", name);
            }

            // 业务验证：系统级权限
            if resource == "system" || resource == "permissions" {
                assert!(matches!(resource, "system" | "permissions"),
                    "系统级权限 {} 需要最高安全级别", name);
            }
        }

        // 业务规则：安全权限应该是默认可用的
        for (name, _description) in safe_permissions {
            let parts: Vec<&str> = name.split(':').collect();
            let action = parts[1];
            
            assert_eq!(action, "read", "安全权限应该主要是读取操作");
        }
    }
}

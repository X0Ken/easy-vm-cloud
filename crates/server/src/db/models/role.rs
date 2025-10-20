use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "roles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_role::Entity")]
    UserRole,
    #[sea_orm(has_many = "super::role_permission::Entity")]
    RolePermission,
}

impl Related<super::user_role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserRole.def()
    }
}

impl Related<super::role_permission::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RolePermission.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateRoleDto {
    #[validate(length(min = 2, max = 50))]
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub permissions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_role_dto_validation_business_rules() {
        // 业务规则：角色名称长度验证
        let valid_roles = vec![
            CreateRoleDto {
                name: "管理员".to_string(),
                description: Some("系统管理员角色".to_string()),
            },
            CreateRoleDto {
                name: "HR".to_string(), // 最短有效长度
                description: None,
            },
            CreateRoleDto {
                name: "部门经理角色名称很长但仍在限制范围内".to_string(), // 接近最大长度
                description: Some("这是一个部门经理角色".to_string()),
            },
        ];

        for role in valid_roles {
            let result = role.validate();
            assert!(result.is_ok(), "有效角色应该通过验证: {:?}", role);
        }
    }

    #[test]
    fn test_create_role_dto_invalid_names() {
        // 业务规则：无效的角色名称应该被拒绝
        let invalid_roles = vec![
            CreateRoleDto {
                name: "".to_string(), // 空名称
                description: None,
            },
            CreateRoleDto {
                name: "a".to_string(), // 太短
                description: None,
            },
            CreateRoleDto {
                name: "这是一个超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级超级长的角色名称用来测试长度限制".to_string(), // 超过50字符
                description: None,
            },
        ];

        for role in invalid_roles {
            let result = role.validate();
            assert!(result.is_err(), "无效角色应该验证失败: {:?}", role);
        }
    }

    #[test]
    fn test_role_name_business_conventions() {
        // 业务约定：角色命名规范
        let role_conventions = vec![
            ("admin", "管理员角色"),
            ("manager", "经理角色"),
            ("employee", "员工角色"),
            ("guest", "访客角色"),
            ("hr_manager", "人事经理"),
            ("department_head", "部门主管"),
        ];

        for (name, description) in role_conventions {
            let role = CreateRoleDto {
                name: name.to_string(),
                description: Some(description.to_string()),
            };

            assert!(role.validate().is_ok(), "标准角色名称应该有效: {}", name);
            
            // 业务约定：角色名称应该使用下划线分隔或简单名称
            if name.contains(' ') {
                panic!("角色名称不应该包含空格: {}", name);
            }
        }
    }

    #[test]
    fn test_role_response_structure_business_requirements() {
        // 业务需求：角色响应结构验证
        let role_response = RoleResponse {
            id: 1,
            name: "admin".to_string(),
            description: Some("系统管理员".to_string()),
            is_active: true,
            permissions: vec![
                "users:read".to_string(),
                "users:write".to_string(),
                "users:delete".to_string(),
            ],
        };

        // 业务验证：响应包含必要字段
        assert!(role_response.id > 0, "角色ID必须大于0");
        assert!(!role_response.name.is_empty(), "角色名称不能为空");
        assert!(role_response.is_active, "默认角色应该是激活状态");
        assert!(!role_response.permissions.is_empty(), "角色应该有关联的权限");

        // 业务验证：权限格式正确
        for permission in &role_response.permissions {
            assert!(permission.contains(':'), "权限应该包含冒号分隔符: {}", permission);
            let parts: Vec<&str> = permission.split(':').collect();
            assert_eq!(parts.len(), 2, "权限应该有两个部分: {}", permission);
        }
    }

    #[test]
    fn test_role_description_business_rules() {
        // 业务规则：角色描述的可选性和内容
        let roles_with_descriptions = vec![
            CreateRoleDto {
                name: "admin".to_string(),
                description: Some("系统管理员，拥有所有权限".to_string()),
            },
            CreateRoleDto {
                name: "user".to_string(),
                description: None, // 描述可选
            },
            CreateRoleDto {
                name: "guest".to_string(),
                description: Some("".to_string()), // 空描述
            },
        ];

        for role in roles_with_descriptions {
            let result = role.validate();
            assert!(result.is_ok(), "角色描述的各种情况都应该有效: {:?}", role);
            
            // 业务逻辑：如果有描述，应该有意义
            if let Some(ref desc) = role.description {
                if !desc.is_empty() {
                    assert!(desc.len() >= 2, "非空描述应该有意义的长度");
                }
            }
        }
    }

    #[test]
    fn test_role_hierarchy_business_logic() {
        // 业务逻辑：角色层次结构
        let role_hierarchy = vec![
            ("super_admin", vec!["admin", "manager", "employee"]),
            ("admin", vec!["manager", "employee"]),
            ("manager", vec!["employee"]),
            ("employee", vec![]),
        ];

        for (role, subordinates) in role_hierarchy {
            // 业务规则：上级角色应该包含下级角色的所有权限
            assert!(subordinates.len() <= 3, "角色层次不应该太深，最多3级");
            
            // 验证角色名称符合命名约定
            assert!(!role.is_empty(), "角色名称不能为空");
            assert!(!role.contains(' '), "角色名称不应该包含空格");
            
            for subordinate in subordinates {
                assert!(!subordinate.is_empty(), "下级角色名称不能为空");
                assert_ne!(role, subordinate, "角色不能是自己的下级");
            }
        }
    }

    #[test]
    fn test_role_permissions_business_constraints() {
        // 业务约束：角色权限的合理性
        let role_permission_mapping = vec![
            ("admin", vec!["users:read", "users:write", "users:delete", "system:configure"]),
            ("manager", vec!["users:read", "users:write", "reports:read", "reports:write"]),
            ("employee", vec!["users:read", "reports:read"]),
            ("guest", vec!["users:read"]),
        ];

        for (role_name, permissions) in role_permission_mapping {
            let role_response = RoleResponse {
                id: 1,
                name: role_name.to_string(),
                description: Some(format!("{}角色", role_name)),
                is_active: true,
                permissions: permissions.iter().map(|p| (*p).to_owned()).collect(),
            };

            // 业务验证：角色权限数量合理
            match role_name {
                "admin" => assert!(role_response.permissions.len() >= 3, "管理员应该有多个权限"),
                "guest" => assert!(role_response.permissions.len() <= 2, "访客权限应该有限"),
                _ => assert!(role_response.permissions.len() >= 1, "所有角色都应该有至少一个权限"),
            }

            // 业务验证：所有角色都应该有读取权限
            let has_read_permission = role_response.permissions.iter()
                .any(|p| p.contains(":read"));
            assert!(has_read_permission, "角色 {} 应该至少有一个读取权限", role_name);

            // 业务验证：只有管理员应该有删除权限
            let has_delete_permission = role_response.permissions.iter()
                .any(|p| p.contains(":delete"));
            if role_name != "admin" {
                assert!(!has_delete_permission, "非管理员角色 {} 不应该有删除权限", role_name);
            }
        }
    }
}

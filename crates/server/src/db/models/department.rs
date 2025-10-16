use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "departments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub code: String,
    pub parent_id: Option<i32>,
    pub level: i32,
    pub sort_order: i32,
    pub description: Option<String>,
    pub manager_id: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentId",
        to = "Column::Id"
    )]
    SelfReferencing,

    #[sea_orm(has_many = "super::user_department::Entity")]
    UserDepartment,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ManagerId",
        to = "super::user::Column::Id"
    )]
    Manager,
}

impl Related<super::user_department::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserDepartment.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Manager.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateDepartmentDto {
    #[validate(length(min = 1, max = 100, message = "部门名称长度必须在1-100个字符之间"))]
    pub name: String,

    #[validate(length(min = 1, max = 50, message = "部门编码长度必须在1-50个字符之间"))]
    pub code: String,

    pub parent_id: Option<i32>,

    #[validate(range(min = 0, message = "排序值必须大于等于0"))]
    pub sort_order: i32,

    #[validate(length(max = 500, message = "描述长度不能超过500个字符"))]
    pub description: Option<String>,

    pub manager_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateDepartmentDto {
    #[validate(length(min = 1, max = 100, message = "部门名称长度必须在1-100个字符之间"))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 50, message = "部门编码长度必须在1-50个字符之间"))]
    pub code: Option<String>,

    pub parent_id: Option<i32>,

    #[validate(range(min = 0, message = "排序值必须大于等于0"))]
    pub sort_order: Option<i32>,

    #[validate(length(max = 500, message = "描述长度不能超过500个字符"))]
    pub description: Option<String>,

    pub manager_id: Option<i32>,

    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepartmentTreeDto {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub level: i32,
    pub sort_order: i32,
    pub description: Option<String>,
    pub manager_id: Option<i32>,
    pub is_active: bool,
    pub children: Vec<DepartmentTreeDto>,
    pub user_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_department_dto_validation() {
        // 测试有效的部门创建DTO
        let valid_dto = CreateDepartmentDto {
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            parent_id: Some(1),
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
        };
        assert!(valid_dto.validate().is_ok());

        // 测试空名称
        let invalid_name_dto = CreateDepartmentDto {
            name: "".to_string(),
            code: "TECH".to_string(),
            parent_id: None,
            sort_order: 0,
            description: None,
            manager_id: None,
        };
        assert!(invalid_name_dto.validate().is_err());

        // 测试名称过长
        let long_name_dto = CreateDepartmentDto {
            name: "a".repeat(101),
            code: "TECH".to_string(),
            parent_id: None,
            sort_order: 0,
            description: None,
            manager_id: None,
        };
        assert!(long_name_dto.validate().is_err());

        // 测试空编码
        let invalid_code_dto = CreateDepartmentDto {
            name: "技术部".to_string(),
            code: "".to_string(),
            parent_id: None,
            sort_order: 0,
            description: None,
            manager_id: None,
        };
        assert!(invalid_code_dto.validate().is_err());

        // 测试编码过长
        let long_code_dto = CreateDepartmentDto {
            name: "技术部".to_string(),
            code: "a".repeat(51),
            parent_id: None,
            sort_order: 0,
            description: None,
            manager_id: None,
        };
        assert!(long_code_dto.validate().is_err());

        // 测试负排序值
        let negative_sort_dto = CreateDepartmentDto {
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            parent_id: None,
            sort_order: -1,
            description: None,
            manager_id: None,
        };
        assert!(negative_sort_dto.validate().is_err());

        // 测试描述过长
        let long_desc_dto = CreateDepartmentDto {
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            parent_id: None,
            sort_order: 0,
            description: Some("a".repeat(501)),
            manager_id: None,
        };
        assert!(long_desc_dto.validate().is_err());
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

        // 测试空名称
        let invalid_name_dto = UpdateDepartmentDto {
            name: Some("".to_string()),
            code: None,
            parent_id: None,
            sort_order: None,
            description: None,
            manager_id: None,
            is_active: None,
        };
        assert!(invalid_name_dto.validate().is_err());

        // 测试所有字段为None
        let all_none_dto = UpdateDepartmentDto {
            name: None,
            code: None,
            parent_id: None,
            sort_order: None,
            description: None,
            manager_id: None,
            is_active: None,
        };
        assert!(all_none_dto.validate().is_ok());
    }

    #[test]
    fn test_department_tree_dto_creation() {
        let tree_dto = DepartmentTreeDto {
            id: 1,
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            level: 2,
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
            is_active: true,
            children: vec![],
            user_count: 5,
        };

        assert_eq!(tree_dto.id, 1);
        assert_eq!(tree_dto.name, "技术部");
        assert_eq!(tree_dto.code, "TECH");
        assert_eq!(tree_dto.level, 2);
        assert_eq!(tree_dto.sort_order, 1);
        assert_eq!(tree_dto.manager_id, Some(2));
        assert!(tree_dto.is_active);
        assert_eq!(tree_dto.children.len(), 0);
        assert_eq!(tree_dto.user_count, 5);
    }

    #[test]
    fn test_department_tree_dto_with_children() {
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

        assert_eq!(parent_dto.children.len(), 1);
        assert_eq!(parent_dto.children[0].name, "后端开发组");
        assert_eq!(parent_dto.children[0].level, 3);
    }

    #[test]
    fn test_department_model_fields() {
        // 测试部门模型的字段
        let model = Model {
            id: 1,
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            parent_id: Some(1),
            level: 2,
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
            is_active: true,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(model.id, 1);
        assert_eq!(model.name, "技术部");
        assert_eq!(model.code, "TECH");
        assert_eq!(model.parent_id, Some(1));
        assert_eq!(model.level, 2);
        assert_eq!(model.sort_order, 1);
        assert_eq!(model.description, Some("负责技术开发".to_string()));
        assert_eq!(model.manager_id, Some(2));
        assert!(model.is_active);
    }
}

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_departments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub department_id: i32,
    pub position: Option<String>,
    pub is_primary: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,

    #[sea_orm(
        belongs_to = "super::department::Entity",
        from = "Column::DepartmentId",
        to = "super::department::Column::Id"
    )]
    Department,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::department::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Department.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateUserDepartmentDto {
    pub user_id: i32,
    pub department_id: i32,

    #[validate(length(max = 100, message = "职位名称长度不能超过100个字符"))]
    pub position: Option<String>,

    pub is_primary: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateUserDepartmentDto {
    #[validate(length(max = 100, message = "职位名称长度不能超过100个字符"))]
    pub position: Option<String>,

    pub is_primary: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDepartmentDto {
    pub id: i32,
    pub user_id: i32,
    pub department_id: i32,
    pub position: Option<String>,
    pub is_primary: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub user: Option<super::user::Model>,
    pub department: Option<super::department::Model>,
}

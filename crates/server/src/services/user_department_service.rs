use anyhow::Result;
use sea_orm::{*, prelude::Expr};
use crate::db::models::user_department::{Entity, Model, ActiveModel, Column, CreateUserDepartmentDto, UpdateUserDepartmentDto, UserDepartmentDto};
use crate::db::models::user::Entity as UserEntity;
use crate::db::models::department::Entity as DepartmentEntity;
use crate::services::{UserService, DepartmentService};

pub struct UserDepartmentService;

impl UserDepartmentService {
    /// 为用户分配部门
    pub async fn assign_user_to_department(db: &DatabaseConnection, dto: CreateUserDepartmentDto) -> Result<Model> {
        let db = db;

        // 检查用户是否存在
        let _user = UserService::get_user(db, dto.user_id).await?
            .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;

        // 检查部门是否存在
        let _department = DepartmentService::get_department(db, dto.department_id).await?
            .ok_or_else(|| anyhow::anyhow!("部门不存在"))?;

        // 检查是否已经分配
        let existing = crate::db::models::user_department::Entity::find()
            .filter(Column::UserId.eq(dto.user_id))
            .filter(Column::DepartmentId.eq(dto.department_id))
            .one(db)
            .await?;

        if existing.is_some() {
            return Err(anyhow::anyhow!("用户已在该部门中"));
        }

        // 如果设置为主要部门，需要将其他主要部门设为非主要
        if dto.is_primary {
            let _ = crate::db::models::user_department::Entity::update_many()
                .col_expr(Column::IsPrimary, Expr::value(false))
                .filter(Column::UserId.eq(dto.user_id))
                .exec(db)
                .await?;
        }

        let user_department = crate::db::models::user_department::ActiveModel {
            user_id: Set(dto.user_id),
            department_id: Set(dto.department_id),
            position: Set(dto.position),
            is_primary: Set(dto.is_primary),
            ..Default::default()
        };

        let result = user_department.insert(db).await?;
        Ok(result)
    }

    /// 更新用户部门信息
    pub async fn update_user_department(db: &DatabaseConnection, id: i32, dto: UpdateUserDepartmentDto) -> Result<Model> {
        let db = db;

        let user_department = crate::db::models::user_department::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("用户部门关联不存在"))?;

        let mut user_department: crate::db::models::user_department::ActiveModel = user_department.into();

        if let Some(position) = dto.position {
            user_department.position = Set(Some(position));
        }

        if let Some(is_primary) = dto.is_primary {
            // 如果设置为主要部门，需要将其他主要部门设为非主要
            if is_primary {
                let _ = crate::db::models::user_department::Entity::update_many()
                    .col_expr(Column::IsPrimary, Expr::value(false))
                    .filter(Column::UserId.eq(*user_department.user_id.as_ref()))
                    .filter(Column::Id.ne(id))
                    .exec(db)
                    .await?;
            }
            user_department.is_primary = Set(is_primary);
        }

        let result = user_department.update(db).await?;
        Ok(result)
    }

    /// 移除用户部门关联
    pub async fn remove_user_from_department(db: &DatabaseConnection, id: i32) -> Result<bool> {
        let db = db;

        let result = crate::db::models::user_department::Entity::delete_by_id(id)
            .exec(db)
            .await?;

        Ok(result.rows_affected > 0)
    }

    /// 获取用户部门关联详情
    pub async fn get_user_department(db: &DatabaseConnection, id: i32) -> Result<Option<UserDepartmentDto>> {
        let db = db;

        let user_department = Entity::find_by_id(id)
            .one(db)
            .await?;

        if let Some(model) = user_department {
            // 单独获取用户和部门信息
            let user = UserEntity::find_by_id(model.user_id).one(db).await?;
            let department = DepartmentEntity::find_by_id(model.department_id).one(db).await?;

            let dto = UserDepartmentDto {
                id: model.id,
                user_id: model.user_id,
                department_id: model.department_id,
                position: model.position,
                is_primary: model.is_primary,
                created_at: model.created_at,
                updated_at: model.updated_at,
                user,
                department,
            };
            Ok(Some(dto))
        } else {
            Ok(None)
        }
    }

    /// 获取用户的所有部门
    pub async fn get_user_departments(db: &DatabaseConnection, user_id: i32) -> Result<Vec<UserDepartmentDto>> {
        let db = db;

        let user_departments = crate::db::models::user_department::Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await?;

        let mut result = Vec::new();
        for model in user_departments {
            // 单独获取用户和部门信息
            let user = UserEntity::find_by_id(model.user_id).one(db).await?;
            let department = DepartmentEntity::find_by_id(model.department_id).one(db).await?;

            let dto = UserDepartmentDto {
                id: model.id,
                user_id: model.user_id,
                department_id: model.department_id,
                position: model.position,
                is_primary: model.is_primary,
                created_at: model.created_at,
                updated_at: model.updated_at,
                user,
                department,
            };
            result.push(dto);
        }

        Ok(result)
    }

    /// 获取部门的所有用户
    pub async fn get_department_users(db: &DatabaseConnection, department_id: i32) -> Result<Vec<UserDepartmentDto>> {
        let db = db;

        let user_departments = crate::db::models::user_department::Entity::find()
            .filter(Column::DepartmentId.eq(department_id))
            .all(db)
            .await?;

        let mut result = Vec::new();
        for model in user_departments {
            // 单独获取用户和部门信息
            let user = UserEntity::find_by_id(model.user_id).one(db).await?;
            let department = DepartmentEntity::find_by_id(model.department_id).one(db).await?;

            let dto = UserDepartmentDto {
                id: model.id,
                user_id: model.user_id,
                department_id: model.department_id,
                position: model.position,
                is_primary: model.is_primary,
                created_at: model.created_at,
                updated_at: model.updated_at,
                user,
                department,
            };
            result.push(dto);
        }

        Ok(result)
    }

    /// 统计部门用户数量
    pub async fn count_users_by_department(db: &DatabaseConnection, department_id: i32) -> Result<i64> {
        let db = db;

                let count = crate::db::models::user_department::Entity::find()
            .filter(Column::DepartmentId.eq(department_id))
            .count(db)
            .await?;

        Ok(count as i64)
    }

    /// 获取用户的主要部门
    pub async fn get_user_primary_department(db: &DatabaseConnection, user_id: i32) -> Result<Option<UserDepartmentDto>> {
        let db = db;

        let user_department = crate::db::models::user_department::Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::IsPrimary.eq(true))
            .one(db)
            .await?;

        if let Some(model) = user_department {
            // 单独获取用户和部门信息
            let user = UserEntity::find_by_id(model.user_id).one(db).await?;
            let department = DepartmentEntity::find_by_id(model.department_id).one(db).await?;

            let dto = UserDepartmentDto {
                id: model.id,
                user_id: model.user_id,
                department_id: model.department_id,
                position: model.position,
                is_primary: model.is_primary,
                created_at: model.created_at,
                updated_at: model.updated_at,
                user,
                department,
            };
            Ok(Some(dto))
        } else {
            Ok(None)
        }
    }

    /// 批量分配用户到部门
    pub async fn batch_assign_users_to_department(db: &DatabaseConnection, 
        user_ids: Vec<i32>,
        department_id: i32,
        position: Option<String>
    ) -> Result<Vec<Model>> {
        let db = db;

        // 检查部门是否存在
        let _department = DepartmentService::get_department(db, department_id).await?
            .ok_or_else(|| anyhow::anyhow!("部门不存在"))?;

        let mut results = Vec::new();

        for user_id in user_ids {
            // 检查用户是否存在
            let _user = UserService::get_user(db, user_id).await?
                .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;

            // 检查是否已经分配
            let existing = crate::db::models::user_department::Entity::find()
                .filter(Column::UserId.eq(user_id))
                .filter(Column::DepartmentId.eq(department_id))
                .one(db)
                .await?;

            if existing.is_some() {
                continue; // 跳过已存在的关联
            }

            let user_department = ActiveModel {
                user_id: Set(user_id),
                department_id: Set(department_id),
                position: Set(position.clone()),
                is_primary: Set(false), // 批量分配时默认为非主要部门
                ..Default::default()
            };

            let result = user_department.insert(db).await?;
            results.push(result);
        }

        Ok(results)
    }
}

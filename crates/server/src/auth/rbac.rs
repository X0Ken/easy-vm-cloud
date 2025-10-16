use crate::auth::Claims;
use crate::db::models::{permission, role, user, user_role, role_permission};
use sea_orm::*;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RbacError {
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] DbErr),
    #[error("用户不存在")]
    UserNotFound,
    #[error("权限不足")]
    InsufficientPermissions,
    #[error("角色不存在")]
    RoleNotFound,
}

pub struct RbacService;

impl RbacService {
    /// 检查用户是否有指定权限
    pub async fn check_permission(
        db: &DatabaseConnection,
        user_id: i32,
        resource: &str,
        action: &str,
    ) -> Result<bool, RbacError> {
        let user_permissions = Self::get_user_permissions(db, user_id).await?;

        let required_permission = format!("{}:{}", resource, action);
        Ok(user_permissions.contains(&required_permission))
    }

    /// 获取用户所有权限
    pub async fn get_user_permissions(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<HashSet<String>, RbacError> {
        let user_roles = user_role::Entity::find()
            .filter(user_role::Column::UserId.eq(user_id))
            .find_also_related(role::Entity)
            .all(db)
            .await?;

        let mut permissions = HashSet::new();

        for (_user_role, role) in user_roles {
            if let Some(role) = role {
                if role.is_active {
                    let role_permissions = role_permission::Entity::find()
                        .filter(role_permission::Column::RoleId.eq(role.id))
                        .find_also_related(permission::Entity)
                        .all(db)
                        .await?;

                    for (_, permission) in role_permissions {
                        if let Some(permission) = permission {
                            if permission.is_active {
                                let permission_string = format!("{}:{}", permission.resource, permission.action);
                                permissions.insert(permission_string);
                            }
                        }
                    }
                }
            }
        }

        Ok(permissions)
    }

    /// 获取用户所有角色
    pub async fn get_user_roles(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Vec<String>, RbacError> {
        let user_roles = user_role::Entity::find()
            .filter(user_role::Column::UserId.eq(user_id))
            .find_also_related(role::Entity)
            .all(db)
            .await?;

        let roles: Vec<String> = user_roles
            .into_iter()
            .filter_map(|(_, role)| {
                role.filter(|r| r.is_active).map(|r| r.name)
            })
            .collect();

        Ok(roles)
    }

    /// 为用户分配角色
    pub async fn assign_role_to_user(
        db: &DatabaseConnection,
        user_id: i32,
        role_id: i32,
    ) -> Result<(), RbacError> {
        // 检查用户和角色是否存在
        let user = user::Entity::find_by_id(user_id).one(db).await?;
        if user.is_none() {
            return Err(RbacError::UserNotFound);
        }

        let role = role::Entity::find_by_id(role_id).one(db).await?;
        if role.is_none() {
            return Err(RbacError::RoleNotFound);
        }

        // 检查是否已经分配了该角色
        let existing = user_role::Entity::find()
            .filter(user_role::Column::UserId.eq(user_id))
            .filter(user_role::Column::RoleId.eq(role_id))
            .one(db)
            .await?;

        if existing.is_some() {
            return Ok(()); // 已经分配了该角色
        }

        // 创建新的用户角色关联
        let user_role = user_role::ActiveModel {
            user_id: Set(user_id),
            role_id: Set(role_id),
            ..Default::default()
        };

        user_role::Entity::insert(user_role).exec(db).await?;
        Ok(())
    }

    /// 获取角色的所有权限
    pub async fn get_role_permissions(
        db: &DatabaseConnection,
        role_id: i32,
    ) -> Result<HashSet<String>, RbacError> {
        let role_permissions = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .find_also_related(permission::Entity)
            .all(db)
            .await?;

        let mut permissions = HashSet::new();

        for (_, permission) in role_permissions {
            if let Some(permission) = permission {
                if permission.is_active {
                    let permission_string = format!("{}:{}", permission.resource, permission.action);
                    permissions.insert(permission_string);
                }
            }
        }

        Ok(permissions)
    }

    /// 为角色分配权限
    pub async fn assign_permission_to_role(
        db: &DatabaseConnection,
        role_id: i32,
        permission_id: i32,
    ) -> Result<(), RbacError> {
        // 检查角色和权限是否存在
        let role = role::Entity::find_by_id(role_id).one(db).await?;
        if role.is_none() {
            return Err(RbacError::RoleNotFound);
        }

        let permission = permission::Entity::find_by_id(permission_id).one(db).await?;
        if permission.is_none() {
            return Err(RbacError::InsufficientPermissions);
        }

        // 检查是否已经分配了该权限
        let existing = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::PermissionId.eq(permission_id))
            .one(db)
            .await?;

        if existing.is_some() {
            return Ok(()); // 已经分配了该权限
        }

        // 创建新的角色权限关联
        let role_permission = role_permission::ActiveModel {
            role_id: Set(role_id),
            permission_id: Set(permission_id),
            ..Default::default()
        };

        role_permission::Entity::insert(role_permission).exec(db).await?;
        Ok(())
    }

    /// 为角色移除权限
    pub async fn remove_permission_from_role(
        db: &DatabaseConnection,
        role_id: i32,
        permission_id: i32,
    ) -> Result<(), RbacError> {
        // 检查角色和权限是否存在
        let role = role::Entity::find_by_id(role_id).one(db).await?;
        if role.is_none() {
            return Err(RbacError::RoleNotFound);
        }

        let permission = permission::Entity::find_by_id(permission_id).one(db).await?;
        if permission.is_none() {
            return Err(RbacError::InsufficientPermissions);
        }

        // 查找并删除角色权限关联
        let role_permission = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::PermissionId.eq(permission_id))
            .one(db)
            .await?;

        if let Some(role_permission) = role_permission {
            role_permission::Entity::delete_by_id(role_permission.id).exec(db).await?;
        }

        Ok(())
    }
}

/// 权限检查中间件
pub async fn require_permission(
    claims: Claims,
    db: DatabaseConnection,
    resource: &str,
    action: &str,
) -> Result<Claims, RbacError> {
    let has_permission = RbacService::check_permission(&db, claims.sub, resource, action).await?;

    if !has_permission {
        return Err(RbacError::InsufficientPermissions);
    }

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_error_business_context() {
        // 业务上下文中的错误处理测试
        let error_scenarios = vec![
            (RbacError::UserNotFound, "用户不存在"),
            (RbacError::RoleNotFound, "角色不存在"), 
            (RbacError::InsufficientPermissions, "权限不足"),
        ];
        
        for (error, expected_context) in error_scenarios {
            let error_message = format!("{}", error);
            assert!(error_message.contains(expected_context),
                "错误消息应该包含业务上下文: '{}'", expected_context);
        }
    }
}

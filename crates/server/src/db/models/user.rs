use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_role::Entity")]
    UserRole,
}

impl Related<super::user_role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserRole.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// 自定义邮箱验证函数
fn validate_email_format(email: &str) -> Result<(), ValidationError> {
    // 检查是否包含连续的点号
    if email.contains("..") {
        return Err(ValidationError::new("invalid_email_format"));
    }
    
    // 使用标准邮箱验证
    if !validator::validate_email(email) {
        return Err(ValidationError::new("invalid_email"));
    }
    
    // 额外检查：确保域名部分包含至少一个点（顶级域名）
    if let Some(domain_part) = email.split('@').nth(1) {
        if !domain_part.contains('.') {
            return Err(ValidationError::new("invalid_domain"));
        }
    }
    
    Ok(())
}

// DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateUserDto {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(custom = "validate_email_format")]
    pub email: String,
    #[validate(length(min = 6))]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct LoginDto {
    #[validate(length(min = 1))]
    pub username: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_user_dto_validation() {
        // 测试有效的用户创建DTO
        let valid_dto = CreateUserDto {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(valid_dto.validate().is_ok());

        // 测试用户名过短
        let invalid_username_dto = CreateUserDto {
            username: "ab".to_string(), // 少于3个字符
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(invalid_username_dto.validate().is_err());

        // 测试用户名过长
        let long_username_dto = CreateUserDto {
            username: "a".repeat(51), // 超过50个字符
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(long_username_dto.validate().is_err());

        // 测试无效邮箱格式
        let invalid_email_dto = CreateUserDto {
            username: "testuser".to_string(),
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
        };
        assert!(invalid_email_dto.validate().is_err());

        // 测试密码过短
        let short_password_dto = CreateUserDto {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "12345".to_string(), // 少于6个字符
        };
        assert!(short_password_dto.validate().is_err());

        // 测试空邮箱
        let empty_email_dto = CreateUserDto {
            username: "testuser".to_string(),
            email: "".to_string(),
            password: "password123".to_string(),
        };
        assert!(empty_email_dto.validate().is_err());
    }

    #[test]
    fn test_login_dto_validation() {
        // 测试有效的登录DTO
        let valid_dto = LoginDto {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };
        assert!(valid_dto.validate().is_ok());

        // 测试空用户名
        let empty_username_dto = LoginDto {
            username: "".to_string(),
            password: "password123".to_string(),
        };
        assert!(empty_username_dto.validate().is_err());

        // 测试空密码
        let empty_password_dto = LoginDto {
            username: "testuser".to_string(),
            password: "".to_string(),
        };
        assert!(empty_password_dto.validate().is_err());

        // 测试都为空
        let all_empty_dto = LoginDto {
            username: "".to_string(),
            password: "".to_string(),
        };
        assert!(all_empty_dto.validate().is_err());
    }

    #[test]
    fn test_user_model_fields() {
        let user = Model {
            id: 1,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            is_active: true,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(user.id, 1);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "hashed_password");
        assert!(user.is_active);
    }

    #[test]
    fn test_user_response_structure() {
        let user_response = UserResponse {
            id: 1,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            is_active: true,
            roles: vec!["admin".to_string(), "user".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
        };

        assert_eq!(user_response.id, 1);
        assert_eq!(user_response.username, "testuser");
        assert_eq!(user_response.email, "test@example.com");
        assert!(user_response.is_active);
        assert_eq!(user_response.roles.len(), 2);
        assert_eq!(user_response.permissions.len(), 2);
        assert!(user_response.roles.contains(&"admin".to_string()));
        assert!(user_response.permissions.contains(&"read".to_string()));
    }

    #[test]
    fn test_email_validation_edge_cases() {
        // 测试各种邮箱格式
        let test_cases = vec![
            ("user@domain.com", true),
            ("user.name@domain.com", true),
            ("user+tag@domain.com", true),
            ("user@sub.domain.com", true),
            ("user@domain-name.com", true),
            ("plainaddress", false),
            ("@domain.com", false),
            ("user@", false),
            ("user..name@domain.com", false),
            ("user@domain", false),
        ];

        for (email, should_be_valid) in test_cases {
            let dto = CreateUserDto {
                username: "testuser".to_string(),
                email: email.to_string(),
                password: "password123".to_string(),
            };
            
            if should_be_valid {
                assert!(dto.validate().is_ok(), "Email '{}' should be valid", email);
            } else {
                assert!(dto.validate().is_err(), "Email '{}' should be invalid", email);
            }
        }
    }

    #[test]
    fn test_username_validation_edge_cases() {
        let long_username = "a".repeat(50);
        let too_long_username = "a".repeat(51);
        let test_cases = vec![
            ("abc", true),           // 最小长度
            (long_username.as_str(), true),  // 最大长度
            ("ab", false),           // 太短
            (too_long_username.as_str(), false), // 太长
            ("", false),             // 空字符串
        ];

        for (username, should_be_valid) in test_cases {
            let dto = CreateUserDto {
                username: username.to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            };
            
            if should_be_valid {
                assert!(dto.validate().is_ok(), "Username '{}' should be valid", username);
            } else {
                assert!(dto.validate().is_err(), "Username '{}' should be invalid", username);
            }
        }
    }

    #[test]
    fn test_password_validation_edge_cases() {
        let test_cases = vec![
            ("123456", true),        // 最小长度
            ("password123", true),   // 正常密码
            ("12345", false),        // 太短
            ("", false),             // 空字符串
        ];

        for (password, should_be_valid) in test_cases {
            let dto = CreateUserDto {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password: password.to_string(),
            };
            
            if should_be_valid {
                assert!(dto.validate().is_ok(), "Password '{}' should be valid", password);
            } else {
                assert!(dto.validate().is_err(), "Password '{}' should be invalid", password);
            }
        }
    }
}

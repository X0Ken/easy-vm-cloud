use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("密码加密失败: {0}")]
    PasswordHashError(#[from] bcrypt::BcryptError),
    #[error("密码验证失败")]
    PasswordVerifyError,
    #[error("JWT生成失败: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
    #[error("无效的JWT令牌")]
    InvalidToken,
    #[error("令牌已过期")]
    TokenExpired,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i32, // 用户ID
    pub username: String,
    pub exp: u64, // 过期时间
    pub iat: u64, // 签发时间
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

pub struct AuthService;

impl AuthService {
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        let hashed = hash(password, DEFAULT_COST)?;
        Ok(hashed)
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        let is_valid = verify(password, hash)?;
        if !is_valid {
            return Err(AuthError::PasswordVerifyError);
        }
        Ok(true)
    }

    pub fn generate_token(user_id: i32, username: &str) -> Result<String, AuthError> {
        let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
        let expiration = env::var("JWT_EXPIRATION")
            .unwrap_or_else(|_| "24h".to_string())
            .parse::<u64>()
            .unwrap_or(86400); // 默认24小时

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            exp: now + expiration,
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )?;

        Ok(token)
    }

    pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
        let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
        
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub fn extract_token_from_header(auth_header: &str) -> Result<String, AuthError> {
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken);
        }
        
        let token = auth_header[7..].trim();
        if token.is_empty() {
            return Err(AuthError::InvalidToken);
        }
        
        Ok(token.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_password_hashing_business_logic() {
        // 测试密码哈希业务逻辑：相同密码应该产生不同的哈希值（因为盐值不同）
        let password = "business_password123";
        let hash1 = AuthService::hash_password(password).unwrap();
        let hash2 = AuthService::hash_password(password).unwrap();
        
        // 业务要求：相同密码的哈希值应该不同（安全性）
        assert_ne!(hash1, hash2);
        
        // 业务要求：哈希值不能是原密码
        assert_ne!(hash1, password);
        assert_ne!(hash2, password);
    }

    #[test]
    fn test_password_verification_business_rules() {
        let password = "correct_password";
        let hash = AuthService::hash_password(password).unwrap();
        
        // 业务规则：正确密码必须验证成功
        assert!(AuthService::verify_password(password, &hash).is_ok());
        
        // 业务规则：错误密码必须验证失败
        let wrong_passwords = vec![
            "wrong_password",
            "CORRECT_PASSWORD", // 大小写敏感
            "correct_password ", // 多余空格
            "correct_passwor", // 缺少字符
            "",
        ];
        
        for wrong_pwd in wrong_passwords {
            let result = AuthService::verify_password(wrong_pwd, &hash);
            assert!(result.is_err(), "密码 '{}' 应该验证失败", wrong_pwd);
        }
    }

    #[test]
    fn test_jwt_token_business_requirements() {
        let test_secret = "test_secret_for_business";
        env::set_var("JWT_SECRET", test_secret);
        env::set_var("JWT_EXPIRATION", "3600");
        
        let user_id = 12345;
        let username = "business_user";
        
        // 业务需求：生成的令牌必须包含用户信息
        let token = AuthService::generate_token(user_id, username).unwrap();
        
        // 确保使用相同的secret验证
        env::set_var("JWT_SECRET", test_secret);
        let claims = AuthService::verify_token(&token).unwrap();
        
        // 验证业务数据完整性
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
        
        // 业务需求：令牌必须有有效的时间戳
        assert!(claims.exp > claims.iat);
        
        // 业务需求：过期时间应该是1小时后
        let expected_duration = 3600; // 1小时
        let actual_duration = claims.exp - claims.iat;
        assert_eq!(actual_duration, expected_duration);
    }

    #[test]
    fn test_token_security_business_rules() {
        // 业务安全规则：不同的密钥应该产生不可互换的令牌
        let user_id = 1;
        let username = "test_user";
        
        env::set_var("JWT_SECRET", "secret1");
        let token1 = AuthService::generate_token(user_id, username).unwrap();
        
        env::set_var("JWT_SECRET", "secret2");
        let token2 = AuthService::generate_token(user_id, username).unwrap();
        
        // 业务规则：不同密钥生成的令牌必须不同
        assert_ne!(token1, token2);
        
        // 业务规则：用错误密钥验证令牌必须失败
        let result = AuthService::verify_token(&token1);
        assert!(result.is_err());
    }

    #[test]
    fn test_authorization_header_parsing_business_logic() {
        // 业务逻辑：正确格式的Authorization头必须能解析出令牌
        let valid_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.business.token";
        let auth_header = format!("Bearer {}", valid_token);
        
        let result = AuthService::extract_token_from_header(&auth_header).unwrap();
        assert_eq!(result, valid_token);
        
        // 业务逻辑：处理空格
        let header_with_spaces = format!("Bearer   {}   ", valid_token);
        let result = AuthService::extract_token_from_header(&header_with_spaces).unwrap();
        assert_eq!(result, valid_token);
    }

    #[test]
    fn test_invalid_authorization_header_business_rules() {
        // 业务规则：无效的Authorization头格式必须被拒绝
        let invalid_cases = vec![
            "InvalidFormat token123",      // 错误的前缀
            "Bearer",                      // 没有令牌
            "Bearer ",                     // 空令牌
            "",                           // 空字符串
            "Basic dXNlcjpwYXNz",         // 错误的认证类型
            "bearer token123",            // 小写bearer
        ];
        
        for invalid_header in invalid_cases {
            let result = AuthService::extract_token_from_header(invalid_header);
            assert!(result.is_err(), "无效头 '{}' 应该被拒绝", invalid_header);
        }
    }

    #[test]
    fn test_token_expiration_business_logic() {
        let test_secret = "expiration_test_secret";
        
        // 测试不同的过期时间业务逻辑
        let test_cases = vec![
            ("1", 1),      // 1秒
            ("60", 60),    // 1分钟  
            ("3600", 3600), // 1小时
            ("86400", 86400), // 1天
        ];
        
        for (exp_str, expected_seconds) in test_cases {
            // 在每次生成和验证token前都设置相同的secret
            env::set_var("JWT_SECRET", test_secret);
            env::set_var("JWT_EXPIRATION", exp_str);
            
            let token = AuthService::generate_token(1, "test").unwrap();
            
            // 确保验证时使用相同的secret
            env::set_var("JWT_SECRET", test_secret);
            let claims = AuthService::verify_token(&token).unwrap();
            
            let duration = claims.exp - claims.iat;
            assert_eq!(duration, expected_seconds, 
                "过期时间设置 {} 应该产生 {} 秒的持续时间", exp_str, expected_seconds);
        }
    }

    #[test]
    fn test_user_identification_business_logic() {
        env::set_var("JWT_SECRET", "user_test_secret");
        
        // 业务逻辑：不同用户应该生成不同的令牌
        let users = vec![
            (1, "admin"),
            (2, "user"),  
            (3, "guest"),
        ];
        
        let mut tokens = Vec::new();
        
        for (user_id, username) in &users {
            let token = AuthService::generate_token(*user_id, username).unwrap();
            let claims = AuthService::verify_token(&token).unwrap();
            
            // 验证用户身份信息正确
            assert_eq!(claims.sub, *user_id);
            assert_eq!(claims.username, *username);
            
            tokens.push(token);
        }
        
        // 业务规则：不同用户的令牌必须不同
        for i in 0..tokens.len() {
            for j in i+1..tokens.len() {
                assert_ne!(tokens[i], tokens[j], 
                    "用户{}和用户{}的令牌不应该相同", i+1, j+1);
            }
        }
    }
}

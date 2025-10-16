use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: "操作成功".to_string(),
            data: Some(data),
        }
    }

    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_api_response_success_business_logic() {
        // 业务逻辑：成功响应必须包含数据和正确的状态
        let test_data = vec!["item1", "item2", "item3"];
        let response = ApiResponse::success(test_data.clone());
        
        // 业务要求：成功响应的基本结构
        assert!(response.success, "成功响应的success字段必须为true");
        assert_eq!(response.message, "操作成功", "默认成功消息不正确");
        assert!(response.data.is_some(), "成功响应必须包含数据");
        assert_eq!(response.data.unwrap(), test_data, "响应数据必须与输入数据一致");
    }

    #[test]
    fn test_api_response_success_with_custom_message() {
        // 业务逻辑：支持自定义成功消息
        let data = "test_result";
        let custom_message = "用户创建成功";
        let response = ApiResponse::success_with_message(data, custom_message);
        
        assert!(response.success);
        assert_eq!(response.message, custom_message);
        assert_eq!(response.data.unwrap(), data);
    }

    #[test]
    fn test_api_response_error_business_logic() {
        // 业务逻辑：错误响应必须包含错误信息且不包含数据
        let error_messages = vec![
            "用户不存在",
            "权限不足", 
            "参数验证失败",
            "服务器内部错误",
            "数据库连接失败",
        ];
        
        for error_msg in error_messages {
            let response: ApiResponse<String> = ApiResponse::error(error_msg);
            
            // 业务要求：错误响应的基本结构
            assert!(!response.success, "错误响应的success字段必须为false");
            assert_eq!(response.message, error_msg, "错误消息必须与输入一致");
            assert!(response.data.is_none(), "错误响应不应该包含数据");
        }
    }

    #[test]
    fn test_api_response_serialization_business_requirements() {
        // 业务需求：API响应必须能够正确序列化为JSON
        let success_response = ApiResponse::success(vec![1, 2, 3]);
        let success_json = serde_json::to_string(&success_response).unwrap();
        
        // 验证JSON包含必要字段
        assert!(success_json.contains("\"success\":true"));
        assert!(success_json.contains("\"message\":\"操作成功\""));
        assert!(success_json.contains("\"data\":[1,2,3]"));
        
        let error_response: ApiResponse<()> = ApiResponse::error("测试错误");
        let error_json = serde_json::to_string(&error_response).unwrap();
        
        assert!(error_json.contains("\"success\":false"));
        assert!(error_json.contains("\"message\":\"测试错误\""));
        assert!(error_json.contains("\"data\":null"));
    }

    #[test]
    fn test_api_response_deserialization_business_requirements() {
        // 业务需求：能够从JSON反序列化API响应
        let success_json = r#"{"success":true,"message":"操作成功","data":"test_data"}"#;
        let success_response: ApiResponse<String> = serde_json::from_str(success_json).unwrap();
        
        assert!(success_response.success);
        assert_eq!(success_response.message, "操作成功");
        assert_eq!(success_response.data.unwrap(), "test_data");
        
        let error_json = r#"{"success":false,"message":"错误信息","data":null}"#;
        let error_response: ApiResponse<String> = serde_json::from_str(error_json).unwrap();
        
        assert!(!error_response.success);
        assert_eq!(error_response.message, "错误信息");
        assert!(error_response.data.is_none());
    }

    #[test]
    fn test_api_response_different_data_types() {
        // 业务场景：测试不同数据类型的响应
        
        // 字符串数据
        let string_response = ApiResponse::success("字符串数据".to_string());
        assert_eq!(string_response.data.unwrap(), "字符串数据");
        
        // 数字数据
        let number_response = ApiResponse::success(42);
        assert_eq!(number_response.data.unwrap(), 42);
        
        // 布尔数据
        let bool_response = ApiResponse::success(true);
        assert_eq!(bool_response.data.unwrap(), true);
        
        // 数组数据
        let array_response = ApiResponse::success(vec!["a", "b", "c"]);
        assert_eq!(array_response.data.unwrap(), vec!["a", "b", "c"]);
        
        // 结构体数据
        #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
        struct TestStruct {
            id: i32,
            name: String,
        }
        
        let struct_data = TestStruct { id: 1, name: "测试".to_string() };
        let struct_response = ApiResponse::success(struct_data.clone());
        assert_eq!(struct_response.data.unwrap(), struct_data);
    }

    #[test]
    fn test_api_response_empty_data_scenarios() {
        // 业务场景：处理空数据的情况
        
        // 空字符串
        let empty_string_response = ApiResponse::success(String::new());
        assert_eq!(empty_string_response.data.unwrap(), "");
        
        // 空数组
        let empty_vec_response: ApiResponse<Vec<String>> = ApiResponse::success(Vec::new());
        assert!(empty_vec_response.data.unwrap().is_empty());
        
        // Option类型
        let none_response: ApiResponse<Option<String>> = ApiResponse::success(None);
        assert!(none_response.data.unwrap().is_none());
        
        let some_response = ApiResponse::success(Some("有值".to_string()));
        assert_eq!(some_response.data.unwrap().unwrap(), "有值");
    }

    #[test]
    fn test_api_response_error_message_business_rules() {
        // 业务规则：错误消息的格式和内容要求
        let error_scenarios = vec![
            ("", "错误消息不能为空"),
            ("用户名已存在", "用户相关错误"),
            ("密码格式不正确", "验证相关错误"),
            ("数据库连接超时", "系统相关错误"),
            ("权限验证失败", "权限相关错误"),
        ];
        
        for (error_msg, _scenario) in error_scenarios {
            let response: ApiResponse<()> = ApiResponse::error(error_msg);
            
            // 业务规则：错误消息必须保持原样
            assert_eq!(response.message, error_msg);
            
            // 业务规则：错误响应的结构必须一致
            assert!(!response.success);
            assert!(response.data.is_none());
        }
    }

    #[test]
    fn test_api_response_success_message_variations() {
        // 业务需求：支持不同的成功消息
        let user_data = vec!["用户ID: 123"];
        let response1 = ApiResponse::success_with_message(user_data, "用户创建成功");
        assert!(response1.success);
        assert_eq!(response1.message, "用户创建成功");
        
        let response2 = ApiResponse::success_with_message("更新记录", "数据更新完成");
        assert!(response2.success);
        assert_eq!(response2.message, "数据更新完成");
        
        let response3 = ApiResponse::success_with_message("file.txt", "文件上传成功");
        assert!(response3.success);
        assert_eq!(response3.message, "文件上传成功");
        
        let response4 = ApiResponse::success_with_message(true, "邮件发送成功");
        assert!(response4.success);
        assert_eq!(response4.message, "邮件发送成功");
        
        let response5 = ApiResponse::success_with_message(1000, "导出完成");
        assert!(response5.success);
        assert_eq!(response5.message, "导出完成");
    }
}

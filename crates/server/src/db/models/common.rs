use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct PaginationQuery {
    #[validate(range(min = 1, max = 100))]
    pub page: Option<u32>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u32>,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(20),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub current_page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new(current_page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        let has_next = current_page < total_pages;
        let has_prev = current_page > 1;

        Self {
            current_page,
            per_page,
            total,
            total_pages,
            has_next,
            has_prev,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_pagination_query_validation() {
        // 测试有效的分页查询
        let valid_query = PaginationQuery {
            page: Some(1),
            per_page: Some(20),
        };
        assert!(valid_query.validate().is_ok());

        // 测试边界值
        let min_values = PaginationQuery {
            page: Some(1),
            per_page: Some(1),
        };
        assert!(min_values.validate().is_ok());

        let max_values = PaginationQuery {
            page: Some(100),
            per_page: Some(100),
        };
        assert!(max_values.validate().is_ok());

        // 测试无效值 - page为0
        let invalid_page = PaginationQuery {
            page: Some(0),
            per_page: Some(20),
        };
        assert!(invalid_page.validate().is_err());

        // 测试无效值 - per_page过大
        let invalid_per_page = PaginationQuery {
            page: Some(1),
            per_page: Some(101),
        };
        assert!(invalid_per_page.validate().is_err());

        // 测试None值（应该有效）
        let none_values = PaginationQuery {
            page: None,
            per_page: None,
        };
        assert!(none_values.validate().is_ok());
    }

    #[test]
    fn test_pagination_query_default() {
        let default_query = PaginationQuery::default();
        assert_eq!(default_query.page, Some(1));
        assert_eq!(default_query.per_page, Some(20));
        assert!(default_query.validate().is_ok());
    }

    #[test]
    fn test_pagination_info_creation() {
        // 测试第一页
        let info = PaginationInfo::new(1, 10, 95);
        assert_eq!(info.current_page, 1);
        assert_eq!(info.per_page, 10);
        assert_eq!(info.total, 95);
        assert_eq!(info.total_pages, 10); // ceil(95/10) = 10
        assert!(info.has_next);
        assert!(!info.has_prev);

        // 测试中间页
        let info = PaginationInfo::new(5, 10, 95);
        assert_eq!(info.current_page, 5);
        assert_eq!(info.total_pages, 10);
        assert!(info.has_next);
        assert!(info.has_prev);

        // 测试最后一页
        let info = PaginationInfo::new(10, 10, 95);
        assert_eq!(info.current_page, 10);
        assert_eq!(info.total_pages, 10);
        assert!(!info.has_next);
        assert!(info.has_prev);

        // 测试只有一页的情况
        let info = PaginationInfo::new(1, 10, 5);
        assert_eq!(info.current_page, 1);
        assert_eq!(info.total_pages, 1);
        assert!(!info.has_next);
        assert!(!info.has_prev);
    }

    #[test]
    fn test_pagination_info_edge_cases() {
        // 测试总数为0
        let info = PaginationInfo::new(1, 10, 0);
        assert_eq!(info.total_pages, 0);
        assert!(!info.has_next);
        assert!(!info.has_prev);

        // 测试总数刚好是每页数量的倍数
        let info = PaginationInfo::new(2, 10, 20);
        assert_eq!(info.total_pages, 2);
        assert_eq!(info.current_page, 2);
        assert!(!info.has_next);
        assert!(info.has_prev);

        // 测试per_page为1的情况
        let info = PaginationInfo::new(50, 1, 100);
        assert_eq!(info.total_pages, 100);
        assert!(info.has_next);
        assert!(info.has_prev);
    }

    #[test]
    fn test_pagination_response_structure() {
        let data = vec!["item1".to_string(), "item2".to_string()];
        let pagination = PaginationInfo::new(1, 2, 10);
        
        let response = PaginationResponse {
            data: data.clone(),
            pagination,
        };

        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0], "item1");
        assert_eq!(response.pagination.current_page, 1);
        assert_eq!(response.pagination.total, 10);
    }

    #[test]
    fn test_pagination_math_precision() {
        // 测试浮点数计算的精度
        let test_cases = vec![
            (1, 3, 10, 4),    // ceil(10/3) = 4
            (1, 7, 20, 3),    // ceil(20/7) = 3
            (1, 13, 100, 8),  // ceil(100/13) = 8
        ];

        for (page, per_page, total, expected_pages) in test_cases {
            let info = PaginationInfo::new(page, per_page, total);
            assert_eq!(info.total_pages, expected_pages, 
                "For total={}, per_page={}, expected {} pages but got {}", 
                total, per_page, expected_pages, info.total_pages);
        }
    }

    #[test]
    fn test_pagination_boundary_conditions() {
        // 测试边界条件
        let boundary_cases = vec![
            (1, 1, 1, false, false),   // 单页单项
            (2, 1, 2, false, true),   // 最后一页
            (1, 100, 50, false, false), // per_page大于总数
        ];

        for (page, per_page, total, expected_next, expected_prev) in boundary_cases {
            let info = PaginationInfo::new(page, per_page, total);
            assert_eq!(info.has_next, expected_next, 
                "Page {}/{} with total {} should have_next={}", 
                page, per_page, total, expected_next);
            assert_eq!(info.has_prev, expected_prev,
                "Page {}/{} with total {} should have_prev={}", 
                page, per_page, total, expected_prev);
        }
    }
}

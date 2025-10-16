use anyhow::Result;
use sea_orm::*;
use crate::db::models::department::{Entity, Model, ActiveModel, Column, CreateDepartmentDto, UpdateDepartmentDto, DepartmentTreeDto};
use crate::services::UserDepartmentService;

pub struct DepartmentService;

impl DepartmentService {
    /// 创建部门
    pub async fn create_department(db: &DatabaseConnection, dto: CreateDepartmentDto) -> Result<Model> {
                let db = db;

        // 检查部门编码是否已存在
        let existing = Entity::find()
            .filter(Column::Code.eq(&dto.code))
            .one(db)
            .await?;

        if existing.is_some() {
            return Err(anyhow::anyhow!("部门编码已存在"));
        }

        // 检查经理用户是否存在
        if let Some(manager_id) = dto.manager_id {
            let manager_exists = crate::db::models::user::Entity::find_by_id(manager_id)
                .one(db)
                .await?;
            
            if manager_exists.is_none() {
                return Err(anyhow::anyhow!("指定的经理用户不存在"));
            }
        }

        // 计算部门层级
        let level = if let Some(parent_id) = dto.parent_id {
            let parent = Entity::find_by_id(parent_id)
                .one(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("父部门不存在"))?;
            parent.level + 1
        } else {
            1
        };

        let department = ActiveModel {
            name: Set(dto.name),
            code: Set(dto.code),
            parent_id: Set(dto.parent_id),
            level: Set(level),
            sort_order: Set(dto.sort_order),
            description: Set(dto.description),
            manager_id: Set(dto.manager_id),
            is_active: Set(true),
            ..Default::default()
        };

        let result = department.insert(db).await?;
        Ok(result)
    }

    /// 更新部门
    pub async fn update_department(db: &DatabaseConnection, id: i32, dto: UpdateDepartmentDto) -> Result<Model> {
        let db = db;

        let department = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("部门不存在"))?;

        let mut department: ActiveModel = department.into();

        if let Some(name) = dto.name {
            department.name = Set(name);
        }

        if let Some(code) = dto.code {
            // 检查编码是否与其他部门冲突
            let existing = Entity::find()
                .filter(Column::Code.eq(&code))
                .filter(Column::Id.ne(id))
                .one(db)
                .await?;

            if existing.is_some() {
                return Err(anyhow::anyhow!("部门编码已存在"));
            }
            department.code = Set(code);
        }

        if let Some(parent_id) = dto.parent_id {
            // 检查是否形成循环引用
            if parent_id == id {
                return Err(anyhow::anyhow!("不能将自己设为父部门"));
            }

            // 检查父部门是否存在
            let parent = Entity::find_by_id(parent_id)
                .one(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("父部门不存在"))?;

            // 检查是否将子部门设为父部门
            if Self::is_descendant(id, parent_id, db).await? {
                return Err(anyhow::anyhow!("不能将子部门设为父部门"));
            }

            department.level = Set(parent.level + 1);
            department.parent_id = Set(dto.parent_id);
        }

        if let Some(sort_order) = dto.sort_order {
            department.sort_order = Set(sort_order);
        }

        if let Some(description) = dto.description {
            department.description = Set(Some(description));
        }

        if let Some(manager_id) = dto.manager_id {
            // 检查经理用户是否存在
            let manager_exists = crate::db::models::user::Entity::find_by_id(manager_id)
                .one(db)
                .await?;
            
            if manager_exists.is_none() {
                return Err(anyhow::anyhow!("指定的经理用户不存在"));
            }
            department.manager_id = Set(Some(manager_id));
        }

        if let Some(is_active) = dto.is_active {
            department.is_active = Set(is_active);
        }

        let result = department.update(db).await?;
        Ok(result)
    }

    /// 删除部门
    pub async fn delete_department(db: &DatabaseConnection, id: i32) -> Result<bool> {
        let db = db;

        // 检查是否有子部门
        let children = Entity::find()
            .filter(Column::ParentId.eq(id))
            .count(db)
            .await?;

        if children > 0 {
            return Err(anyhow::anyhow!("请先删除子部门"));
        }

        // 检查是否有用户关联
        let user_count = UserDepartmentService::count_users_by_department(db, id).await?;
        if user_count > 0 {
            return Err(anyhow::anyhow!("部门下还有用户，无法删除"));
        }

        let result = Entity::delete_by_id(id)
            .exec(db)
            .await?;

        Ok(result.rows_affected > 0)
    }

    /// 获取部门详情
    pub async fn get_department(db: &DatabaseConnection, id: i32) -> Result<Option<Model>> {
        let db = db;
        let department = Entity::find_by_id(id).one(db).await?;
        Ok(department)
    }

    /// 获取部门列表
    pub async fn list_departments(db: &DatabaseConnection) -> Result<Vec<Model>> {
        let db = db;
        let departments = Entity::find()
            .order_by(Column::SortOrder, Order::Asc)
            .order_by(Column::Id, Order::Asc)
            .all(db)
            .await?;
        Ok(departments)
    }

    /// 获取部门树形结构
    pub async fn get_department_tree(db: &DatabaseConnection) -> Result<Vec<DepartmentTreeDto>> {
        let departments = Self::list_departments(db).await?;
        let mut tree = Vec::new();
        let mut map = std::collections::HashMap::new();

        // 获取每个部门的用户数量
        let _db = db;

        for dept in &departments {
            let user_count = UserDepartmentService::count_users_by_department(db, dept.id).await?;

            let dto = DepartmentTreeDto {
                id: dept.id,
                name: dept.name.clone(),
                code: dept.code.clone(),
                level: dept.level,
                sort_order: dept.sort_order,
                description: dept.description.clone(),
                manager_id: dept.manager_id,
                is_active: dept.is_active,
                children: Vec::new(),
                user_count,
            };

            map.insert(dept.id, dto);
        }

        // 构建树形结构
        // 先处理根部门，再处理子部门
        for dept in &departments {
            let id = dept.id;
            if dept.parent_id.is_none() {
                // 根部门直接添加到树中
                if let Some(root) = map.remove(&id) {
                    tree.push(root);
                }
            }
        }
        
        // 然后处理所有子部门
        for dept in &departments {
            let id = dept.id;
            if let Some(parent_id) = dept.parent_id {
                // 先移除子节点，避免借用冲突
                if let Some(child) = map.remove(&id) {
                    // 在树中查找父部门
                    if let Some(parent) = Self::find_parent_in_tree(&mut tree, parent_id) {
                        parent.children.push(child);
                    }
                }
            }
        }

        // 递归排序
        Self::sort_department_tree(&mut tree);

        Ok(tree)
    }

    /// 检查是否为后代部门
    async fn is_descendant(department_id: i32, ancestor_id: i32, db: &DatabaseConnection) -> Result<bool> {
        let mut current_id = department_id;

        loop {
            let department = Entity::find_by_id(current_id)
                .one(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("部门不存在"))?;

            if let Some(parent_id) = department.parent_id {
                if parent_id == ancestor_id {
                    return Ok(true);
                }
                current_id = parent_id;
            } else {
                break;
            }
        }

        Ok(false)
    }

    /// 递归排序部门树
    fn sort_department_tree(tree: &mut Vec<DepartmentTreeDto>) {
        tree.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.id.cmp(&b.id)));

        for child in tree.iter_mut() {
            Self::sort_department_tree(&mut child.children);
        }
    }

    /// 在树中递归查找父部门
    fn find_parent_in_tree(tree: &mut Vec<DepartmentTreeDto>, parent_id: i32) -> Option<&mut DepartmentTreeDto> {
        for node in tree.iter_mut() {
            if node.id == parent_id {
                return Some(node);
            }
            // 递归查找子节点
            if let Some(found) = Self::find_parent_in_tree(&mut node.children, parent_id) {
                return Some(found);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_parent_in_tree() {
        let child1 = DepartmentTreeDto {
            id: 3,
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

        let child2 = DepartmentTreeDto {
            id: 4,
            name: "前端开发组".to_string(),
            code: "FRONTEND".to_string(),
            level: 3,
            sort_order: 2,
            description: Some("负责前端开发".to_string()),
            manager_id: Some(4),
            is_active: true,
            children: vec![],
            user_count: 2,
        };

        let parent = DepartmentTreeDto {
            id: 2,
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            level: 2,
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
            is_active: true,
            children: vec![child1, child2],
            user_count: 8,
        };

        let mut tree = vec![parent];

        // 测试查找根节点
        let found_root = DepartmentService::find_parent_in_tree(&mut tree, 2);
        assert!(found_root.is_some());
        assert_eq!(found_root.unwrap().name, "技术部");

        // 测试查找子节点
        let found_child = DepartmentService::find_parent_in_tree(&mut tree, 3);
        assert!(found_child.is_some());
        assert_eq!(found_child.unwrap().name, "后端开发组");

        // 测试查找不存在的节点
        let not_found = DepartmentService::find_parent_in_tree(&mut tree, 999);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_sort_department_tree() {
        let child1 = DepartmentTreeDto {
            id: 4,
            name: "前端开发组".to_string(),
            code: "FRONTEND".to_string(),
            level: 3,
            sort_order: 2,
            description: Some("负责前端开发".to_string()),
            manager_id: Some(4),
            is_active: true,
            children: vec![],
            user_count: 2,
        };

        let child2 = DepartmentTreeDto {
            id: 3,
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

        let parent = DepartmentTreeDto {
            id: 2,
            name: "技术部".to_string(),
            code: "TECH".to_string(),
            level: 2,
            sort_order: 1,
            description: Some("负责技术开发".to_string()),
            manager_id: Some(2),
            is_active: true,
            children: vec![child1, child2], // 故意放错顺序
            user_count: 8,
        };

        let mut tree = vec![parent];

        // 排序前，检查顺序是错误的
        assert_eq!(tree[0].children[0].sort_order, 2);
        assert_eq!(tree[0].children[1].sort_order, 1);

        // 执行排序
        DepartmentService::sort_department_tree(&mut tree);

        // 排序后，检查顺序是正确的
        assert_eq!(tree[0].children[0].sort_order, 1);
        assert_eq!(tree[0].children[1].sort_order, 2);
        assert_eq!(tree[0].children[0].name, "后端开发组");
        assert_eq!(tree[0].children[1].name, "前端开发组");
    }

    #[test]
    fn test_department_tree_building_logic() {
        // 模拟部门数据
        let departments = vec![
            Model {
                id: 1,
                name: "总公司".to_string(),
                code: "HQ".to_string(),
                parent_id: None,
                level: 1,
                sort_order: 1,
                description: Some("公司根部门".to_string()),
                manager_id: Some(1),
                is_active: true,
                created_at: chrono::Utc::now().into(),
                updated_at: chrono::Utc::now().into(),
            },
            Model {
                id: 2,
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
            },
            Model {
                id: 3,
                name: "人事部".to_string(),
                code: "HR".to_string(),
                parent_id: Some(1),
                level: 2,
                sort_order: 2,
                description: Some("负责人事管理".to_string()),
                manager_id: Some(3),
                is_active: true,
                created_at: chrono::Utc::now().into(),
                updated_at: chrono::Utc::now().into(),
            },
        ];

        // 模拟树构建逻辑
        let mut tree = Vec::new();
        let mut map = std::collections::HashMap::new();

        // 创建DTO
        for dept in &departments {
            let dto = DepartmentTreeDto {
                id: dept.id,
                name: dept.name.clone(),
                code: dept.code.clone(),
                level: dept.level,
                sort_order: dept.sort_order,
                description: dept.description.clone(),
                manager_id: dept.manager_id,
                is_active: dept.is_active,
                children: Vec::new(),
                user_count: 0,
            };
            map.insert(dept.id, dto);
        }

        // 先处理根部门
        for dept in &departments {
            let id = dept.id;
            if dept.parent_id.is_none() {
                if let Some(root) = map.remove(&id) {
                    tree.push(root);
                }
            }
        }

        // 再处理子部门
        for dept in &departments {
            let id = dept.id;
            if let Some(_parent_id) = dept.parent_id {
                if let Some(child) = map.remove(&id) {
                    if let Some(parent) = DepartmentService::find_parent_in_tree(&mut tree, _parent_id) {
                        parent.children.push(child);
                    }
                }
            }
        }

        // 验证树结构
        assert_eq!(tree.len(), 1, "应该只有一个根部门");
        assert_eq!(tree[0].id, 1);
        assert_eq!(tree[0].name, "总公司");
        assert_eq!(tree[0].children.len(), 2);
        
        // 验证子部门
        let tech_dept = tree[0].children.iter().find(|d| d.id == 2);
        assert!(tech_dept.is_some());
        assert_eq!(tech_dept.unwrap().name, "技术部");
        
        let hr_dept = tree[0].children.iter().find(|d| d.id == 3);
        assert!(hr_dept.is_some());
        assert_eq!(hr_dept.unwrap().name, "人事部");
    }

    #[test]
    fn test_create_department_dto_conversion() {
        let dto = CreateDepartmentDto {
            name: "测试部门".to_string(),
            code: "TEST".to_string(),
            parent_id: Some(1),
            sort_order: 5,
            description: Some("测试描述".to_string()),
            manager_id: Some(2),
        };

        // 验证DTO字段
        assert_eq!(dto.name, "测试部门");
        assert_eq!(dto.code, "TEST");
        assert_eq!(dto.parent_id, Some(1));
        assert_eq!(dto.sort_order, 5);
        assert_eq!(dto.description, Some("测试描述".to_string()));
        assert_eq!(dto.manager_id, Some(2));
    }

    #[test]
    fn test_update_department_dto_partial_update() {
        let dto = UpdateDepartmentDto {
            name: Some("更新的部门名称".to_string()),
            code: None, // 不更新编码
            parent_id: Some(2),
            sort_order: Some(10),
            description: None, // 不更新描述
            manager_id: Some(3),
            is_active: Some(false),
        };

        // 验证部分更新的字段
        assert_eq!(dto.name, Some("更新的部门名称".to_string()));
        assert_eq!(dto.code, None);
        assert_eq!(dto.parent_id, Some(2));
        assert_eq!(dto.sort_order, Some(10));
        assert_eq!(dto.description, None);
        assert_eq!(dto.manager_id, Some(3));
        assert_eq!(dto.is_active, Some(false));
    }

    #[test]
    fn test_department_hierarchy_logic() {
        // 测试部门层级逻辑
        let root_level = 1;
        let child_level = root_level + 1;
        let grandchild_level = child_level + 1;

        assert_eq!(child_level, 2);
        assert_eq!(grandchild_level, 3);

        // 测试最大层级限制（假设最大10层）
        let max_level = 10;
        assert!(grandchild_level <= max_level);
    }

    #[test]
    fn test_department_sorting_criteria() {
        let mut departments = vec![
            DepartmentTreeDto {
                id: 3,
                name: "C部门".to_string(),
                code: "C".to_string(),
                level: 1,
                sort_order: 2,
                description: None,
                manager_id: None,
                is_active: true,
                children: vec![],
                user_count: 0,
            },
            DepartmentTreeDto {
                id: 1,
                name: "A部门".to_string(),
                code: "A".to_string(),
                level: 1,
                sort_order: 1,
                description: None,
                manager_id: None,
                is_active: true,
                children: vec![],
                user_count: 0,
            },
            DepartmentTreeDto {
                id: 2,
                name: "B部门".to_string(),
                code: "B".to_string(),
                level: 1,
                sort_order: 1, // 与A部门相同的sort_order
                description: None,
                manager_id: None,
                is_active: true,
                children: vec![],
                user_count: 0,
            },
        ];

        // 按sort_order排序，如果相同则按id排序
        departments.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.id.cmp(&b.id)));

        // 验证排序结果
        assert_eq!(departments[0].id, 1); // sort_order=1, id=1
        assert_eq!(departments[1].id, 2); // sort_order=1, id=2
        assert_eq!(departments[2].id, 3); // sort_order=2, id=3
    }
}

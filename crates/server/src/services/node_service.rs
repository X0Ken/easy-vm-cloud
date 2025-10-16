/// 节点管理服务

use chrono::Utc;
use uuid::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};

use crate::db::models::node::{
    CreateNodeDto, UpdateNodeDto, NodeResponse, NodeListResponse, NodeStatus, 
    NodeHeartbeatDto, NodeStatsResponse, Entity as NodeEntity, Column as NodeColumn, 
    ActiveModel as NodeActiveModel,
};
use crate::app_state::AppState;

pub struct NodeService {
    state: AppState,
}

impl NodeService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 创建节点
    pub async fn create_node(&self, dto: CreateNodeDto) -> anyhow::Result<NodeResponse> {
        let db = &self.state.sea_db();
        
        // 生成节点 ID
        let node_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // 检查 IP 地址是否已存在
        let existing = NodeEntity::find()
            .filter(NodeColumn::IpAddress.eq(&dto.ip_address))
            .one(db)
            .await?;

        if existing.is_some() {
            return Err(anyhow::anyhow!("该 IP 地址已被使用"));
        }

        // 创建 ActiveModel
        let node_active = NodeActiveModel {
            id: Set(node_id),
            hostname: Set(dto.hostname),
            ip_address: Set(dto.ip_address),
            status: Set(NodeStatus::Offline.as_str().to_string()),
            hypervisor_type: Set(dto.hypervisor_type),
            hypervisor_version: Set(dto.hypervisor_version),
            cpu_cores: Set(None),
            cpu_threads: Set(None),
            memory_total: Set(None),
            disk_total: Set(None),
            metadata: Set(dto.metadata),
            last_heartbeat: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        // 插入数据库
        let node = node_active.insert(db).await?;

        Ok(NodeResponse::from(node))
    }

    /// 获取节点列表
    pub async fn list_nodes(
        &self,
        page: usize,
        page_size: usize,
        status: Option<String>,
    ) -> anyhow::Result<NodeListResponse> {
        let db = &self.state.sea_db();

        let mut query = NodeEntity::find();

        // 状态过滤
        if let Some(status) = status {
            query = query.filter(NodeColumn::Status.eq(status));
        }

        // 按更新时间降序排序
        query = query.order_by_desc(NodeColumn::UpdatedAt);

        // 计算总数
        let total = query.clone().count(db).await?;

        // 分页查询
        let nodes = query
            .offset(((page - 1) * page_size) as u64)
            .limit(page_size as u64)
            .all(db)
            .await?;

        let node_responses: Vec<NodeResponse> = nodes.into_iter().map(NodeResponse::from).collect();

        Ok(NodeListResponse {
            nodes: node_responses,
            total,
            page,
            page_size,
        })
    }

    /// 获取单个节点详情
    pub async fn get_node(&self, id: &str) -> anyhow::Result<NodeResponse> {
        let db = &self.state.sea_db();

        let node = NodeEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("节点不存在"))?;

        Ok(NodeResponse::from(node))
    }

    /// 更新节点
    pub async fn update_node(&self, id: &str, dto: UpdateNodeDto) -> anyhow::Result<NodeResponse> {
        let db = &self.state.sea_db();

        // 查询节点
        let node = NodeEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("节点不存在"))?;

        // 如果更新 IP 地址，检查是否已被使用
        if let Some(ref ip_address) = dto.ip_address {
            let existing = NodeEntity::find()
                .filter(NodeColumn::IpAddress.eq(ip_address))
                .filter(NodeColumn::Id.ne(id))
                .one(db)
                .await?;

            if existing.is_some() {
                return Err(anyhow::anyhow!("该 IP 地址已被使用"));
            }
        }

        let now = Utc::now();
        let mut node_active: NodeActiveModel = node.into();

        // 更新字段
        if let Some(hostname) = dto.hostname {
            node_active.hostname = Set(hostname);
        }
        if let Some(ip_address) = dto.ip_address {
            node_active.ip_address = Set(ip_address);
        }
        if let Some(status) = dto.status {
            node_active.status = Set(status);
        }
        if let Some(hypervisor_type) = dto.hypervisor_type {
            node_active.hypervisor_type = Set(Some(hypervisor_type));
        }
        if let Some(hypervisor_version) = dto.hypervisor_version {
            node_active.hypervisor_version = Set(Some(hypervisor_version));
        }
        if let Some(metadata) = dto.metadata {
            node_active.metadata = Set(Some(metadata));
        }

        node_active.updated_at = Set(now.into());

        // 更新数据库
        let updated_node = node_active.update(db).await?;

        Ok(NodeResponse::from(updated_node))
    }

    /// 删除节点
    pub async fn delete_node(&self, id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询节点
        let node = NodeEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("节点不存在"))?;

        // 检查是否有虚拟机在该节点上
        use sea_orm::FromQueryResult;
        
        #[derive(FromQueryResult)]
        struct VmCount {
            count: i64,
        }

        let vm_count = VmCount::find_by_statement(
            sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT COUNT(*) as count FROM vms WHERE node_id = $1",
                vec![id.to_string().into()],
            )
        )
        .one(db)
        .await?;

        if let Some(vm_count) = vm_count {
            if vm_count.count > 0 {
                return Err(anyhow::anyhow!("该节点上还有虚拟机，无法删除"));
            }
        }

        // 删除节点
        let node_active: NodeActiveModel = node.into();
        node_active.delete(db).await?;

        Ok(())
    }

    /// 更新节点心跳
    pub async fn update_heartbeat(&self, id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询节点
        let node = NodeEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("节点不存在"))?;

        let now = Utc::now();
        let mut node_active: NodeActiveModel = node.into();

        // 更新心跳时间和资源信息
        node_active.last_heartbeat = Set(Some(now.into()));
        node_active.status = Set(NodeStatus::Online.as_str().to_string());

        // 更新数据库
        node_active.update(db).await?;

        Ok(())
    }

    /// 更新节点资源信息
    pub async fn update_node_resource_info(
        &self,
        id: &str,
        cpu_cores: u32,
        cpu_threads: u32,
        memory_total: u64,
        disk_total: u64,
        hypervisor_type: Option<String>,
        hypervisor_version: Option<String>,
    ) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 查询节点
        let node = NodeEntity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("节点不存在"))?;

        let now = Utc::now();
        let mut node_active: NodeActiveModel = node.into();

        // 更新资源信息
        node_active.cpu_cores = Set(Some(cpu_cores as i32));
        node_active.cpu_threads = Set(Some(cpu_threads as i32));
        node_active.memory_total = Set(Some(memory_total as i64));
        node_active.disk_total = Set(Some(disk_total as i64));
        
        // 更新虚拟化信息（如果提供）
        if let Some(hypervisor_type) = hypervisor_type {
            node_active.hypervisor_type = Set(Some(hypervisor_type));
        }
        if let Some(hypervisor_version) = hypervisor_version {
            node_active.hypervisor_version = Set(Some(hypervisor_version));
        }
        
        // 更新心跳时间和状态
        node_active.last_heartbeat = Set(Some(now.into()));
        node_active.status = Set(NodeStatus::Online.as_str().to_string());
        node_active.updated_at = Set(now.into());

        // 更新数据库
        node_active.update(db).await?;

        Ok(())
    }

    /// 检查并更新超时的节点状态
    /// 将超过指定时间（秒）未收到心跳的在线节点标记为离线
    pub async fn check_and_update_timeout_nodes(&self, timeout_secs: u64) -> anyhow::Result<Vec<String>> {
        let db = &self.state.sea_db();
        let now = Utc::now();
        let timeout_duration = chrono::Duration::seconds(timeout_secs as i64);
        let timeout_threshold = now - timeout_duration;

        // 查找所有在线但心跳超时的节点
        let timeout_nodes = NodeEntity::find()
            .filter(NodeColumn::Status.eq(NodeStatus::Online.as_str()))
            .filter(NodeColumn::LastHeartbeat.lt(timeout_threshold))
            .all(db)
            .await?;

        let mut updated_node_ids = Vec::new();

        // 更新超时节点状态为离线
        for node in timeout_nodes {
            let mut node_active: NodeActiveModel = node.clone().into();
            node_active.status = Set(NodeStatus::Offline.as_str().to_string());
            node_active.updated_at = Set(now.into());

            node_active.update(db).await?;
            updated_node_ids.push(node.id.clone());
            
            tracing::warn!("节点心跳超时，已标记为离线: node_id={}, last_heartbeat={:?}", 
                          node.id, node.last_heartbeat);
        }

        if !updated_node_ids.is_empty() {
            tracing::info!("已更新 {} 个超时节点状态为离线", updated_node_ids.len());
        }

        Ok(updated_node_ids)
    }

    /// 获取节点统计信息
    pub async fn get_stats(&self) -> anyhow::Result<NodeStatsResponse> {
        let db = &self.state.sea_db();

        use sea_orm::FromQueryResult;
        
        #[derive(FromQueryResult)]
        struct StatusCount {
            status: String,
            count: i64,
        }

        let status_counts = StatusCount::find_by_statement(
            sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT status, COUNT(*) as count FROM nodes GROUP BY status",
                vec![],
            )
        )
        .all(db)
        .await?;

        let total_nodes = NodeEntity::find().count(db).await? as i64;
        let mut online_nodes = 0i64;
        let mut offline_nodes = 0i64;
        let mut maintenance_nodes = 0i64;
        let mut error_nodes = 0i64;

        for status_count in status_counts {
            match status_count.status.as_str() {
                "online" => online_nodes = status_count.count,
                "offline" => offline_nodes = status_count.count,
                "maintenance" => maintenance_nodes = status_count.count,
                "error" => error_nodes = status_count.count,
                _ => {}
            }
        }

        Ok(NodeStatsResponse {
            total_nodes,
            online_nodes,
            offline_nodes,
            maintenance_nodes,
            error_nodes,
        })
    }
}

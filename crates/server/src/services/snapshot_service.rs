use anyhow::{anyhow, Result};
/// 快照管理服务
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::db::models::snapshot::{
    ActiveModel as SnapshotActiveModel, Column as SnapshotColumn, CreateSnapshotDto,
    Entity as SnapshotEntity, SnapshotListResponse, SnapshotResponse, SnapshotStatus,
    UpdateSnapshotDto,
};
use crate::db::models::storage_pool::Entity as StoragePoolEntity;
use crate::db::models::volume::Entity as VolumeEntity;
use crate::ws::frontend_handler::FrontendMessage;

use tracing::{error, info, warn};

pub struct SnapshotService {
    state: AppState,
}

impl SnapshotService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 广播快照状态更新到前端
    async fn notify_snapshot_status_update(
        &self,
        snapshot_id: &str,
        status: &str,
        message: Option<&str>,
    ) {
        let frontend_msg = FrontendMessage::SnapshotStatusUpdate {
            snapshot_id: snapshot_id.to_string(),
            status: status.to_string(),
            message: message.map(|s| s.to_string()),
        };

        let count = self.state.frontend_manager().broadcast(frontend_msg).await;
        if count > 0 {
            info!(
                "已向 {} 个前端连接发送快照 {} 状态更新: {}",
                count, snapshot_id, status
            );
        }
    }

    /// 创建快照
    pub async fn create_snapshot(&self, dto: CreateSnapshotDto) -> Result<SnapshotResponse> {
        let db = &self.state.sea_db();

        // 查找存储卷
        let volume = VolumeEntity::find_by_id(&dto.volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("存储卷不存在"))?;

        // 查找存储池以获取节点信息
        let pool = StoragePoolEntity::find_by_id(&volume.pool_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("存储池不存在"))?;

        let node_id = pool.node_id.ok_or_else(|| anyhow!("存储池未关联节点"))?;

        // 创建快照记录，状态为creating
        let snapshot_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let snapshot_active = SnapshotActiveModel {
            id: Set(snapshot_id.clone()),
            name: Set(dto.name.clone()),
            volume_id: Set(dto.volume_id.clone()),
            status: Set(SnapshotStatus::Creating.as_str().to_string()),
            size_gb: Set(Some(volume.size_gb)),
            snapshot_tag: Set(None),
            description: Set(dto.description.clone()),
            metadata: Set(dto.metadata.clone()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let snapshot = snapshot_active.insert(db).await?;
        info!(
            "创建快照记录: snapshot_id={}, volume_id={}",
            snapshot_id, dto.volume_id
        );

        // 构造 Agent 请求
        let request = serde_json::json!({
            "snapshot_id": snapshot_id,
            "volume_id": dto.volume_id,
            "snapshot_name": dto.name,
            "pool_id": volume.pool_id,
        });

        // 异步通知 Agent 创建快照，不等待结果
        self.state
            .agent_manager()
            .notify(&node_id, "create_snapshot_async", request)
            .await
            .map_err(|e| anyhow!("发送创建快照通知失败: {}", e))?;

        info!("快照 {} 创建通知已发送给 Agent", snapshot_id);

        let mut response = SnapshotResponse::from(snapshot);
        response.volume_name = Some(volume.name);
        Ok(response)
    }

    /// 删除快照
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let db = &self.state.sea_db();

        // 查找快照
        let snapshot = SnapshotEntity::find_by_id(snapshot_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("快照不存在"))?;

        // 查找存储卷
        let volume = VolumeEntity::find_by_id(&snapshot.volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("存储卷不存在"))?;

        // 查找存储池以获取节点信息
        let pool = StoragePoolEntity::find_by_id(&volume.pool_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("存储池不存在"))?;

        let node_id = pool.node_id.ok_or_else(|| anyhow!("存储池未关联节点"))?;

        // 更新快照状态为 deleting
        let mut snapshot_active: SnapshotActiveModel = snapshot.clone().into();
        snapshot_active.status = Set(SnapshotStatus::Deleting.as_str().to_string());
        snapshot_active.updated_at = Set(Utc::now().into());
        snapshot_active.update(db).await?;

        info!("标记快照为删除中: snapshot_id={}", snapshot_id);

        // 构造 Agent 请求
        let request = serde_json::json!({
            "snapshot_id": snapshot_id.to_string(),
            "volume_id": snapshot.volume_id,
            "pool_id": volume.pool_id,
        });

        // 异步通知 Agent 删除快照，不等待结果
        self.state
            .agent_manager()
            .notify(&node_id, "delete_snapshot_async", request)
            .await
            .map_err(|e| anyhow!("发送删除快照通知失败: {}", e))?;

        info!("快照 {} 删除通知已发送给 Agent", snapshot_id);
        Ok(())
    }

    /// 恢复快照（异步操作，不等待 Agent 响应）
    pub async fn restore_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let db = &self.state.sea_db();

        // 查找快照
        let snapshot = SnapshotEntity::find_by_id(snapshot_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("快照不存在"))?;

        // 查找存储卷
        let volume = VolumeEntity::find_by_id(&snapshot.volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("存储卷不存在"))?;

        // 检查存储卷是否在使用中
        if volume.status == "in-use" || volume.vm_id.is_some() {
            return Err(anyhow!(
                "存储卷正在被虚拟机使用，需要先停止虚拟机才能恢复快照"
            ));
        }

        // 查找存储池以获取节点信息
        let pool = StoragePoolEntity::find_by_id(&volume.pool_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("存储池不存在"))?;

        let node_id = pool.node_id.ok_or_else(|| anyhow!("存储池未关联节点"))?;

        // 更新快照状态为 restoring（使用 available 状态表示正在恢复）
        let mut snapshot_active: SnapshotActiveModel = snapshot.clone().into();
        let now = Utc::now();
        snapshot_active.updated_at = Set(now.into());
        snapshot_active.update(db).await?;

        info!(
            "开始恢复快照: snapshot_id={}, volume_id={}",
            snapshot_id, snapshot.volume_id
        );

        // 构造 Agent 请求
        let request = serde_json::json!({
            "snapshot_id": snapshot.snapshot_tag.clone().unwrap_or(snapshot_id.to_string()),
            "volume_id": snapshot.volume_id,
            "pool_id": volume.pool_id,
        });

        // 异步通知 Agent 恢复快照，不等待结果
        self.state
            .agent_manager()
            .notify(&node_id, "restore_snapshot_async", request)
            .await
            .map_err(|e| anyhow!("发送恢复快照通知失败: {}", e))?;

        info!("快照 {} 恢复通知已发送给 Agent", snapshot_id);
        Ok(())
    }

    /// 处理 Agent 的快照操作完成通知
    pub async fn handle_snapshot_operation_completed(
        &self,
        snapshot_id: &str,
        operation: &str,
        success: bool,
        message: &str,
    ) -> Result<()> {
        let db = &self.state.sea_db();
        let now = Utc::now();

        // 查询快照信息
        let snapshot = SnapshotEntity::find_by_id(snapshot_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("快照不存在"))?;

        let mut snapshot_active: SnapshotActiveModel = snapshot.into();

        match operation {
            "create_snapshot" => {
                if success {
                    snapshot_active.status = Set(SnapshotStatus::Available.as_str().to_string());
                    // 从 message 中提取 snapshot_tag（如果有）
                    if !message.is_empty() && message.starts_with("snapshot_tag:") {
                        let snapshot_tag = message.replace("snapshot_tag:", "").trim().to_string();
                        snapshot_active.snapshot_tag = Set(Some(snapshot_tag));
                    }
                    self.notify_snapshot_status_update(
                        snapshot_id,
                        "available",
                        Some("快照创建成功"),
                    )
                    .await;
                } else {
                    snapshot_active.status = Set(SnapshotStatus::Error.as_str().to_string());
                    self.notify_snapshot_status_update(
                        snapshot_id,
                        "error",
                        Some(&format!("快照创建失败: {}", message)),
                    )
                    .await;
                }
            }
            "delete_snapshot" => {
                if success {
                    // 删除成功，先广播通知再删除数据库记录
                    self.notify_snapshot_status_update(
                        snapshot_id,
                        "deleted",
                        Some("快照删除成功"),
                    )
                    .await;
                    snapshot_active.delete(db).await?;
                    info!("快照 {} 删除成功并已从数据库移除", snapshot_id);
                    return Ok(());
                } else {
                    snapshot_active.status = Set(SnapshotStatus::Error.as_str().to_string());
                    self.notify_snapshot_status_update(
                        snapshot_id,
                        "error",
                        Some(&format!("快照删除失败: {}", message)),
                    )
                    .await;
                }
            }
            "restore_snapshot" => {
                if success {
                    // 恢复成功，保持快照状态为 available
                    snapshot_active.status = Set(SnapshotStatus::Available.as_str().to_string());
                    self.notify_snapshot_status_update(
                        snapshot_id,
                        "available",
                        Some("快照恢复成功"),
                    )
                    .await;
                    info!("快照 {} 恢复成功", snapshot_id);
                } else {
                    // 恢复失败，标记为错误状态
                    snapshot_active.status = Set(SnapshotStatus::Error.as_str().to_string());
                    self.notify_snapshot_status_update(
                        snapshot_id,
                        "error",
                        Some(&format!("快照恢复失败: {}", message)),
                    )
                    .await;
                    error!("快照 {} 恢复失败: {}", snapshot_id, message);
                }
            }
            _ => {
                warn!("未知的快照操作: {}", operation);
                return Ok(());
            }
        }

        snapshot_active.updated_at = Set(now.into());
        snapshot_active.update(db).await?;

        info!(
            "快照 {} 操作 {} 完成: success={}, message={}",
            snapshot_id, operation, success, message
        );
        Ok(())
    }

    /// 获取快照列表
    pub async fn list_snapshots(
        &self,
        page: usize,
        page_size: usize,
        volume_id: Option<String>,
        status: Option<String>,
    ) -> Result<SnapshotListResponse> {
        let db = &self.state.sea_db();

        let mut query = SnapshotEntity::find();

        if let Some(vid) = volume_id {
            query = query.filter(SnapshotColumn::VolumeId.eq(vid));
        }

        if let Some(s) = status {
            query = query.filter(SnapshotColumn::Status.eq(s));
        }

        let total = query.clone().count(db).await? as usize;

        let snapshots = query
            .order_by_desc(SnapshotColumn::CreatedAt)
            .offset(((page - 1) * page_size) as u64)
            .limit(page_size as u64)
            .all(db)
            .await?;

        // 获取所有相关的存储卷信息
        let mut snapshot_responses = Vec::new();

        for snapshot in snapshots {
            let mut snapshot_response = SnapshotResponse::from(snapshot.clone());

            // 获取存储卷名称
            if let Ok(Some(volume)) = VolumeEntity::find_by_id(&snapshot.volume_id).one(db).await {
                snapshot_response.volume_name = Some(volume.name);
            }

            snapshot_responses.push(snapshot_response);
        }

        Ok(SnapshotListResponse {
            snapshots: snapshot_responses,
            total,
            page,
            page_size,
        })
    }

    /// 获取单个快照
    pub async fn get_snapshot(&self, snapshot_id: &str) -> Result<SnapshotResponse> {
        let db = &self.state.sea_db();

        let snapshot = SnapshotEntity::find_by_id(snapshot_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("快照不存在"))?;

        let mut response = SnapshotResponse::from(snapshot.clone());

        // 获取存储卷名称
        if let Ok(Some(volume)) = VolumeEntity::find_by_id(&snapshot.volume_id).one(db).await {
            response.volume_name = Some(volume.name);
        }

        Ok(response)
    }

    /// 更新快照（仅允许更新名称和描述）
    pub async fn update_snapshot(
        &self,
        snapshot_id: &str,
        dto: UpdateSnapshotDto,
    ) -> Result<SnapshotResponse> {
        let db = &self.state.sea_db();

        let snapshot = SnapshotEntity::find_by_id(snapshot_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow!("快照不存在"))?;

        let mut snapshot_active: SnapshotActiveModel = snapshot.clone().into();

        // 仅允许更新名称和描述
        if let Some(name) = dto.name {
            snapshot_active.name = Set(name);
        }
        if let Some(description) = dto.description {
            snapshot_active.description = Set(Some(description));
        }

        snapshot_active.updated_at = Set(Utc::now().into());

        let updated_snapshot = snapshot_active.update(db).await?;

        let mut response = SnapshotResponse::from(updated_snapshot);

        // 获取存储卷名称
        if let Ok(Some(volume)) = VolumeEntity::find_by_id(&snapshot.volume_id).one(db).await {
            response.volume_name = Some(volume.name);
        }

        info!("快照 {} 已更新", snapshot_id);
        Ok(response)
    }
}

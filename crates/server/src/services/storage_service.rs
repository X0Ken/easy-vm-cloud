/// 存储管理服务

use chrono::Utc;
use uuid::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::db::models::storage_pool::{
    CreateStoragePoolDto, UpdateStoragePoolDto, StoragePoolListResponse, StoragePoolResponse,
    Entity as StoragePoolEntity, Column as StoragePoolColumn, ActiveModel as StoragePoolActiveModel,
};
use crate::db::models::volume::{
    CreateVolumeDto, UpdateVolumeDto, ResizeVolumeDto, VolumeListResponse, VolumeResponse, VolumeStatus,
    Entity as VolumeEntity, Column as VolumeColumn, ActiveModel as VolumeActiveModel,
};
use crate::db::models::vm::Entity as VmEntity;
use crate::app_state::AppState;
use common::ws_rpc::{
    CreateVolumeRequest, DeleteVolumeRequest, ResizeVolumeRequest, SnapshotVolumeRequest,
    CreateVolumeResponse, DeleteVolumeResponse, ResizeVolumeResponse,
};
use tracing::warn;
use std::time::Duration;

pub struct StorageService {
    state: AppState,
}

impl StorageService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 创建存储池
    pub async fn create_storage_pool(&self, dto: CreateStoragePoolDto) -> anyhow::Result<StoragePoolResponse> {
        let pool_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let pool_active = StoragePoolActiveModel {
            id: Set(pool_id),
            name: Set(dto.name),
            pool_type: Set(dto.pool_type),
            status: Set("active".to_string()),
            config: Set(dto.config),
            capacity_gb: Set(dto.capacity_gb),
            allocated_gb: Set(Some(0)),
            available_gb: Set(dto.capacity_gb),
            metadata: Set(dto.metadata),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let pool = pool_active.insert(&self.state.sea_db()).await?;
        Ok(StoragePoolResponse::from(pool))
    }

    /// 获取存储池列表
    pub async fn list_storage_pools(
        &self,
        page: usize,
        page_size: usize,
        pool_type: Option<String>,
        status: Option<String>,
    ) -> anyhow::Result<StoragePoolListResponse> {
        let db = &self.state.sea_db();

        let mut query = StoragePoolEntity::find();

        if let Some(pt) = pool_type {
            query = query.filter(StoragePoolColumn::PoolType.eq(pt));
        }

        if let Some(s) = status {
            query = query.filter(StoragePoolColumn::Status.eq(s));
        }

        let total = query.clone().count(db).await? as usize;

        let pools = query
            .order_by_desc(StoragePoolColumn::CreatedAt)
            .offset(((page - 1) * page_size) as u64)
            .limit(page_size as u64)
            .all(db)
            .await?;

        let pool_responses: Vec<StoragePoolResponse> = pools.into_iter().map(StoragePoolResponse::from).collect();

        Ok(StoragePoolListResponse {
            pools: pool_responses,
            total,
            page,
            page_size,
        })
    }

    /// 获取单个存储池
    pub async fn get_storage_pool(&self, pool_id: &str) -> anyhow::Result<StoragePoolResponse> {
        let db = &self.state.sea_db();

        let pool = StoragePoolEntity::find_by_id(pool_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储池不存在"))?;

        Ok(StoragePoolResponse::from(pool))
    }

    /// 更新存储池
    pub async fn update_storage_pool(&self, pool_id: &str, dto: UpdateStoragePoolDto) -> anyhow::Result<StoragePoolResponse> {
        let db = &self.state.sea_db();

        let pool = StoragePoolEntity::find_by_id(pool_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储池不存在"))?;

        let mut pool_active: StoragePoolActiveModel = pool.into();

        if let Some(name) = dto.name {
            pool_active.name = Set(name);
        }
        if let Some(status) = dto.status {
            pool_active.status = Set(status);
        }
        if let Some(config) = dto.config {
            pool_active.config = Set(config);
        }
        if let Some(capacity_gb) = dto.capacity_gb {
            pool_active.capacity_gb = Set(Some(capacity_gb));
        }
        if let Some(allocated_gb) = dto.allocated_gb {
            pool_active.allocated_gb = Set(Some(allocated_gb));
        }
        if let Some(available_gb) = dto.available_gb {
            pool_active.available_gb = Set(Some(available_gb));
        }
        if let Some(metadata) = dto.metadata {
            pool_active.metadata = Set(Some(metadata));
        }
        pool_active.updated_at = Set(Utc::now().into());

        let updated_pool = pool_active.update(db).await?;
        Ok(StoragePoolResponse::from(updated_pool))
    }

    /// 删除存储池
    pub async fn delete_storage_pool(&self, pool_id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        // 检查是否有存储卷在使用此存储池
        let volume_count = VolumeEntity::find()
            .filter(VolumeColumn::PoolId.eq(pool_id))
            .count(db)
            .await?;

        if volume_count > 0 {
            return Err(anyhow::anyhow!("存储池下还有存储卷，无法删除"));
        }

        StoragePoolEntity::delete_by_id(pool_id)
            .exec(db)
            .await?;

        Ok(())
    }

    /// 创建存储卷
    pub async fn create_volume(&self, dto: CreateVolumeDto) -> anyhow::Result<VolumeResponse> {
        let db = &self.state.sea_db();

        // 检查存储池是否存在
        let pool = StoragePoolEntity::find_by_id(&dto.pool_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储池不存在"))?;

        let volume_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // 构建metadata，包含source信息
        let mut metadata = dto.metadata.unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(source) = &dto.source {
            if let Some(metadata_obj) = metadata.as_object_mut() {
                metadata_obj.insert("source".to_string(), serde_json::Value::String(source.clone()));
            }
        }

        // 先在数据库中创建记录
        let volume_active = VolumeActiveModel {
            id: Set(volume_id.clone()),
            name: Set(dto.name.clone()),
            volume_type: Set(dto.volume_type.clone()),
            size_gb: Set(dto.size_gb),
            pool_id: Set(dto.pool_id.clone()),
            path: Set(None),
            status: Set(VolumeStatus::Creating.as_str().to_string()),
            node_id: Set(dto.node_id.clone()),
            vm_id: Set(None),
            metadata: Set(Some(metadata)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let mut volume = volume_active.insert(db).await?;

        // 调用 Agent 创建实际的存储卷
        if let Some(node_id) = &dto.node_id {
            let request = CreateVolumeRequest {
                volume_id: volume_id.clone(),
                name: dto.name.clone(),
                size_gb: dto.size_gb as u64,
                storage_type: pool.pool_type.clone(),
                format: dto.volume_type.clone(),
                pool_id: pool.id.clone(),  // Agent会自动获取存储池信息
                source: dto.source.clone(),  // 传递外部URL
            };
            
            // 使用 WebSocket RPC 调用 Agent 创建存储卷
            let node_id = volume.node_id.as_ref()
                .ok_or_else(|| anyhow::anyhow!("存储卷未关联节点"))?;
            
            let response_msg = self.state.agent_manager()
                .call(
                    node_id,
                    "create_volume",
                    serde_json::to_value(&request)?,
                    Duration::from_secs(120)  // 存储卷创建可能需要较长时间
                )
                .await
                .map_err(|e| anyhow::anyhow!("WebSocket RPC 调用失败: {}", e))?;

            let result: CreateVolumeResponse = serde_json::from_value(
                response_msg.payload.ok_or_else(|| anyhow::anyhow!("响应无数据"))?
            )?;

            if !result.success {
                return Err(anyhow::anyhow!("Agent 创建存储卷失败: {}", result.message));
            }

            // 更新卷状态和路径
            let mut volume_active: VolumeActiveModel = volume.into();
            volume_active.status = Set(VolumeStatus::Available.as_str().to_string());
            if let Some(path) = result.path {
                volume_active.path = Set(Some(path));
            }
            volume_active.updated_at = Set(Utc::now().into());
            volume = volume_active.update(db).await?;
        }

        Ok(VolumeResponse::from(volume))
    }

    /// 获取存储卷列表
    pub async fn list_volumes(
        &self,
        page: usize,
        page_size: usize,
        pool_id: Option<String>,
        node_id: Option<String>,
        status: Option<String>,
    ) -> anyhow::Result<VolumeListResponse> {
        let db = &self.state.sea_db();

        let mut query = VolumeEntity::find();

        if let Some(pid) = pool_id {
            query = query.filter(VolumeColumn::PoolId.eq(pid));
        }

        if let Some(nid) = node_id {
            query = query.filter(VolumeColumn::NodeId.eq(nid));
        }

        if let Some(s) = status {
            query = query.filter(VolumeColumn::Status.eq(s));
        }

        let total = query.clone().count(db).await? as usize;

        let volumes = query
            .order_by_desc(VolumeColumn::CreatedAt)
            .offset(((page - 1) * page_size) as u64)
            .limit(page_size as u64)
            .all(db)
            .await?;

        // 获取所有相关的存储池和虚拟机信息
        let mut volume_responses = Vec::new();
        
        for volume in volumes {
            let mut volume_response = VolumeResponse::from(volume.clone());
            
            // 获取存储池名称
            if let Ok(pool) = StoragePoolEntity::find_by_id(&volume.pool_id).one(db).await {
                if let Some(pool) = pool {
                    volume_response.pool_name = Some(pool.name);
                }
            }
            
            // 获取虚拟机名称
            if let Some(vm_id) = &volume.vm_id {
                if let Ok(vm) = VmEntity::find_by_id(vm_id).one(db).await {
                    if let Some(vm) = vm {
                        volume_response.vm_name = Some(vm.name);
                    }
                }
            }
            
            volume_responses.push(volume_response);
        }

        Ok(VolumeListResponse {
            volumes: volume_responses,
            total,
            page,
            page_size,
        })
    }

    /// 获取单个存储卷
    pub async fn get_volume(&self, volume_id: &str) -> anyhow::Result<VolumeResponse> {
        let db = &self.state.sea_db();

        let volume = VolumeEntity::find_by_id(volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储卷不存在"))?;

        Ok(VolumeResponse::from(volume))
    }

    /// 更新存储卷
    pub async fn update_volume(&self, volume_id: &str, dto: UpdateVolumeDto) -> anyhow::Result<VolumeResponse> {
        let db = &self.state.sea_db();

        let volume = VolumeEntity::find_by_id(volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储卷不存在"))?;

        let mut volume_active: VolumeActiveModel = volume.into();

        if let Some(name) = dto.name {
            volume_active.name = Set(name);
        }
        if let Some(status) = dto.status {
            volume_active.status = Set(status);
        }
        if let Some(path) = dto.path {
            volume_active.path = Set(Some(path));
        }
        if let Some(node_id) = dto.node_id {
            volume_active.node_id = Set(Some(node_id));
        }
        if let Some(vm_id) = dto.vm_id {
            volume_active.vm_id = Set(Some(vm_id));
        }
        if let Some(metadata) = dto.metadata {
            volume_active.metadata = Set(Some(metadata));
        }
        volume_active.updated_at = Set(Utc::now().into());

        let updated_volume = volume_active.update(db).await?;
        Ok(VolumeResponse::from(updated_volume))
    }

    /// 调整存储卷大小
    pub async fn resize_volume(&self, volume_id: &str, dto: ResizeVolumeDto) -> anyhow::Result<VolumeResponse> {
        let db = &self.state.sea_db();

        let volume = VolumeEntity::find_by_id(volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储卷不存在"))?;

        // 调用 Agent 调整存储卷大小
        if let Some(node_id) = &volume.node_id {
            let request = ResizeVolumeRequest {
                volume_id: volume_id.to_string(),
                new_size_gb: dto.new_size_gb as u64,
                pool_id: volume.pool_id.clone(),
            };
            // 使用 WebSocket RPC 调用 Agent 调整存储卷大小
            let node_id = volume.node_id.as_ref()
                .ok_or_else(|| anyhow::anyhow!("存储卷未关联节点"))?;
            
            let response_msg = self.state.agent_manager()
                .call(
                    node_id,
                    "resize_volume",
                    serde_json::to_value(&request)?,
                    Duration::from_secs(60)
                )
                .await
                .map_err(|e| anyhow::anyhow!("WebSocket RPC 调用失败: {}", e))?;

            let result: ResizeVolumeResponse = serde_json::from_value(
                response_msg.payload.ok_or_else(|| anyhow::anyhow!("响应无数据"))?
            )?;

            if !result.success {
                return Err(anyhow::anyhow!("Agent 调整存储卷大小失败: {}", result.message));
            }
        }

        // 更新数据库中的大小
        let mut volume_active: VolumeActiveModel = volume.into();
        volume_active.size_gb = Set(dto.new_size_gb);
        volume_active.updated_at = Set(Utc::now().into());

        let updated_volume = volume_active.update(db).await?;
        Ok(VolumeResponse::from(updated_volume))
    }

    /// 删除存储卷
    pub async fn delete_volume(&self, volume_id: &str) -> anyhow::Result<()> {
        let db = &self.state.sea_db();

        let volume = VolumeEntity::find_by_id(volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储卷不存在"))?;

        // 检查是否正在被虚拟机使用
        if volume.vm_id.is_some() {
            return Err(anyhow::anyhow!("存储卷正在被虚拟机使用，无法删除"));
        }

        // 调用 Agent 删除实际的存储卷
        if let Some(node_id) = &volume.node_id {
            let request = DeleteVolumeRequest {
                volume_id: volume_id.to_string(),
                pool_id: volume.pool_id.clone(),
            };
            
            // 使用 WebSocket RPC 调用 Agent 删除存储卷
            let node_id = volume.node_id.as_ref()
                .ok_or_else(|| anyhow::anyhow!("存储卷未关联节点"))?;
            
            let response_msg = self.state.agent_manager()
                .call(
                    node_id,
                    "delete_volume",
                    serde_json::to_value(&request)?,
                    Duration::from_secs(60)
                )
                .await
                .map_err(|e| anyhow::anyhow!("WebSocket RPC 调用失败: {}", e))?;

            let result: DeleteVolumeResponse = serde_json::from_value(
                response_msg.payload.ok_or_else(|| anyhow::anyhow!("响应无数据"))?
            )?;

            if !result.success {
                warn!("Agent 删除存储卷失败: {}，将继续删除数据库记录", result.message);
            }
        }

        // 从数据库中删除
        VolumeEntity::delete_by_id(volume_id)
            .exec(db)
            .await?;

        Ok(())
    }

    /// 创建快照
    pub async fn create_snapshot(&self, volume_id: &str, snapshot_name: String) -> anyhow::Result<String> {
        let db = &self.state.sea_db();

        let volume = VolumeEntity::find_by_id(volume_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("存储卷不存在"))?;

        // 调用 Agent 创建快照
        if let Some(node_id) = &volume.node_id {
            let request = SnapshotVolumeRequest {
                volume_id: volume_id.to_string(),
                snapshot_name: snapshot_name.clone(),
                pool_id: volume.pool_id.clone(),
            };
            
            // 使用 WebSocket RPC 调用 Agent 创建快照
            let node_id = volume.node_id.as_ref()
                .ok_or_else(|| anyhow::anyhow!("存储卷未关联节点"))?;
            
            let response_msg = self.state.agent_manager()
                .call(
                    node_id,
                    "snapshot_volume",
                    serde_json::to_value(&request)?,
                    Duration::from_secs(120)  // 快照创建可能需要较长时间
                )
                .await
                .map_err(|e| anyhow::anyhow!("WebSocket RPC 调用失败: {}", e))?;

            let result: common::ws_rpc::SnapshotVolumeResponse = serde_json::from_value(
                response_msg.payload.ok_or_else(|| anyhow::anyhow!("响应无数据"))?
            )?;

            if !result.success {
                return Err(anyhow::anyhow!("Agent 创建快照失败: {}", result.message));
            }

            return Ok(result.snapshot_id.unwrap_or_else(|| format!("{}-{}", volume_id, snapshot_name)));
        }

        // 临时返回
        Ok(format!("{}-{}", volume_id, snapshot_name))
    }
}


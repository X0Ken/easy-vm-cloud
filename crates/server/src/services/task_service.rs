/// 任务管理服务

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::info;

use crate::db::models::task::{Entity as TaskEntity, Column as TaskColumn, ActiveModel as TaskActiveModel};
use crate::db::models::vm::{Entity as VmEntity, ActiveModel as VmActiveModel};
use crate::app_state::AppState;
use crate::ws::FrontendMessage;

pub struct TaskService {
    state: AppState,
}

impl TaskService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 更新任务状态
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: &str,
        progress: Option<i32>,
        result: Option<serde_json::Value>,
        error_message: Option<String>,
    ) -> anyhow::Result<()> {
        let db = &self.state.sea_db();
        
        // 查找任务
        let task = TaskEntity::find_by_id(task_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("任务不存在: {}", task_id))?;

        // 更新任务状态
        let mut task_active: TaskActiveModel = task.into();
        task_active.status = Set(status.to_string());
        task_active.updated_at = Set(Utc::now().into());
        
        if let Some(progress) = progress {
            task_active.progress = Set(progress);
        }
        
        if let Some(result) = result {
            task_active.result = Set(Some(result));
        }
        
        if let Some(error_message) = error_message {
            task_active.error_message = Set(Some(error_message));
        }

        // 设置完成时间
        if status == "completed" || status == "failed" {
            task_active.completed_at = Set(Some(Utc::now().into()));
        }

        task_active.update(db).await?;
        
        info!("任务状态已更新: task_id={}, status={}", task_id, status);
        Ok(())
    }

    /// 根据虚拟机ID和操作类型查找任务
    pub async fn find_task_by_vm_operation(
        &self,
        vm_id: &str,
        operation: &str,
    ) -> anyhow::Result<Option<String>> {
        let db = &self.state.sea_db();
        
        let task = TaskEntity::find()
            .filter(TaskColumn::TargetId.eq(vm_id))
            .filter(TaskColumn::TaskType.eq(operation))
            .filter(TaskColumn::Status.ne("completed"))
            .filter(TaskColumn::Status.ne("failed"))
            .one(db)
            .await?;

        Ok(task.map(|t| t.id))
    }

    /// 更新虚拟机状态
    pub async fn update_vm_status(
        &self,
        vm_id: &str,
        status: &str,
    ) -> anyhow::Result<()> {
        let db = &self.state.sea_db();
        
        // 查找虚拟机
        let vm = VmEntity::find_by_id(vm_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("虚拟机不存在: {}", vm_id))?;

        // 更新虚拟机状态
        let mut vm_active: VmActiveModel = vm.into();
        vm_active.status = Set(status.to_string());
        vm_active.updated_at = Set(Utc::now().into());
        
        // 根据状态设置相应的时间戳
        match status {
            "stopped" => {
                vm_active.stopped_at = Set(Some(Utc::now().into()));
            }
            "running" => {
                vm_active.started_at = Set(Some(Utc::now().into()));
            }
            _ => {}
        }

        vm_active.update(db).await?;
        
        info!("虚拟机状态已更新: vm_id={}, status={}", vm_id, status);
        
        // 发送状态更新通知给前端
        let frontend_msg = FrontendMessage::VmStatusUpdate {
            vm_id: vm_id.to_string(),
            status: status.to_string(),
            message: Some(format!("虚拟机状态已更新为: {}", status)),
        };
        
        let count = self.state.frontend_manager().broadcast(frontend_msg).await;
        if count > 0 {
            info!("已向 {} 个前端连接发送 VM {} 状态更新: {}", count, vm_id, status);
        }
        
        Ok(())
    }

    /// 处理虚拟机操作完成通知
    pub async fn handle_vm_operation_completed(
        &self,
        vm_id: &str,
        operation: &str,
        success: bool,
        message: &str,
    ) -> anyhow::Result<()> {
        // 查找对应的任务
        if let Some(task_id) = self.find_task_by_vm_operation(vm_id, operation).await? {
            // 更新任务状态
            let status = if success { "completed" } else { "failed" };
            let result = serde_json::json!({
                "success": success,
                "message": message
            });
            
            self.update_task_status(
                &task_id,
                status,
                Some(100),
                Some(result),
                if success { None } else { Some(message.to_string()) },
            ).await?;
        }

        // 更新虚拟机状态
        let vm_status = match (operation, success) {
            ("stop_vm", true) => "stopped",
            ("stop_vm", false) => "error",
            ("start_vm", true) => "running",
            ("start_vm", false) => "error",
            ("restart_vm", true) => "running",
            ("restart_vm", false) => "error",
            _ => "error",
        };

        self.update_vm_status(vm_id, vm_status).await?;
        
        Ok(())
    }

    /// 通过 task_id 处理虚拟机操作完成通知
    pub async fn handle_vm_operation_completed_by_task_id(
        &self,
        task_id: &str,
        vm_id: &str,
        operation: &str,
        success: bool,
        message: &str,
    ) -> anyhow::Result<()> {
        // 更新任务状态
        let status = if success { "completed" } else { "failed" };
        let result = serde_json::json!({
            "success": success,
            "message": message
        });
        
        self.update_task_status(
            task_id,
            status,
            Some(100),
            Some(result),
            if success { None } else { Some(message.to_string()) },
        ).await?;

        // 更新虚拟机状态
        let vm_status = match (operation, success) {
            ("stop_vm", true) => "stopped",
            ("stop_vm", false) => "error",
            ("start_vm", true) => "running",
            ("start_vm", false) => "error",
            _ => "error",
        };

        self.update_vm_status(vm_id, vm_status).await?;
        
        Ok(())
    }
}

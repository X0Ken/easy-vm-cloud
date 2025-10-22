import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { ActivatedRoute } from '@angular/router';
import {
  FormsModule,
  ReactiveFormsModule,
  FormBuilder,
  FormGroup,
  Validators,
} from '@angular/forms';
import { NzTableModule } from 'ng-zorro-antd/table';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzModalModule, NzModalService } from 'ng-zorro-antd/modal';
import { NzMessageService } from 'ng-zorro-antd/message';
import { NzFormModule } from 'ng-zorro-antd/form';
import { NzInputModule } from 'ng-zorro-antd/input';
import { NzSelectModule } from 'ng-zorro-antd/select';
import { NzTagModule } from 'ng-zorro-antd/tag';
import { NzPopconfirmModule } from 'ng-zorro-antd/popconfirm';
import { NzToolTipModule } from 'ng-zorro-antd/tooltip';
import { NzSpaceModule } from 'ng-zorro-antd/space';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzIconModule } from 'ng-zorro-antd/icon';
import { NzDividerModule } from 'ng-zorro-antd/divider';
import { NzDropDownModule } from 'ng-zorro-antd/dropdown';
import { NzSpinModule } from 'ng-zorro-antd/spin';

import {
  SnapshotService,
  SnapshotResponse,
  CreateSnapshotDto,
  UpdateSnapshotDto,
} from '../../../services/snapshot.service';
import {
  StorageService,
  StorageVolume,
  PaginatedResponse,
} from '../../../services/storage.service';
import { WebSocketService } from '../../../services/websocket.service';

/**
 * 快照管理组件
 * 提供快照的列表展示、创建、删除、恢复等功能
 */
@Component({
  selector: 'app-snapshots',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    NzTableModule,
    NzButtonModule,
    NzModalModule,
    NzFormModule,
    NzInputModule,
    NzSelectModule,
    NzTagModule,
    NzPopconfirmModule,
    NzToolTipModule,
    NzSpaceModule,
    NzCardModule,
    NzIconModule,
    NzDividerModule,
    NzDropDownModule,
    NzSpinModule,
  ],
  templateUrl: './snapshots.component.html',
  styleUrls: ['./snapshots.component.scss'],
})
export class SnapshotsComponent implements OnInit, OnDestroy {
  // 销毁信号
  private destroy$ = new Subject<void>();

  // 数据列表
  snapshots: SnapshotResponse[] = [];
  volumes: StorageVolume[] = [];

  // 加载状态
  loading = false;
  loadingVolumes = false;

  // 分页
  total = 0;
  pageIndex = 1;
  pageSize = 10;

  // 筛选条件
  filterVolumeId: string | null = null;
  filterStatus: string | null = null;

  // 创建模态框
  isCreateModalVisible = false;
  createForm!: FormGroup;
  createLoading = false;

  // 编辑模态框
  isEditModalVisible = false;
  editForm!: FormGroup;
  editLoading = false;
  currentEditSnapshot: SnapshotResponse | null = null;

  // 详情模态框
  isDetailModalVisible = false;
  selectedSnapshot: SnapshotResponse | null = null;
  snapshotDetailsLoading = false;

  // 状态映射
  statusMap: { [key: string]: { color: string; text: string } } = {
    creating: { color: 'processing', text: '创建中' },
    available: { color: 'success', text: '可用' },
    deleting: { color: 'warning', text: '删除中' },
    error: { color: 'error', text: '错误' },
  };

  constructor(
    private snapshotService: SnapshotService,
    private storageService: StorageService,
    private fb: FormBuilder,
    private modal: NzModalService,
    private message: NzMessageService,
    private route: ActivatedRoute,
    private websocketService: WebSocketService,
  ) {
    this.initForm();
  }

  ngOnInit(): void {
    // 从URL查询参数中读取volume_id
    this.route.queryParams.subscribe((params) => {
      if (params['volume_id']) {
        this.filterVolumeId = params['volume_id'];
      }
    });

    this.loadSnapshots();
    this.loadVolumes();
    this.setupWebSocketListeners();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * 设置 WebSocket 监听器
   */
  setupWebSocketListeners(): void {
    // 监听快照状态更新
    this.websocketService.snapshotStatusUpdates$
      .pipe(takeUntil(this.destroy$))
      .subscribe((update) => {
        console.log('收到快照状态更新:', update);
        this.handleSnapshotStatusUpdate(update);
      });

    // 监听系统通知
    this.websocketService.systemNotifications$
      .pipe(takeUntil(this.destroy$))
      .subscribe((notification) => {
        console.log('收到系统通知:', notification);
        this.handleSystemNotification(notification);
      });
  }

  /**
   * 处理快照状态更新
   */
  handleSnapshotStatusUpdate(update: {
    snapshot_id: string;
    status: string;
    message?: string;
  }): void {
    // 更新本地快照列表中的状态
    const snapshot = this.snapshots.find((s) => s.id === update.snapshot_id);
    if (snapshot) {
      const oldStatus = snapshot.status;
      snapshot.status = update.status;
      console.log(`快照 ${snapshot.name} 状态已更新为: ${update.status}`);

      // 根据状态变化显示相应的消息
      if (update.status === 'available' && oldStatus !== 'available') {
        this.message.success(`快照 ${snapshot.name} 已创建完成`);
      } else if (update.status === 'error') {
        this.message.error(
          `快照 ${snapshot.name} 操作失败${update.message ? ': ' + update.message : ''}`,
        );
      } else if (update.message) {
        this.message.info(`快照 ${snapshot.name}: ${update.message}`);
      }
    } else {
      // 如果快照不在当前列表中（可能是新创建的），刷新列表
      if (update.status === 'available' || update.status === 'error') {
        this.loadSnapshots();
      }
    }
  }

  /**
   * 处理系统通知
   */
  handleSystemNotification(notification: { title: string; message: string; level: string }): void {
    switch (notification.level) {
      case 'error':
        this.message.error(`${notification.title}: ${notification.message}`);
        break;
      case 'warning':
        this.message.warning(`${notification.title}: ${notification.message}`);
        break;
      default:
        this.message.info(`${notification.title}: ${notification.message}`);
        break;
    }
  }

  /**
   * 初始化创建表单
   */
  initForm(): void {
    this.createForm = this.fb.group({
      name: ['', [Validators.required]],
      volume_id: [null, [Validators.required]],
      description: [''],
    });

    this.editForm = this.fb.group({
      name: ['', [Validators.required]],
      description: [''],
    });
  }

  /**
   * 加载快照列表
   */
  loadSnapshots(): void {
    this.loading = true;
    const params: any = {
      page: this.pageIndex,
      page_size: this.pageSize,
    };

    if (this.filterVolumeId) {
      params.volume_id = this.filterVolumeId;
    }
    if (this.filterStatus) {
      params.status = this.filterStatus;
    }

    this.snapshotService.listSnapshots(params).subscribe({
      next: (response) => {
        this.snapshots = response.snapshots;
        this.total = response.total;
        this.loading = false;
      },
      error: (error) => {
        console.error('加载快照列表失败:', error);
        this.message.error('加载快照列表失败');
        this.loading = false;
      },
    });
  }

  /**
   * 加载存储卷列表
   */
  loadVolumes(): void {
    this.loadingVolumes = true;
    this.storageService.getStorageVolumes(1, 1000).subscribe({
      next: (response: PaginatedResponse<StorageVolume>) => {
        this.volumes = response.data;
        this.loadingVolumes = false;
      },
      error: (error: any) => {
        console.error('加载存储卷列表失败:', error);
        this.message.error('加载存储卷列表失败');
        this.loadingVolumes = false;
      },
    });
  }

  /**
   * 页码改变
   */
  onPageIndexChange(pageIndex: number): void {
    this.pageIndex = pageIndex;
    this.loadSnapshots();
  }

  /**
   * 每页条数改变
   */
  onPageSizeChange(pageSize: number): void {
    this.pageSize = pageSize;
    this.pageIndex = 1;
    this.loadSnapshots();
  }

  /**
   * 应用筛选
   */
  applyFilter(): void {
    this.pageIndex = 1;
    this.loadSnapshots();
  }

  /**
   * 重置筛选
   */
  resetFilter(): void {
    this.filterVolumeId = null;
    this.filterStatus = null;
    this.pageIndex = 1;
    this.loadSnapshots();
  }

  /**
   * 显示创建快照模态框
   */
  showCreateModal(): void {
    this.createForm.reset();
    this.isCreateModalVisible = true;
  }

  /**
   * 处理创建快照
   */
  handleCreateSnapshot(): void {
    if (this.createForm.valid) {
      this.createLoading = true;
      const dto: CreateSnapshotDto = this.createForm.value;

      this.snapshotService.createSnapshot(dto).subscribe({
        next: () => {
          this.message.success('快照创建中');
          this.isCreateModalVisible = false;
          this.createForm.reset();
          this.createLoading = false;
          this.loadSnapshots();
        },
        error: (error) => {
          console.error('创建快照失败:', error);
          this.message.error(error.error?.message || '创建快照失败');
          this.createLoading = false;
        },
      });
    } else {
      Object.values(this.createForm.controls).forEach((control) => {
        if (control.invalid) {
          control.markAsDirty();
          control.updateValueAndValidity({ onlySelf: true });
        }
      });
    }
  }

  /**
   * 取消创建
   */
  handleCancelCreate(): void {
    this.isCreateModalVisible = false;
    this.createForm.reset();
  }

  /**
   * 删除快照
   */
  deleteSnapshot(snapshot: SnapshotResponse): void {
    this.snapshotService.deleteSnapshot(snapshot.id).subscribe({
      next: () => {
        this.message.success('快照删除中');
        this.loadSnapshots();
      },
      error: (error) => {
        console.error('删除快照失败:', error);
        this.message.error(error.error?.message || '删除快照失败');
      },
    });
  }

  /**
   * 恢复快照
   */
  restoreSnapshot(snapshot: SnapshotResponse): void {
    this.modal.confirm({
      nzTitle: '确认恢复快照',
      nzContent: `确定要将存储卷 "${snapshot.volume_name}" 恢复到快照 "${snapshot.name}" 的状态吗？此操作将覆盖当前数据，请确保虚拟机已停止。`,
      nzOkText: '确认',
      nzOkType: 'primary',
      nzOkDanger: true,
      nzCancelText: '取消',
      nzOnOk: () => {
        return new Promise((resolve, reject) => {
          this.snapshotService.restoreSnapshot(snapshot.id).subscribe({
            next: () => {
              this.message.success('快照恢复中');
              this.loadSnapshots();
              resolve(true);
            },
            error: (error) => {
              console.error('恢复快照失败:', error);
              this.message.error(error.error?.message || '恢复快照失败');
              reject(error);
            },
          });
        });
      },
    });
  }

  /**
   * 显示编辑快照模态框
   */
  showEditModal(snapshot: SnapshotResponse): void {
    this.currentEditSnapshot = snapshot;
    this.editForm.patchValue({
      name: snapshot.name,
      description: snapshot.description || '',
    });
    this.isEditModalVisible = true;
  }

  /**
   * 处理编辑快照
   */
  handleEditSnapshot(): void {
    if (this.editForm.valid && this.currentEditSnapshot) {
      this.editLoading = true;
      const dto: UpdateSnapshotDto = this.editForm.value;

      this.snapshotService.updateSnapshot(this.currentEditSnapshot.id, dto).subscribe({
        next: (response) => {
          this.message.success('快照更新成功');
          this.isEditModalVisible = false;
          this.editForm.reset();
          this.currentEditSnapshot = null;
          this.editLoading = false;
          this.loadSnapshots();
        },
        error: (error) => {
          console.error('更新快照失败:', error);
          this.message.error(error.error?.message || '更新快照失败');
          this.editLoading = false;
        },
      });
    } else {
      Object.values(this.editForm.controls).forEach((control) => {
        if (control.invalid) {
          control.markAsDirty();
          control.updateValueAndValidity({ onlySelf: true });
        }
      });
    }
  }

  /**
   * 取消编辑
   */
  handleCancelEdit(): void {
    this.isEditModalVisible = false;
    this.editForm.reset();
    this.currentEditSnapshot = null;
  }

  /**
   * 显示详情模态框
   */
  showDetailModal(snapshot: SnapshotResponse): void {
    this.selectedSnapshot = snapshot;
    this.isDetailModalVisible = true;
    this.loadSnapshotDetails(snapshot.id);
  }

  /**
   * 关闭详情模态框
   */
  handleDetailCancel(): void {
    this.isDetailModalVisible = false;
    this.selectedSnapshot = null;
  }

  /**
   * 加载快照详情
   */
  loadSnapshotDetails(snapshotId: string): void {
    this.snapshotDetailsLoading = true;
    // 获取快照详细信息
    this.snapshotService.getSnapshot(snapshotId).subscribe({
      next: (snapshot) => {
        this.selectedSnapshot = snapshot;
        this.snapshotDetailsLoading = false;
      },
      error: (error) => {
        console.error('加载快照详情失败:', error);
        this.message.error('加载快照详情失败');
        this.snapshotDetailsLoading = false;
      },
    });
  }

  /**
   * 刷新列表
   */
  refresh(): void {
    this.loadSnapshots();
  }

  /**
   * 获取状态标签颜色
   */
  getStatusColor(status: string): string {
    return this.statusMap[status]?.color || 'default';
  }

  /**
   * 获取状态文本
   */
  getStatusText(status: string): string {
    return this.statusMap[status]?.text || status;
  }

  /**
   * 格式化日期
   */
  formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleString('zh-CN');
  }

  /**
   * 格式化大小
   */
  formatSize(sizeGb?: number): string {
    if (!sizeGb) {
      return '-';
    }
    return `${sizeGb} GB`;
  }

  /**
   * 判断是否可以恢复
   */
  canRestore(snapshot: SnapshotResponse): boolean {
    return snapshot.status === 'available';
  }

  /**
   * 判断是否可以删除
   */
  canDelete(snapshot: SnapshotResponse): boolean {
    return snapshot.status === 'available' || snapshot.status === 'error';
  }
}

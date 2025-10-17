import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NzTableModule } from 'ng-zorro-antd/table';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzTagModule } from 'ng-zorro-antd/tag';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzIconModule } from 'ng-zorro-antd/icon';
import { NzMessageService } from 'ng-zorro-antd/message';
import { NzSpinModule } from 'ng-zorro-antd/spin';
import { NzModalModule } from 'ng-zorro-antd/modal';
import { NzFormModule } from 'ng-zorro-antd/form';
import { NzInputModule } from 'ng-zorro-antd/input';
import { NzSelectModule } from 'ng-zorro-antd/select';
import { NzInputNumberModule } from 'ng-zorro-antd/input-number';
import { NzPopconfirmModule } from 'ng-zorro-antd/popconfirm';
import { FormsModule } from '@angular/forms';
import { StorageService, StoragePool, CreateStoragePoolRequest, UpdateStoragePoolRequest, PaginatedResponse } from '../../../services/storage.service';

@Component({
  selector: 'app-storage-pools',
  standalone: true,
  imports: [
    CommonModule,
    NzTableModule,
    NzCardModule,
    NzTagModule,
    NzButtonModule,
    NzIconModule,
    NzSpinModule,
    NzModalModule,
    NzFormModule,
    NzInputModule,
    NzSelectModule,
    NzInputNumberModule,
    NzPopconfirmModule,
    FormsModule
  ],
  templateUrl: './storage-pools.component.html',
  styleUrls: ['./storage-pools.component.scss']
})
export class StoragePoolsComponent implements OnInit {
  storagePools: StoragePool[] = [];
  loading = false;
  isModalVisible = false;
  isEditMode = false;
  currentPool: StoragePool | null = null;
  
  // 分页状态
  pagination = {
    current_page: 1,
    per_page: 20,
    total: 0,
    total_pages: 0,
    has_next: false,
    has_prev: false
  };
  
  // 存储池表单数据
  poolFormData = {
    name: '',
    pool_type: 'nfs' as 'lvm' | 'nfs' | 'ceph' | 'iscsi', // 默认选择NFS
    capacity_gb: 100,
    config: {} as any
  };

  constructor(
    private storageService: StorageService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadStoragePools();
  }

  loadStoragePools(page: number = 1): void {
    this.loading = true;
    this.storageService.getStoragePools(page, this.pagination.per_page).subscribe({
      next: (response: PaginatedResponse<StoragePool>) => {
        this.storagePools = response.data;
        // 安全地更新分页信息，确保所有必要的属性都存在
        this.pagination = {
          current_page: response.pagination?.current_page || page,
          per_page: response.pagination?.per_page || this.pagination.per_page,
          total: response.pagination?.total || 0,
          total_pages: response.pagination?.total_pages || 0,
          has_next: response.pagination?.has_next || false,
          has_prev: response.pagination?.has_prev || false
        };
        this.loading = false;
      },
      error: (error) => {
        console.error('获取存储池列表失败:', error);
        this.message.error('获取存储池列表失败');
        this.loading = false;
      }
    });
  }

  onPageIndexChange(page: number): void {
    this.loadStoragePools(page);
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadStoragePools(this.pagination.current_page);
  }

  getStatusColor(status: string): string {
    const statusColors: { [key: string]: string } = {
      'active': 'green',
      'inactive': 'red',
      'error': 'red',
      'available': 'green',
      'in_use': 'blue',
      'in-use': 'blue',
      'creating': 'orange',
      'deleting': 'red'
    };
    return statusColors[status] || 'default';
  }

  getStatusText(status: string): string {
    const statusTexts: { [key: string]: string } = {
      'active': '活跃',
      'inactive': '非活跃',
      'error': '错误',
      'available': '可用',
      'in_use': '使用中',
      'in-use': '使用中',
      'creating': '创建中',
      'deleting': '删除中'
    };
    return statusTexts[status] || status;
  }

  getTypeText(type: string): string {
    const typeTexts: { [key: string]: string } = {
      'lvm': 'LVM',
      'nfs': 'NFS',
      'ceph': 'Ceph',
      'iscsi': 'iSCSI'
    };
    return typeTexts[type] || type;
  }

  getTypeColor(type: string): string {
    const typeColors: { [key: string]: string } = {
      'lvm': 'blue',
      'nfs': 'green',
      'ceph': 'purple',
      'iscsi': 'orange'
    };
    return typeColors[type] || 'default';
  }

  showCreatePoolModal(): void {
    this.isEditMode = false;
    this.currentPool = null;
    this.resetPoolForm();
    this.isModalVisible = true;
  }

  showEditPoolModal(pool: StoragePool): void {
    this.isEditMode = true;
    this.currentPool = pool;
    this.poolFormData = {
      name: pool.name,
      pool_type: pool.type,
      capacity_gb: pool.total_size_gb,
      config: pool.config || {}
    };
    this.isModalVisible = true;
  }

  handleOk(): void {
    if (this.isEditMode && this.currentPool) {
      this.updatePool();
    } else {
      this.createPool();
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetPoolForm();
  }

  createPool(): void {
    const createData: CreateStoragePoolRequest = {
      name: this.poolFormData.name,
      pool_type: this.poolFormData.pool_type,
      capacity_gb: this.poolFormData.capacity_gb,
      config: this.poolFormData.config
    };

    this.storageService.createStoragePool(createData).subscribe({
      next: (response) => {
        this.message.success('存储池创建成功');
        this.isModalVisible = false;
        this.resetPoolForm();
        this.loadStoragePools(this.pagination.current_page);
      },
      error: (error) => {
        console.error('创建存储池失败:', error);
        this.message.error('创建存储池失败');
      }
    });
  }

  updatePool(): void {
    if (!this.currentPool) return;
    
    const updateData: UpdateStoragePoolRequest = {
      name: this.poolFormData.name,
      total_size_gb: this.poolFormData.capacity_gb
    };

    this.storageService.updateStoragePool(this.currentPool.id, updateData).subscribe({
      next: (response) => {
        this.message.success('存储池更新成功');
        this.isModalVisible = false;
        this.resetPoolForm();
        this.loadStoragePools(this.pagination.current_page);
      },
      error: (error) => {
        console.error('更新存储池失败:', error);
        this.message.error('更新存储池失败');
      }
    });
  }

  deletePool(pool: StoragePool): void {
    this.storageService.deleteStoragePool(pool.id).subscribe({
      next: () => {
        this.message.success('存储池删除成功');
        this.loadStoragePools(this.pagination.current_page);
      },
      error: (error) => {
        console.error('删除存储池失败:', error);
        
        // 尝试从错误响应中提取具体的错误信息
        let errorMessage = '删除存储池失败';
        if (error.error && error.error.message) {
          errorMessage = error.error.message;
        } else if (error.message) {
          errorMessage = error.message;
        }
        
        this.message.error(errorMessage);
      }
    });
  }

  resetPoolForm(): void {
    this.poolFormData = {
      name: '',
      pool_type: 'nfs', // 默认选择NFS
      capacity_gb: 100,
      config: {}
    };
  }

  formatSize(sizeGb: number): string {
    if (sizeGb >= 1024) {
      return `${(sizeGb / 1024).toFixed(1)} TB`;
    }
    return `${sizeGb} GB`;
  }

  calculateUsagePercentage(used: number, total: number): number {
    return total > 0 ? Math.round((used / total) * 100) : 0;
  }
}

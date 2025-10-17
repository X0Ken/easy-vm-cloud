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
import { NzTabsModule } from 'ng-zorro-antd/tabs';
import { NzDescriptionsModule } from 'ng-zorro-antd/descriptions';
import { NzTooltipModule } from 'ng-zorro-antd/tooltip';
import { FormsModule } from '@angular/forms';
import { StorageService, StoragePool, StorageVolume, Node, CreateStoragePoolRequest, UpdateStoragePoolRequest, CreateStorageVolumeRequest, UpdateStorageVolumeRequest, PaginatedResponse } from '../../services/storage.service';

@Component({
  selector: 'app-storage',
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
    NzTabsModule,
    NzDescriptionsModule,
    NzTooltipModule,
    FormsModule
  ],
  templateUrl: './storage.component.html',
  styleUrls: ['./storage.component.scss']
})
export class StorageComponent implements OnInit {
  storagePools: StoragePool[] = [];
  storageVolumes: StorageVolume[] = [];
  nodes: Node[] = [];
  loading = false;
  isModalVisible = false;
  isDetailModalVisible = false;
  isEditMode = false;
  currentPool: StoragePool | null = null;
  currentVolume: StorageVolume | null = null;
  selectedVolume: StorageVolume | null = null;
  activeTab = 'pools';
  activeTabIndex = 0;
  
  // 加载状态标志
  poolsLoaded = false;
  volumesLoaded = false;
  nodesLoaded = false;
  
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
    type: 'nfs' as 'lvm' | 'nfs' | 'ceph' | 'iscsi', // 默认选择NFS
    total_size_gb: 100
  };

  // 存储卷表单数据
  volumeFormData = {
    name: '',
    pool_id: null as number | null,
    size_gb: 20,
    volume_type: 'qcow2' as 'qcow2' | 'raw',
    node_id: null as string | null,
    dataSource: 'blank' as 'blank' | 'url',  // 数据源选择
    source: null as string | null  // 外部URL
  };

  constructor(
    private storageService: StorageService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    // 初始化时加载默认tab（存储池）的数据
    this.loadDataForActiveTab();
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
        this.poolsLoaded = true;
      },
      error: (error) => {
        console.error('获取存储池列表失败:', error);
        this.message.error('获取存储池列表失败');
        this.loading = false;
      }
    });
  }

  loadStorageVolumes(page: number = 1): void {
    this.loading = true;
    this.storageService.getStorageVolumes(page, this.pagination.per_page).subscribe({
      next: (response: PaginatedResponse<StorageVolume>) => {
        this.storageVolumes = response.data;
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
        this.volumesLoaded = true;
      },
      error: (error) => {
        console.error('获取存储卷列表失败:', error);
        this.message.error('获取存储卷列表失败');
        this.loading = false;
      }
    });
  }

  loadNodes(): void {
    this.storageService.getNodes().subscribe({
      next: (nodes: Node[]) => {
        this.nodes = nodes;
        this.nodesLoaded = true;
      },
      error: (error) => {
        console.error('获取节点列表失败:', error);
        this.message.error('获取节点列表失败');
      }
    });
  }

  onTabChange(tabIndex: number): void {
    const tabs = ['pools', 'volumes'];
    this.activeTab = tabs[tabIndex];
    this.activeTabIndex = tabIndex;
    this.loadDataForActiveTab();
  }

  loadDataForActiveTab(): void {
    switch (this.activeTab) {
      case 'pools':
        if (!this.poolsLoaded) {
          this.loadStoragePools();
        }
        break;
      case 'volumes':
        if (!this.volumesLoaded) {
          this.loadStorageVolumes();
        }
        break;
    }
  }

  onPageIndexChange(page: number): void {
    this.loadDataForActiveTab();
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadDataForActiveTab();
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
      type: pool.type,
      total_size_gb: pool.total_size_gb
    };
    this.isModalVisible = true;
  }

  showCreateVolumeModal(): void {
    this.isEditMode = false;
    this.currentVolume = null;
    this.resetVolumeForm();
    
    // 在新建存储卷时加载node信息
    if (!this.nodesLoaded) {
      this.loadNodes();
    }
    
    this.isModalVisible = true;
  }

  showEditVolumeModal(volume: StorageVolume): void {
    this.isEditMode = true;
    this.currentVolume = volume;
    this.volumeFormData = {
      name: volume.name,
      pool_id: volume.pool_id,
      size_gb: volume.size_gb,
      volume_type: volume.volume_type || 'qcow2',
      node_id: volume.node_id || null,
      dataSource: 'blank' as 'blank' | 'url',  // 编辑时默认为空白
      source: null as string | null  // 编辑时不支持外部URL
    };
    this.isModalVisible = true;
  }

  handleOk(): void {
    if (this.activeTab === 'pools') {
      if (this.isEditMode && this.currentPool) {
        this.updatePool();
      } else {
        this.createPool();
      }
    } else {
      if (this.isEditMode && this.currentVolume) {
        this.updateVolume();
      } else {
        this.createVolume();
      }
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetPoolForm();
    this.resetVolumeForm();
  }

  createPool(): void {
    const createData: CreateStoragePoolRequest = {
      name: this.poolFormData.name,
      type: this.poolFormData.type,
      total_size_gb: this.poolFormData.total_size_gb
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
      total_size_gb: this.poolFormData.total_size_gb
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

  // 数据源变化处理
  onDataSourceChange(dataSource: 'blank' | 'url'): void {
    if (dataSource === 'blank') {
      this.volumeFormData.source = null;
    }
  }

  createVolume(): void {
    const createData: CreateStorageVolumeRequest = {
      name: this.volumeFormData.name,
      pool_id: this.volumeFormData.pool_id!,
      size_gb: this.volumeFormData.size_gb,
      volume_type: this.volumeFormData.volume_type,
      node_id: this.volumeFormData.node_id,
      source: this.volumeFormData.dataSource === 'url' ? this.volumeFormData.source : null
    };

    this.storageService.createStorageVolume(createData).subscribe({
      next: (response) => {
        this.message.success('存储卷创建成功');
        this.isModalVisible = false;
        this.resetVolumeForm();
        this.loadStorageVolumes(this.pagination.current_page);
      },
      error: (error) => {
        console.error('创建存储卷失败:', error);
        this.message.error('创建存储卷失败');
      }
    });
  }

  updateVolume(): void {
    if (!this.currentVolume) return;
    
    const updateData: UpdateStorageVolumeRequest = {
      name: this.volumeFormData.name,
      size_gb: this.volumeFormData.size_gb
    };

    this.storageService.updateStorageVolume(this.currentVolume.id, updateData).subscribe({
      next: (response) => {
        this.message.success('存储卷更新成功');
        this.isModalVisible = false;
        this.resetVolumeForm();
        this.loadStorageVolumes(this.pagination.current_page);
      },
      error: (error) => {
        console.error('更新存储卷失败:', error);
        this.message.error('更新存储卷失败');
      }
    });
  }

  deleteVolume(volume: StorageVolume): void {
    this.storageService.deleteStorageVolume(volume.id).subscribe({
      next: () => {
        this.message.success('存储卷删除成功');
        this.loadStorageVolumes(this.pagination.current_page);
      },
      error: (error) => {
        console.error('删除存储卷失败:', error);
        this.message.error('删除存储卷失败');
      }
    });
  }

  resetPoolForm(): void {
    this.poolFormData = {
      name: '',
      type: 'nfs', // 默认选择NFS
      total_size_gb: 100
    };
  }

  resetVolumeForm(): void {
    this.volumeFormData = {
      name: '',
      pool_id: null,
      size_gb: 20,
      volume_type: 'qcow2',
      node_id: null,
      dataSource: 'blank',
      source: null
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

  // 从metadata中获取source信息
  getSourceFromMetadata(metadata: any): string | null {
    if (!metadata || typeof metadata !== 'object') {
      return null;
    }
    return metadata.source || null;
  }

  // 显示存储卷详情
  showVolumeDetails(volume: StorageVolume): void {
    this.selectedVolume = volume;
    this.isDetailModalVisible = true;
  }

  // 关闭详情模态框
  handleDetailCancel(): void {
    this.isDetailModalVisible = false;
    this.selectedVolume = null;
  }
}

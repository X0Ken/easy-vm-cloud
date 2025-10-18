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
import { NzDescriptionsModule } from 'ng-zorro-antd/descriptions';
import { FormsModule } from '@angular/forms';
import { StorageService, StorageVolume, StoragePool, Node, CreateStorageVolumeRequest, UpdateStorageVolumeRequest, PaginatedResponse } from '../../../services/storage.service';

@Component({
  selector: 'app-storage-volumes',
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
    NzDescriptionsModule,
    FormsModule
  ],
  templateUrl: './storage-volumes.component.html',
  styleUrls: ['./storage-volumes.component.scss']
})
export class StorageVolumesComponent implements OnInit {
  storageVolumes: StorageVolume[] = [];
  storagePools: StoragePool[] = [];
  nodes: Node[] = [];
  loading = false;
  isModalVisible = false;
  isDetailModalVisible = false;
  isCloneModalVisible = false;
  isResizeModalVisible = false;
  isEditMode = false;
  currentVolume: StorageVolume | null = null;
  selectedVolume: StorageVolume | null = null;
  cloneSourceVolume: StorageVolume | null = null;
  resizeVolume: StorageVolume | null = null;
  
  // 加载状态标志
  volumesLoaded = false;
  poolsLoaded = false;
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
  
  // 存储卷表单数据
  volumeFormData = {
    name: '',
    pool_id: null as number | null,
    size_gb: 20,
    volume_type: 'qcow2' as 'qcow2' | 'raw',
    dataSource: 'blank' as 'blank' | 'url',  // 数据源选择
    source: null as string | null  // 外部URL
  };

  // 克隆表单数据
  cloneFormData = {
    targetName: ''
  };

  // 扩容表单数据
  resizeFormData = {
    newSizeGb: 0
  };

  constructor(
    private storageService: StorageService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadStorageVolumes();
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

  loadStoragePools(): void {
    this.storageService.getStoragePools(1, 1000).subscribe({
      next: (response: PaginatedResponse<StoragePool>) => {
        this.storagePools = response.data;
        this.poolsLoaded = true;
      },
      error: (error) => {
        console.error('获取存储池列表失败:', error);
        this.message.error('获取存储池列表失败');
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

  onPageIndexChange(page: number): void {
    this.loadStorageVolumes(page);
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadStorageVolumes(this.pagination.current_page);
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

  showCreateVolumeModal(): void {
    this.isEditMode = false;
    this.currentVolume = null;
    this.resetVolumeForm();
    
    // 在新建存储卷时加载相关数据
    if (!this.poolsLoaded) {
      this.loadStoragePools();
    }
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
      dataSource: 'blank' as 'blank' | 'url',  // 编辑时默认为空白
      source: null as string | null  // 编辑时不支持外部URL
    };
    this.isModalVisible = true;
  }

  handleOk(): void {
    if (this.isEditMode && this.currentVolume) {
      this.updateVolume();
    } else {
      this.createVolume();
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetVolumeForm();
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

  resetVolumeForm(): void {
    this.volumeFormData = {
      name: '',
      pool_id: null,
      size_gb: 20,
      volume_type: 'qcow2',
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

  // 显示克隆存储卷模态框
  showCloneVolumeModal(volume: StorageVolume): void {
    this.cloneSourceVolume = volume;
    this.cloneFormData = {
      targetName: `${volume.name}-clone`
    };
    this.isCloneModalVisible = true;
  }

  // 处理克隆确认
  handleCloneOk(): void {
    if (!this.cloneSourceVolume || !this.cloneFormData.targetName.trim()) {
      this.message.error('请输入新存储卷名称');
      return;
    }

    this.loading = true;
    this.storageService.cloneVolume(
      this.cloneSourceVolume.id,
      this.cloneFormData.targetName.trim()
    ).subscribe({
      next: (clonedVolume: StorageVolume) => {
        this.message.success('存储卷克隆成功');
        this.isCloneModalVisible = false;
        this.cloneSourceVolume = null;
        this.cloneFormData = { targetName: '' };
        this.loadStorageVolumes(this.pagination.current_page);
      },
      error: (error) => {
        this.loading = false;
        console.error('克隆存储卷失败:', error);
        this.message.error('克隆存储卷失败: ' + (error.error?.message || error.message || '未知错误'));
      }
    });
  }

  // 处理克隆取消
  handleCloneCancel(): void {
    this.isCloneModalVisible = false;
    this.cloneSourceVolume = null;
    this.cloneFormData = { targetName: '' };
  }

  // 显示扩容存储卷模态框
  showResizeVolumeModal(volume: StorageVolume): void {
    this.resizeVolume = volume;
    this.resizeFormData = {
      newSizeGb: volume.size_gb
    };
    this.isResizeModalVisible = true;
  }

  // 处理扩容确认
  handleResizeOk(): void {
    if (!this.resizeVolume || this.resizeFormData.newSizeGb <= this.resizeVolume.size_gb) {
      this.message.error('新大小必须大于当前大小');
      return;
    }

    this.loading = true;
    this.storageService.resizeVolume(
      this.resizeVolume.id,
      this.resizeFormData.newSizeGb
    ).subscribe({
      next: (resizedVolume: StorageVolume) => {
        this.message.success('存储卷扩容成功');
        this.isResizeModalVisible = false;
        this.resizeVolume = null;
        this.resizeFormData = { newSizeGb: 0 };
        this.loadStorageVolumes(this.pagination.current_page);
      },
      error: (error) => {
        this.loading = false;
        console.error('扩容存储卷失败:', error);
        this.message.error('扩容存储卷失败: ' + (error.error?.message || error.message || '未知错误'));
      }
    });
  }

  // 处理扩容取消
  handleResizeCancel(): void {
    this.isResizeModalVisible = false;
    this.resizeVolume = null;
    this.resizeFormData = { newSizeGb: 0 };
  }
}

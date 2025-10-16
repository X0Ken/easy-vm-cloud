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
import { NzSwitchModule } from 'ng-zorro-antd/switch';
import { NzPopconfirmModule } from 'ng-zorro-antd/popconfirm';
import { NzEmptyModule } from 'ng-zorro-antd/empty';
import { FormsModule } from '@angular/forms';
import { VmService, VM, Node, CreateVMRequest, UpdateVMRequest, PaginatedResponse } from '../../services/vm.service';
import { StorageService } from '../../services/storage.service';
import { NetworkService } from '../../services/network.service';
import { WebSocketService } from '../../services/websocket.service';
import { takeUntil } from 'rxjs/operators';
import { Subject } from 'rxjs';

@Component({
  selector: 'app-vms',
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
    NzSwitchModule,
    NzPopconfirmModule,
    NzEmptyModule,
    FormsModule
  ],
  templateUrl: './vms.component.html',
  styleUrls: ['./vms.component.scss']
})
export class VmsComponent implements OnInit {
  vms: VM[] = [];
  nodes: Node[] = [];
  availableDisks: any[] = [];
  availableNetworks: any[] = [];
  loading = false;
  isModalVisible = false;
  isEditMode = false;
  currentVm: VM | null = null;
  
  // 详情弹窗相关
  isDetailModalVisible = false;
  selectedVm: VM | null = null;
  vmVolumes: any[] = [];
  vmNetworks: any[] = [];
  volumesLoading = false;
  networksLoading = false;
  
  // WebSocket 相关
  private destroy$ = new Subject<void>();
  
  // 分页状态
  pagination = {
    current_page: 1,
    per_page: 20,
    total: 0,
    total_pages: 0,
    has_next: false,
    has_prev: false
  };
  
  // 表单数据
  formData = {
    name: '',
    node_id: null as string | null,
    vcpu: 1,
    memory_mb: 1024,
    selected_volume_id: null as string | null,
    selected_network_id: null as string | null
  };

  constructor(
    private vmService: VmService,
    private storageService: StorageService,
    private networkService: NetworkService,
    private websocketService: WebSocketService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadVms();
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
    // 监听 VM 状态更新
    this.websocketService.vmStatusUpdates$
      .pipe(takeUntil(this.destroy$))
      .subscribe(update => {
        console.log('收到 VM 状态更新:', update);
        this.handleVmStatusUpdate(update);
      });

    // 监听系统通知
    this.websocketService.systemNotifications$
      .pipe(takeUntil(this.destroy$))
      .subscribe(notification => {
        console.log('收到系统通知:', notification);
        this.handleSystemNotification(notification);
      });
  }

  /**
   * 处理 VM 状态更新
   */
  handleVmStatusUpdate(update: {vm_id: string, status: string, message?: string}): void {
    // 更新本地 VM 列表中的状态
    const vm = this.vms.find(v => v.id === update.vm_id);
    if (vm) {
      // 确保状态是有效的类型
      const validStatuses: ('running' | 'stopped' | 'stopping' | 'paused' | 'error')[] = 
        ['running', 'stopped', 'stopping', 'paused', 'error'];
      
      if (validStatuses.includes(update.status as any)) {
        const oldStatus = vm.status;
        vm.status = update.status as 'running' | 'stopped' | 'stopping' | 'paused' | 'error';
        console.log(`VM ${vm.name} 状态已更新为: ${update.status}`);
        
        // 根据状态变化显示相应的消息
        if (update.status === 'running' && oldStatus !== 'running') {
          this.message.success(`虚拟机 ${vm.name} 启动成功`);
        } else if (update.status === 'stopped' && oldStatus !== 'stopped') {
          this.message.success(`虚拟机 ${vm.name} 停止成功`);
        } else if (update.status === 'error') {
          this.message.error(`虚拟机 ${vm.name} 操作失败`);
        } else if (update.message && update.status !== 'running' && update.status !== 'stopped') {
          // 对于其他状态（如stopping），显示原始消息
          this.message.info(`VM ${vm.name}: ${update.message}`);
        }
      } else {
        console.warn(`收到无效的 VM 状态: ${update.status}`);
      }
    }
  }

  /**
   * 处理系统通知
   */
  handleSystemNotification(notification: {title: string, message: string, level: string}): void {
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

  loadVms(page: number = 1): void {
    this.loading = true;
    this.vmService.getVMs(page, this.pagination.per_page).subscribe({
      next: (response: PaginatedResponse<VM>) => {
        console.log('VM列表响应:', response);
        this.vms = response.data;
        console.log('转换后的VM数据:', this.vms);
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
        console.error('获取虚拟机列表失败:', error);
        this.message.error('获取虚拟机列表失败');
        this.loading = false;
      }
    });
  }

  loadNodes(): void {
    this.vmService.getNodes().subscribe({
      next: (nodes: Node[]) => {
        this.nodes = nodes;
      },
      error: (error) => {
        console.error('获取节点列表失败:', error);
        this.message.error('获取节点列表失败');
      }
    });
  }

  loadAvailableDisks(): void {
    this.storageService.getStorageVolumes(1, 100).subscribe({
      next: (response) => {
        this.availableDisks = response.data.filter((volume: any) => volume.status === 'available');
      },
      error: (error) => {
        console.error('获取可用磁盘列表失败:', error);
        this.message.error('获取可用磁盘列表失败');
      }
    });
  }

  loadAvailableNetworks(): void {
    this.networkService.getNetworks(1, 100).subscribe({
      next: (response) => {
        this.availableNetworks = response.data.filter((network: any) => network.status === 'active');
      },
      error: (error) => {
        console.error('获取可用网络列表失败:', error);
        this.message.error('获取可用网络列表失败');
      }
    });
  }

  onPageIndexChange(page: number): void {
    this.loadVms(page);
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadVms(1);
  }

  getStatusColor(status: string): string {
    const statusColors: { [key: string]: string } = {
      'running': 'green',
      'stopped': 'red',
      'stopping': 'orange',
      'paused': 'orange',
      'error': 'red'
    };
    return statusColors[status] || 'default';
  }

  getStatusText(status: string): string {
    const statusTexts: { [key: string]: string } = {
      'running': '运行中',
      'stopped': '已停止',
      'stopping': '停止中',
      'paused': '已暂停',
      'error': '错误'
    };
    return statusTexts[status] || status;
  }

  showCreateModal(): void {
    this.isEditMode = false;
    this.currentVm = null;
    this.resetForm();
    this.isModalVisible = true;
    
    // 加载创建虚拟机所需的数据
    this.loadNodes();
    this.loadAvailableDisks();
    this.loadAvailableNetworks();
  }

  showEditModal(vm: VM): void {
    this.isEditMode = true;
    this.currentVm = vm;
    this.formData = {
      name: vm.name,
      node_id: vm.node_id,
      vcpu: vm.vcpu,
      memory_mb: vm.memory_mb,
      selected_volume_id: null, // 编辑时不显示存储卷和网络选择
      selected_network_id: null
    };
    this.isModalVisible = true;
  }

  handleOk(): void {
    if (this.isEditMode && this.currentVm) {
      this.updateVm();
    } else {
      this.createVm();
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetForm();
  }

  createVm(): void {
    // 验证必需字段
    if (!this.formData.selected_volume_id) {
      this.message.error('请选择存储卷');
      return;
    }
    if (!this.formData.selected_network_id) {
      this.message.error('请选择网络');
      return;
    }
    if (!this.formData.node_id) {
      this.message.error('请选择部署节点');
      return;
    }

    const createData: CreateVMRequest = {
      name: this.formData.name,
      node_id: this.formData.node_id, // 转换为字符串
      vcpu: this.formData.vcpu,
      memory_mb: this.formData.memory_mb,
      disks: [{
        volume_id: this.formData.selected_volume_id!.toString(), // 存储卷ID转换为字符串
        device: 'vda', // 默认设备名
        bootable: true // 设为可启动
      }],
      networks: [{
        network_id: this.formData.selected_network_id!.toString(), // 网络ID转换为字符串
        mac_address: null, // 让后端自动分配
        ip_address: null, // 让后端自动分配
        model: 'virtio', // 默认网络模型
        bridge_name: null // 让后端自动处理
      }]
    };

    this.vmService.createVM(createData).subscribe({
      next: (response) => {
        this.message.success('虚拟机创建成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadVms(this.pagination.current_page);
      },
      error: (error) => {
        console.error('创建虚拟机失败:', error);
        this.message.error('创建虚拟机失败');
      }
    });
  }

  updateVm(): void {
    if (!this.currentVm) return;
    
    const updateData: UpdateVMRequest = {
      name: this.formData.name,
      vcpu: this.formData.vcpu,
      memory_mb: this.formData.memory_mb
    };

    this.vmService.updateVM(this.currentVm.id, updateData).subscribe({
      next: (response) => {
        this.message.success('虚拟机更新成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadVms(this.pagination.current_page);
      },
      error: (error) => {
        console.error('更新虚拟机失败:', error);
        this.message.error('更新虚拟机失败');
      }
    });
  }

  deleteVm(vm: VM): void {
    this.vmService.deleteVM(vm.id).subscribe({
      next: () => {
        this.message.success('虚拟机删除成功');
        this.loadVms(this.pagination.current_page);
      },
      error: (error) => {
        console.error('删除虚拟机失败:', error);
        this.message.error('删除虚拟机失败');
      }
    });
  }

  startVm(vm: VM): void {
    this.vmService.startVM(vm.id).subscribe({
      next: () => {
        this.message.info(`虚拟机 ${vm.name} 启动中...`);
        // 不立即更新状态，等待 WebSocket 通知
      },
      error: (error) => {
        console.error('启动虚拟机失败:', error);
        this.message.error('启动虚拟机失败');
      }
    });
  }

  stopVm(vm: VM): void {
    this.vmService.stopVM(vm.id).subscribe({
      next: () => {
        this.message.info(`虚拟机 ${vm.name} 停止中...`);
        // 不立即更新状态，等待 WebSocket 通知
      },
      error: (error) => {
        console.error('停止虚拟机失败:', error);
        this.message.error('停止虚拟机失败');
      }
    });
  }

  restartVm(vm: VM): void {
    this.vmService.restartVM(vm.id).subscribe({
      next: () => {
        this.message.info(`虚拟机 ${vm.name} 重启中...`);
        // 不立即更新状态，等待 WebSocket 通知
      },
      error: (error) => {
        console.error('重启虚拟机失败:', error);
        this.message.error('重启虚拟机失败');
      }
    });
  }


  resetForm(): void {
    this.formData = {
      name: '',
      node_id: null,
      vcpu: 1,
      memory_mb: 1024,
      selected_volume_id: null,
      selected_network_id: null
    };
  }

  formatMemory(memoryMb: number): string {
    if (memoryMb >= 1024) {
      return `${(memoryMb / 1024).toFixed(1)} GB`;
    }
    return `${memoryMb} MB`;
  }

  /**
   * 显示详情弹窗
   */
  showDetailModal(vm: VM): void {
    this.selectedVm = vm;
    this.isDetailModalVisible = true;
    this.loadVmDetails(vm.id);
  }

  /**
   * 关闭详情弹窗
   */
  handleDetailCancel(): void {
    this.isDetailModalVisible = false;
    this.selectedVm = null;
    this.vmVolumes = [];
    this.vmNetworks = [];
  }

  /**
   * 加载虚拟机详情信息
   */
  loadVmDetails(vmId: string): void {
    // 加载存储卷信息
    this.volumesLoading = true;
    this.vmService.getVMVolumes(vmId).subscribe({
      next: (volumes) => {
        this.vmVolumes = volumes;
        this.volumesLoading = false;
      },
      error: (error) => {
        console.error('获取虚拟机存储卷信息失败:', error);
        this.message.error('获取虚拟机存储卷信息失败');
        this.volumesLoading = false;
      }
    });

    // 加载网络信息
    this.networksLoading = true;
    this.vmService.getVMNetworks(vmId).subscribe({
      next: (networks) => {
        this.vmNetworks = networks;
        this.networksLoading = false;
      },
      error: (error) => {
        console.error('获取虚拟机网络信息失败:', error);
        this.message.error('获取虚拟机网络信息失败');
        this.networksLoading = false;
      }
    });
  }
}

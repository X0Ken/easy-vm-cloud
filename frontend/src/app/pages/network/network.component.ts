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
import { NetworkService, Network, IPAllocation, Node, CreateNetworkRequest, UpdateNetworkRequest, PaginatedResponse } from '../../services/network.service';

@Component({
  selector: 'app-network',
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
  templateUrl: './network.component.html',
  styleUrls: ['./network.component.scss']
})
export class NetworkComponent implements OnInit {
  networks: Network[] = [];
  ipAllocations: IPAllocation[] = [];
  nodes: Node[] = [];
  loading = false;
  isModalVisible = false;
  isEditMode = false;
  currentNetwork: Network | null = null;
  
  // IP分配模态框相关
  ipAllocationsModalVisible = false;
  ipAllocationsLoading = false;
  ipAllocationsPagination = {
    current_page: 1,
    per_page: 20,
    total: 0,
    total_pages: 0,
    has_next: false,
    has_prev: false
  };
  
  // 分页状态
  pagination = {
    current_page: 1,
    per_page: 20,
    total: 0,
    total_pages: 0,
    has_next: false,
    has_prev: false
  };
  
  // 网络表单数据
  networkFormData = {
    name: '',
    type: 'bridge' as 'bridge' | 'ovs',
    cidr: '',
    gateway: '',
    vlan_id: null as number | null,
    mtu: 1500,
    node_id: null as string | null
  };


  constructor(
    private networkService: NetworkService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadNetworks();
    this.loadNodes();
  }

  loadNetworks(page: number = 1): void {
    this.loading = true;
    this.networkService.getNetworks(page, this.pagination.per_page).subscribe({
      next: (response: PaginatedResponse<Network>) => {
        this.networks = response.data;
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
        console.error('获取网络列表失败:', error);
        this.message.error('获取网络列表失败');
        this.loading = false;
      }
    });
  }

  loadNodes(): void {
    this.networkService.getNodes().subscribe({
      next: (nodes: Node[]) => {
        this.nodes = nodes;
      },
      error: (error) => {
        console.error('获取节点列表失败:', error);
        this.message.error('获取节点列表失败');
      }
    });
  }


  onPageIndexChange(page: number): void {
    this.loadNetworks(page);
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadNetworks(1);
  }

  getStatusColor(status: string): string {
    const statusColors: { [key: string]: string } = {
      'active': 'green',
      'inactive': 'red',
      'error': 'red',
      'allocated': 'blue',
      'available': 'green',
      'reserved': 'orange'
    };
    return statusColors[status] || 'default';
  }

  getStatusText(status: string): string {
    const statusTexts: { [key: string]: string } = {
      'active': '活跃',
      'inactive': '非活跃',
      'error': '错误',
      'allocated': '已分配',
      'available': '可用',
      'reserved': '预留'
    };
    return statusTexts[status] || status;
  }

  getTypeText(type: string): string {
    const typeTexts: { [key: string]: string } = {
      'bridge': 'Linux Bridge',
      'ovs': 'Open vSwitch'
    };
    return typeTexts[type] || type;
  }

  getTypeColor(type: string): string {
    const typeColors: { [key: string]: string } = {
      'bridge': 'blue',
      'ovs': 'purple'
    };
    return typeColors[type] || 'default';
  }

  showCreateNetworkModal(): void {
    this.isEditMode = false;
    this.currentNetwork = null;
    this.resetNetworkForm();
    this.isModalVisible = true;
  }

  showEditNetworkModal(network: Network): void {
    this.isEditMode = true;
    this.currentNetwork = network;
    this.networkFormData = {
      name: network.name,
      type: network.type,
      cidr: network.cidr,
      gateway: network.gateway,
      vlan_id: network.vlan_id || null,
      mtu: network.mtu,
      node_id: network.node_id
    };
    this.isModalVisible = true;
  }


  handleOk(): void {
    if (this.isEditMode && this.currentNetwork) {
      this.updateNetwork();
    } else {
      this.createNetwork();
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetNetworkForm();
  }

  createNetwork(): void {
    const createData: CreateNetworkRequest = {
      name: this.networkFormData.name,
      network_type: this.networkFormData.type,
      cidr: this.networkFormData.cidr,
      gateway: this.networkFormData.gateway,
      vlan_id: this.networkFormData.vlan_id || undefined,
      mtu: this.networkFormData.mtu,
      node_id: this.networkFormData.node_id!
    };

    this.networkService.createNetwork(createData).subscribe({
      next: (response) => {
        this.message.success('网络创建成功');
        this.isModalVisible = false;
        this.resetNetworkForm();
        this.loadNetworks(this.pagination.current_page);
      },
      error: (error) => {
        console.error('创建网络失败:', error);
        this.message.error('创建网络失败');
      }
    });
  }

  updateNetwork(): void {
    if (!this.currentNetwork) return;
    
    // 编辑模式下只允许更新名称
    const updateData: UpdateNetworkRequest = {
      name: this.networkFormData.name
    };

    this.networkService.updateNetwork(this.currentNetwork.id, updateData).subscribe({
      next: (response) => {
        this.message.success('网络名称更新成功');
        this.isModalVisible = false;
        this.resetNetworkForm();
        this.loadNetworks(this.pagination.current_page);
      },
      error: (error) => {
        console.error('更新网络失败:', error);
        this.message.error('更新网络失败');
      }
    });
  }

  deleteNetwork(network: Network): void {
    this.networkService.deleteNetwork(network.id).subscribe({
      next: () => {
        this.message.success('网络删除成功');
        this.loadNetworks(this.pagination.current_page);
      },
      error: (error) => {
        console.error('删除网络失败:', error);
        this.message.error('删除网络失败');
      }
    });
  }



  resetNetworkForm(): void {
    this.networkFormData = {
      name: '',
      type: 'bridge', // 默认选择Linux Bridge
      cidr: '',
      gateway: '',
      vlan_id: null,
      mtu: 1500,
      node_id: null
    };
  }


  calculateIPUsage(available: number, total: number): number {
    return total > 0 ? Math.round(((total - available) / total) * 100) : 0;
  }

  // IP分配模态框相关方法
  showIPAllocationsModal(network: Network): void {
    this.currentNetwork = network;
    this.ipAllocationsModalVisible = true;
    this.loadIPAllocationsForNetwork(network.id.toString());
  }

  handleIPAllocationsModalCancel(): void {
    this.ipAllocationsModalVisible = false;
    this.currentNetwork = null;
    this.ipAllocations = [];
  }

  loadIPAllocationsForNetwork(networkId: string, page: number = 1): void {
    this.ipAllocationsLoading = true;
    // 只获取已分配的IP地址，过滤掉可用的IP
    this.networkService.getIPAllocations(networkId, page, this.ipAllocationsPagination.per_page, 'allocated').subscribe({
      next: (response: PaginatedResponse<IPAllocation>) => {
        this.ipAllocations = response.data;
        this.ipAllocationsPagination = {
          current_page: response.pagination?.current_page || page,
          per_page: response.pagination?.per_page || this.ipAllocationsPagination.per_page,
          total: response.pagination?.total || 0,
          total_pages: response.pagination?.total_pages || 0,
          has_next: response.pagination?.has_next || false,
          has_prev: response.pagination?.has_prev || false
        };
        this.ipAllocationsLoading = false;
      },
      error: (error) => {
        console.error('获取IP分配列表失败:', error);
        this.message.error('获取IP分配列表失败');
        this.ipAllocationsLoading = false;
      }
    });
  }

  onIPAllocationsPageIndexChange(page: number): void {
    if (this.currentNetwork) {
      this.loadIPAllocationsForNetwork(this.currentNetwork.id.toString(), page);
    }
  }

  onIPAllocationsPageSizeChange(pageSize: number): void {
    this.ipAllocationsPagination.per_page = pageSize;
    if (this.currentNetwork) {
      this.loadIPAllocationsForNetwork(this.currentNetwork.id.toString(), 1);
    }
  }

  allocateIPForNetwork(): void {
    if (!this.currentNetwork) return;
    
    this.networkService.allocateIP(this.currentNetwork.id.toString()).subscribe({
      next: (allocation: IPAllocation) => {
        this.message.success(`IP ${allocation.ip_address} 分配成功`);
        this.loadIPAllocationsForNetwork(this.currentNetwork!.id.toString(), this.ipAllocationsPagination.current_page);
      },
      error: (error) => {
        console.error('分配IP失败:', error);
        this.message.error('分配IP失败');
      }
    });
  }


  reserveIP(allocation: IPAllocation): void {
    // 根据后端逻辑，预留IP实际上是将状态从available改为reserved
    // 这里可以调用相应的API
    this.message.info('预留IP功能待实现');
  }
}

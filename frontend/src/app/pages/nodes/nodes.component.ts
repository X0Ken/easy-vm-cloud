import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { NzTableModule } from 'ng-zorro-antd/table';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzIconModule } from 'ng-zorro-antd/icon';
import { NzTagModule } from 'ng-zorro-antd/tag';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzStatisticModule } from 'ng-zorro-antd/statistic';
import { NzProgressModule } from 'ng-zorro-antd/progress';
import { NzModalModule } from 'ng-zorro-antd/modal';
import { NzMessageService } from 'ng-zorro-antd/message';
import { NzDrawerModule } from 'ng-zorro-antd/drawer';
import { NzDescriptionsModule } from 'ng-zorro-antd/descriptions';
import { NzSpinModule } from 'ng-zorro-antd/spin';
import { NzEmptyModule } from 'ng-zorro-antd/empty';
import { NzTooltipModule } from 'ng-zorro-antd/tooltip';
import { NzBadgeModule } from 'ng-zorro-antd/badge';
import { NzDividerModule } from 'ng-zorro-antd/divider';
import { NzSpaceModule } from 'ng-zorro-antd/space';
import { NzGridModule } from 'ng-zorro-antd/grid';
import { NzAlertModule } from 'ng-zorro-antd/alert';
import { NzPopconfirmModule } from 'ng-zorro-antd/popconfirm';
import { NzFormModule } from 'ng-zorro-antd/form';
import { NzInputModule } from 'ng-zorro-antd/input';
import { Subject, takeUntil } from 'rxjs';
import { NodeService, Node } from '../../services/node.service';

@Component({
  selector: 'app-nodes',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    NzTableModule,
    NzButtonModule,
    NzIconModule,
    NzTagModule,
    NzCardModule,
    NzStatisticModule,
    NzProgressModule,
    NzModalModule,
    NzDrawerModule,
    NzDescriptionsModule,
    NzSpinModule,
    NzEmptyModule,
    NzTooltipModule,
    NzBadgeModule,
    NzDividerModule,
    NzSpaceModule,
    NzGridModule,
    NzAlertModule,
    NzPopconfirmModule,
    NzFormModule,
    NzInputModule
  ],
  templateUrl: './nodes.component.html',
  styleUrl: './nodes.component.scss'
})
export class NodesComponent implements OnInit, OnDestroy {
  nodes: Node[] = [];
  loading = false;
  selectedNode: Node | null = null;
  detailDrawerVisible = false;
  
  // 详情弹窗相关
  isDetailModalVisible = false;
  
  // 创建弹窗相关
  isCreateModalVisible = false;
  
  
  // 分页状态
  pagination = {
    current_page: 1,
    per_page: 20,
    total: 0,
    total_pages: 0,
    has_next: false,
    has_prev: false
  };
  
  // 创建表单数据
  createFormData = {
    hostname: '',
    ip_address: ''
  };
  
  
  
  private destroy$ = new Subject<void>();

  constructor(
    private nodeService: NodeService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadNodes();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * 加载节点列表
   */
  loadNodes(): void {
    this.loading = true;
    this.nodeService.getNodes()
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (response: any) => {
          console.log('节点数据响应:', response);
          
          // 处理返回的数据结构
          this.nodes = response.nodes || [];
          
          // 更新分页信息
          this.pagination = {
            current_page: response.page || 1,
            per_page: response.page_size || 20,
            total: response.total || 0,
            total_pages: Math.ceil((response.total || 0) / (response.page_size || 20)),
            has_next: (response.page || 1) < Math.ceil((response.total || 0) / (response.page_size || 20)),
            has_prev: (response.page || 1) > 1
          };
          
          console.log('处理后的节点数据:', this.nodes);
          console.log('分页信息:', this.pagination);
          this.loading = false;
        },
        error: (error: any) => {
          console.error('加载节点列表失败:', error);
          this.message.error('加载节点列表失败');
          this.loading = false;
        }
      });
  }



  /**
   * 查看节点详情
   */
  viewNodeDetail(node: Node): void {
    this.selectedNode = node;
    this.isDetailModalVisible = true;
  }

  /**
   * 关闭详情抽屉
   */
  closeDetailDrawer(): void {
    this.detailDrawerVisible = false;
    this.selectedNode = null;
  }

  /**
   * 关闭详情模态框
   */
  handleDetailCancel(): void {
    this.isDetailModalVisible = false;
    this.selectedNode = null;
  }

  /**
   * 分页变化处理
   */
  onPageIndexChange(page: number): void {
    this.pagination.current_page = page;
    this.loadNodes();
  }

  /**
   * 页面大小变化处理
   */
  onPageSizeChange(size: number): void {
    this.pagination.per_page = size;
    this.pagination.current_page = 1;
    this.loadNodes();
  }


  /**
   * 获取状态标签颜色
   */
  getStatusColor(status: string): string {
    switch (status) {
      case 'online':
        return 'green';
      case 'offline':
        return 'red';
      case 'maintenance':
        return 'orange';
      default:
        return 'default';
    }
  }

  /**
   * 获取状态文本
   */
  getStatusText(status: string): string {
    switch (status) {
      case 'online':
        return '在线';
      case 'offline':
        return '离线';
      case 'maintenance':
        return '维护中';
      default:
        return '未知';
    }
  }

  /**
   * 格式化字节大小
   */
  formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }


  /**
   * 格式化时间
   */
  formatTime(time: string | null): string {
    if (!time) return '未知';
    const date = new Date(time);
    return date.toLocaleString('zh-CN');
  }

  /**
   * 获取相对时间
   */
  getRelativeTime(time: string | null): string {
    if (!time) return '未知';
    const now = new Date();
    const date = new Date(time);
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    
    if (minutes < 1) {
      return '刚刚';
    } else if (minutes < 60) {
      return `${minutes}分钟前`;
    } else {
      const hours = Math.floor(minutes / 60);
      if (hours < 24) {
        return `${hours}小时前`;
      } else {
        const days = Math.floor(hours / 24);
        return `${days}天前`;
      }
    }
  }

  /**
   * 删除节点
   */
  deleteNode(node: Node): void {
    this.nodeService.deleteNode(node.id).subscribe({
      next: () => {
        this.message.success('节点删除成功');
        this.loadNodes();
      },
      error: (error) => {
        console.error('删除节点失败:', error);
        this.message.error('删除节点失败');
      }
    });
  }

  /**
   * 显示创建模态框
   */
  showCreateModal(): void {
    this.resetCreateForm();
    this.isCreateModalVisible = true;
  }

  /**
   * 处理创建模态框确认
   */
  handleCreateOk(): void {
    this.createNode();
  }

  /**
   * 处理创建模态框取消
   */
  handleCreateCancel(): void {
    this.isCreateModalVisible = false;
    this.resetCreateForm();
  }

  /**
   * 创建节点
   */
  createNode(): void {
    // 验证必需字段
    if (!this.createFormData.hostname.trim()) {
      this.message.error('请输入节点名称');
      return;
    }
    if (!this.createFormData.ip_address.trim()) {
      this.message.error('请输入IP地址');
      return;
    }

    const createData = {
      hostname: this.createFormData.hostname.trim(),
      ip_address: this.createFormData.ip_address.trim()
    };

    this.nodeService.createNode(createData).subscribe({
      next: (response) => {
        this.message.success('节点创建成功');
        this.isCreateModalVisible = false;
        this.resetCreateForm();
        this.loadNodes();
      },
      error: (error) => {
        console.error('创建节点失败:', error);
        this.message.error('创建节点失败');
      }
    });
  }

  /**
   * 重置创建表单
   */
  resetCreateForm(): void {
    this.createFormData = {
      hostname: '',
      ip_address: ''
    };
  }

}

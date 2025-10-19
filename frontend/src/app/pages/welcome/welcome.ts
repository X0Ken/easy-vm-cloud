import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzIconModule } from 'ng-zorro-antd/icon';
import { NzMessageService } from 'ng-zorro-antd/message';
import { AuthService } from '../../services/auth.service';
import { VmService } from '../../services/vm.service';
import { NodeService } from '../../services/node.service';
import { StorageService } from '../../services/storage.service';
import { NetworkService } from '../../services/network.service';
import { forkJoin, Subscription, timer } from 'rxjs';
import { catchError, map } from 'rxjs/operators';

// 统计数据接口
interface PlatformStats {
  totalVms: number;
  runningVms: number;
  stoppedVms: number;
  totalNodes: number;
  onlineNodes: number;
  offlineNodes: number;
  totalStorage: number;
  usedStorage: number;
  availableStorage: number;
  totalNetworks: number;
  activeNetworks: number;
  inactiveNetworks: number;
}


@Component({
  selector: 'app-welcome',
  standalone: true,
  imports: [
    CommonModule, 
    NzButtonModule, 
    NzCardModule, 
    NzIconModule, 
    RouterModule
  ],
  templateUrl: './welcome.html',
  styleUrl: './welcome.scss'
})
export class Welcome implements OnInit, OnDestroy {
  loading = true;
  stats: PlatformStats = {
    totalVms: 0,
    runningVms: 0,
    stoppedVms: 0,
    totalNodes: 0,
    onlineNodes: 0,
    offlineNodes: 0,
    totalStorage: 0,
    usedStorage: 0,
    availableStorage: 0,
    totalNetworks: 0,
    activeNetworks: 0,
    inactiveNetworks: 0
  };

  private refreshTimer?: Subscription;

  constructor(
    private authService: AuthService,
    private router: Router,
    private message: NzMessageService,
    private vmService: VmService,
    private nodeService: NodeService,
    private storageService: StorageService,
    private networkService: NetworkService
  ) {}

  ngOnInit(): void {
    this.loadPlatformStats();
    
    // 设置定时刷新（每30秒）
    this.refreshTimer = timer(30000, 30000).subscribe(() => {
      this.loadPlatformStats();
    });
  }

  ngOnDestroy(): void {
    if (this.refreshTimer) {
      this.refreshTimer.unsubscribe();
    }
  }

  /**
   * 加载平台统计数据
   */
  private loadPlatformStats(): void {
    this.loading = true;
    
    forkJoin({
      vms: this.vmService.getVMs(1, 1).pipe(
        catchError(() => {
          console.warn('获取虚拟机数据失败');
          return [{ data: [], pagination: { total: 0 } }];
        })
      ),
      nodes: this.nodeService.getNodes({ page: 1, page_size: 1 }).pipe(
        catchError(() => {
          console.warn('获取节点数据失败');
          return [{ data: [], total: 0 }];
        })
      ),
      storage: this.storageService.getStoragePools(1, 1).pipe(
        catchError(() => {
          console.warn('获取存储数据失败');
          return [{ data: [], pagination: { total: 0 } }];
        })
      ),
      networks: this.networkService.getNetworks(1, 1).pipe(
        catchError(() => {
          console.warn('获取网络数据失败');
          return [{ data: [], pagination: { total: 0 } }];
        })
      )
    }).subscribe({
      next: (data) => {
        this.updateStats(data);
        this.loading = false;
      },
      error: (error) => {
        console.error('加载统计数据失败:', error);
        this.loading = false;
        this.message.error('加载统计数据失败');
      }
    });
  }

  /**
   * 更新统计数据
   */
  private updateStats(data: any): void {
    // 虚拟机统计
    const vms = data.vms.data || [];
    this.stats.totalVms = data.vms.pagination?.total || 0;
    this.stats.runningVms = vms.filter((vm: any) => vm.status === 'running').length;
    this.stats.stoppedVms = vms.filter((vm: any) => vm.status === 'stopped').length;

    // 节点统计
    const nodes = data.nodes.data || [];
    this.stats.totalNodes = data.nodes.total || 0;
    this.stats.onlineNodes = nodes.filter((node: any) => node.status === 'online').length;
    this.stats.offlineNodes = nodes.filter((node: any) => node.status === 'offline').length;

    // 存储统计
    const storagePools = data.storage.data || [];
    this.stats.totalStorage = storagePools.reduce((sum: number, pool: any) => sum + (pool.total_size_gb || 0), 0);
    this.stats.usedStorage = storagePools.reduce((sum: number, pool: any) => sum + (pool.used_size_gb || 0), 0);
    this.stats.availableStorage = this.stats.totalStorage - this.stats.usedStorage;

    // 网络统计
    const networks = data.networks.data || [];
    this.stats.totalNetworks = data.networks.pagination?.total || 0;
    this.stats.activeNetworks = networks.filter((network: any) => network.status === 'active').length;
    this.stats.inactiveNetworks = networks.filter((network: any) => network.status === 'inactive').length;
  }


  /**
   * 退出登录
   */
  logout(): void {
    this.authService.logout();
    this.message.success('已退出登录');
    this.router.navigate(['/login']);
  }

  /**
   * 刷新数据
   */
  refreshData(): void {
    this.loadPlatformStats();
    this.message.success('数据已刷新');
  }
}

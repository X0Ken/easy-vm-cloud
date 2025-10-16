import { Component, OnInit, OnDestroy } from '@angular/core';
import { RouterLink, RouterOutlet, Router, NavigationEnd } from '@angular/router';
import { NzIconModule } from 'ng-zorro-antd/icon';
import { NzLayoutModule } from 'ng-zorro-antd/layout';
import { NzMenuModule } from 'ng-zorro-antd/menu';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzTagModule } from 'ng-zorro-antd/tag';
import { NzMessageService } from 'ng-zorro-antd/message';
import { AuthService } from './services/auth.service';
import { WebSocketService } from './services/websocket.service';
import { filter, takeUntil } from 'rxjs/operators';
import { Subject } from 'rxjs';

@Component({
  selector: 'app-root',
  imports: [RouterLink, RouterOutlet, NzIconModule, NzLayoutModule, NzMenuModule, NzButtonModule, NzTagModule],
  templateUrl: './app.html',
  styleUrl: './app.scss'
})
export class App implements OnInit, OnDestroy {
  isCollapsed = false;
  currentUser: any = null;
  
  // WebSocket 连接状态
  websocketConnected = false;
  private destroy$ = new Subject<void>();
  
  // 菜单展开状态
  systemManagementOpen = false;
  personalCenterOpen = false;
  permissionsManagementOpen = false;
  nodesManagementOpen = false;  // 节点管理展开状态
  vmManagementOpen = false;
  storageManagementOpen = false;
  networkManagementOpen = false;

  constructor(
    private authService: AuthService,
    private websocketService: WebSocketService,
    private router: Router,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadCurrentUser();
    this.setupMenuExpansion();
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
    // 监听连接状态
    this.websocketService.connectionState$
      .pipe(takeUntil(this.destroy$))
      .subscribe(state => {
        this.websocketConnected = state === 'connected';
        console.log('WebSocket 连接状态:', state);
      });
  }

  loadCurrentUser(): void {
    if (this.authService.isAuthenticated()) {
      this.authService.getCurrentUser().subscribe({
        next: (response) => {
          this.currentUser = response.user;
        },
        error: (error) => {
          console.error('获取用户信息失败:', error);
        }
      });
    }
  }

  logout(): void {
    this.authService.logout();
    this.message.success('已退出登录');
    this.router.navigate(['/login']);
  }

  /**
   * 设置菜单展开逻辑
   */
  private setupMenuExpansion(): void {
    // 监听路由变化
    this.router.events
      .pipe(filter(event => event instanceof NavigationEnd))
      .subscribe((event: NavigationEnd) => {
        this.updateMenuExpansion(event.url);
      });

    // 初始化时设置菜单状态
    this.updateMenuExpansion(this.router.url);
  }

  /**
   * 根据当前URL更新菜单展开状态
   */
  private updateMenuExpansion(url: string): void {
    // 重置所有菜单状态
    this.systemManagementOpen = false;
    this.personalCenterOpen = false;
    this.permissionsManagementOpen = false;
    this.nodesManagementOpen = false;
    this.vmManagementOpen = false;
    this.storageManagementOpen = false;
    this.networkManagementOpen = false;
    
    // 根据URL路径设置对应的菜单展开
    if (url.startsWith('/permissions')) {
      this.permissionsManagementOpen = true;
    } else if (url.startsWith('/system')) {
      this.systemManagementOpen = true;
    } else if (url.startsWith('/profile')) {
      this.personalCenterOpen = true;
    } else if (url.startsWith('/nodes')) {
      this.nodesManagementOpen = true;
    } else if (url.startsWith('/vms')) {
      this.vmManagementOpen = true;
    } else if (url.startsWith('/storage')) {
      this.storageManagementOpen = true;
    } else if (url.startsWith('/network')) {
      this.networkManagementOpen = true;
    }
  }
}

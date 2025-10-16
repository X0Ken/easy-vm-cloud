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
import { NzSwitchModule } from 'ng-zorro-antd/switch';
import { NzSelectModule } from 'ng-zorro-antd/select';
import { NzCheckboxModule } from 'ng-zorro-antd/checkbox';
import { FormsModule } from '@angular/forms';
import { RoleService, Role, PaginationInfo } from '../../../services/role.service';
import { PermissionService, Permission } from '../../../services/permission.service';

@Component({
  selector: 'app-roles',
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
    NzSwitchModule,
    NzSelectModule,
    NzCheckboxModule,
    FormsModule
  ],
  templateUrl: './roles.component.html',
  styleUrls: ['./roles.component.scss']
})
export class RolesComponent implements OnInit {
  roles: Role[] = [];
  loading = false;
  isModalVisible = false;
  isEditMode = false;
  currentRole: Role | null = null;
  
  // 权限管理相关
  isPermissionModalVisible = false;
  allPermissions: Permission[] = [];
  selectedPermissions: string[] = [];
  permissionLoading = false;
  
  // 分页状态
  pagination: PaginationInfo = {
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
    description: '',
    is_active: true
  };

  constructor(
    private roleService: RoleService,
    private permissionService: PermissionService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadRoles();
    this.loadAllPermissions();
  }

  loadRoles(page: number = 1): void {
    this.loading = true;
    this.roleService.getRoles(page, this.pagination.per_page).subscribe({
      next: (response) => {
        this.roles = response.data;
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
        console.error('获取角色列表失败:', error);
        this.message.error('获取角色列表失败');
        this.loading = false;
      }
    });
  }

  loadAllPermissions(): void {
    this.permissionService.getPermissions(1, 100).subscribe({
      next: (response: any) => {
        this.allPermissions = response.data;
      },
      error: (error: any) => {
        console.error('获取权限列表失败:', error);
        this.message.error('获取权限列表失败');
      }
    });
  }

  onPageIndexChange(page: number): void {
    this.loadRoles(page);
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadRoles(1);
  }

  showCreateModal(): void {
    this.isEditMode = false;
    this.currentRole = null;
    this.formData = {
      name: '',
      description: '',
      is_active: true
    };
    this.isModalVisible = true;
  }

  showEditModal(role: Role): void {
    this.isEditMode = true;
    this.currentRole = role;
    this.formData = {
      name: role.name,
      description: role.description,
      is_active: role.is_active
    };
    this.isModalVisible = true;
  }

  showPermissionModal(role: Role): void {
    this.currentRole = role;
    this.selectedPermissions = [...role.permissions];
    this.isPermissionModalVisible = true;
  }

  handleOk(): void {
    if (this.isEditMode && this.currentRole) {
      this.roleService.updateRole(this.currentRole.id, this.formData).subscribe({
        next: () => {
          this.message.success('角色更新成功');
          this.isModalVisible = false;
          this.loadRoles(this.pagination.current_page);
        },
        error: (error) => {
          console.error('更新角色失败:', error);
          this.message.error('更新角色失败');
        }
      });
    } else {
      this.roleService.createRole(this.formData).subscribe({
        next: () => {
          this.message.success('角色创建成功');
          this.isModalVisible = false;
          this.loadRoles(1);
        },
        error: (error) => {
          console.error('创建角色失败:', error);
          this.message.error('创建角色失败');
        }
      });
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
  }

  deleteRole(role: Role): void {
    this.roleService.deleteRole(role.id).subscribe({
      next: () => {
        this.message.success('角色删除成功');
        this.loadRoles(this.pagination.current_page);
      },
      error: (error) => {
        console.error('删除角色失败:', error);
        this.message.error('删除角色失败');
      }
    });
  }

  handlePermissionOk(): void {
    if (!this.currentRole) return;
    
    this.permissionLoading = true;
    
    // 找出需要添加的权限
    const permissionsToAdd = this.selectedPermissions.filter(
      permission => !this.currentRole!.permissions.includes(permission)
    );
    
    // 找出需要移除的权限
    const permissionsToRemove = this.currentRole.permissions.filter(
      permission => !this.selectedPermissions.includes(permission)
    );
    
    // 执行权限操作
    const operations: Promise<any>[] = [];
    
    // 添加权限
    for (const permissionName of permissionsToAdd) {
      const permission = this.allPermissions.find(p => p.name === permissionName);
      if (permission) {
        operations.push(
          this.roleService.assignPermission(this.currentRole!.id, permission.id).toPromise()
        );
      }
    }
    
    // 移除权限
    for (const permissionName of permissionsToRemove) {
      const permission = this.allPermissions.find(p => p.name === permissionName);
      if (permission) {
        operations.push(
          this.roleService.removePermission(this.currentRole!.id, permission.id).toPromise()
        );
      }
    }
    
    // 执行所有操作
    if (operations.length > 0) {
      Promise.all(operations).then(() => {
        this.message.success('权限更新成功');
        this.isPermissionModalVisible = false;
        this.loadRoles(this.pagination.current_page);
        this.permissionLoading = false;
      }).catch(error => {
        console.error('权限更新失败:', error);
        this.message.error('权限更新失败');
        this.permissionLoading = false;
      });
    } else {
      this.message.info('没有权限变更');
      this.isPermissionModalVisible = false;
      this.permissionLoading = false;
    }
  }

  handlePermissionCancel(): void {
    this.isPermissionModalVisible = false;
    this.selectedPermissions = [];
  }

  getStatusColor(isActive: boolean): string {
    return isActive ? 'green' : 'red';
  }

  getStatusText(isActive: boolean): string {
    return isActive ? '活跃' : '禁用';
  }

  getResourceColor(resource: string): string {
    const colors: { [key: string]: string } = {
      'user': 'blue',
      'role': 'purple',
      'permission': 'orange',
      'article': 'green',
      'comment': 'cyan'
    };
    return colors[resource] || 'default';
  }

  getActionColor(action: string): string {
    const colors: { [key: string]: string } = {
      'read': 'green',
      'create': 'blue',
      'update': 'orange',
      'delete': 'red',
      'write': 'purple',
      'edit': 'cyan'
    };
    return colors[action] || 'default';
  }

  togglePermission(permissionName: string): void {
    if (this.selectedPermissions.includes(permissionName)) {
      this.selectedPermissions = this.selectedPermissions.filter(p => p !== permissionName);
    } else {
      this.selectedPermissions.push(permissionName);
    }
  }
}

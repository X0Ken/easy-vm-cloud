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
import { FormsModule } from '@angular/forms';
import { PermissionService, Permission, PaginationInfo } from '../../../services/permission.service';

@Component({
  selector: 'app-permissions',
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
    FormsModule
  ],
  templateUrl: './permissions.component.html',
  styleUrls: ['./permissions.component.scss']
})
export class PermissionsComponent implements OnInit {
  permissions: Permission[] = [];
  loading = false;
  isModalVisible = false;
  isEditMode = false;
  currentPermission: Permission | null = null;
  
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
    resource: '',
    action: '',
    is_active: true
  };

  // 预定义的资源和操作
  resources = ['user', 'role', 'permission', 'article', 'comment'];
  actions = ['read', 'create', 'update', 'delete', 'write', 'edit'];

  constructor(
    private permissionService: PermissionService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadPermissions();
  }

  loadPermissions(page: number = 1): void {
    this.loading = true;
    this.permissionService.getPermissions(page, this.pagination.per_page).subscribe({
      next: (response) => {
        this.permissions = response.data;
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
        console.error('获取权限列表失败:', error);
        this.message.error('获取权限列表失败');
        this.loading = false;
      }
    });
  }

  onPageIndexChange(page: number): void {
    this.loadPermissions(page);
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadPermissions(1);
  }

  showCreateModal(): void {
    this.isEditMode = false;
    this.currentPermission = null;
    this.resetForm();
    this.isModalVisible = true;
  }

  showEditModal(permission: Permission): void {
    this.isEditMode = true;
    this.currentPermission = permission;
    this.formData = {
      name: permission.name,
      description: permission.description,
      resource: permission.resource,
      action: permission.action,
      is_active: permission.is_active
    };
    this.isModalVisible = true;
  }

  handleOk(): void {
    if (this.isEditMode && this.currentPermission) {
      this.updatePermission();
    } else {
      this.createPermission();
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetForm();
  }

  createPermission(): void {
    this.permissionService.createPermission({
      name: this.formData.name,
      description: this.formData.description,
      resource: this.formData.resource,
      action: this.formData.action
    }).subscribe({
      next: (response) => {
        this.message.success('权限创建成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadPermissions(this.pagination.current_page);
      },
      error: (error) => {
        console.error('创建权限失败:', error);
        this.message.error('创建权限失败');
      }
    });
  }

  updatePermission(): void {
    if (!this.currentPermission) return;
    
    this.permissionService.updatePermission(this.currentPermission.id, {
      name: this.formData.name,
      description: this.formData.description,
      resource: this.formData.resource,
      action: this.formData.action,
      is_active: this.formData.is_active
    }).subscribe({
      next: (response) => {
        this.message.success('权限更新成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadPermissions(this.pagination.current_page);
      },
      error: (error) => {
        console.error('更新权限失败:', error);
        this.message.error('更新权限失败');
      }
    });
  }

  deletePermission(permission: Permission): void {
    this.permissionService.deletePermission(permission.id).subscribe({
      next: (response) => {
        this.message.success('权限删除成功');
        this.loadPermissions(this.pagination.current_page);
      },
      error: (error) => {
        console.error('删除权限失败:', error);
        this.message.error('删除权限失败');
      }
    });
  }

  resetForm(): void {
    this.formData = {
      name: '',
      description: '',
      resource: '',
      action: '',
      is_active: true
    };
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
}

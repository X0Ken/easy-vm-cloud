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
import { FormsModule } from '@angular/forms';
import { UserService, User, PaginationInfo } from '../../../services/user.service';

@Component({
  selector: 'app-users',
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
    FormsModule
  ],
  templateUrl: './users.component.html',
  styleUrls: ['./users.component.scss']
})
export class UsersComponent implements OnInit {
  users: User[] = [];
  loading = false;
  isModalVisible = false;
  isEditMode = false;
  currentUser: User | null = null;
  
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
    username: '',
    email: '',
    password: '',
    is_active: true
  };

  constructor(
    private userService: UserService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadUsers();
  }

  loadUsers(page: number = 1): void {
    this.loading = true;
    this.userService.getUsers(page, this.pagination.per_page).subscribe({
      next: (response) => {
        this.users = response.data;
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
        console.error('获取用户列表失败:', error);
        this.message.error('获取用户列表失败');
        this.loading = false;
      }
    });
  }

  onPageIndexChange(page: number): void {
    this.loadUsers(page);
  }

  onPageSizeChange(pageSize: number): void {
    this.pagination.per_page = pageSize;
    this.loadUsers(1);
  }

  getStatusColor(isActive: boolean): string {
    return isActive ? 'green' : 'red';
  }

  getStatusText(isActive: boolean): string {
    return isActive ? '活跃' : '禁用';
  }

  showCreateModal(): void {
    this.isEditMode = false;
    this.currentUser = null;
    this.resetForm();
    this.isModalVisible = true;
  }

  showEditModal(user: User): void {
    this.isEditMode = true;
    this.currentUser = user;
    this.formData = {
      username: user.username,
      email: user.email,
      password: '',
      is_active: user.is_active
    };
    this.isModalVisible = true;
  }

  handleOk(): void {
    if (this.isEditMode && this.currentUser) {
      this.updateUser();
    } else {
      this.createUser();
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetForm();
  }

  createUser(): void {
    this.userService.createUser({
      username: this.formData.username,
      email: this.formData.email,
      password: this.formData.password
    }).subscribe({
      next: (response) => {
        this.message.success('用户创建成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadUsers(this.pagination.current_page);
      },
      error: (error) => {
        console.error('创建用户失败:', error);
        this.message.error('创建用户失败');
      }
    });
  }

  updateUser(): void {
    if (!this.currentUser) return;
    
    this.userService.updateUser(this.currentUser.id, {
      email: this.formData.email,
      is_active: this.formData.is_active
    }).subscribe({
      next: (response) => {
        this.message.success('用户更新成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadUsers(this.pagination.current_page);
      },
      error: (error) => {
        console.error('更新用户失败:', error);
        this.message.error('更新用户失败');
      }
    });
  }

  deleteUser(user: User): void {
    this.userService.deleteUser(user.id).subscribe({
      next: (response) => {
        this.message.success('用户删除成功');
        this.loadUsers(this.pagination.current_page);
      },
      error: (error) => {
        console.error('删除用户失败:', error);
        this.message.error('删除用户失败');
      }
    });
  }

  resetForm(): void {
    this.formData = {
      username: '',
      email: '',
      password: '',
      is_active: true
    };
  }
}

import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzTableModule } from 'ng-zorro-antd/table';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzIconModule } from 'ng-zorro-antd/icon';
import { NzModalModule } from 'ng-zorro-antd/modal';
import { NzFormModule } from 'ng-zorro-antd/form';
import { NzInputModule } from 'ng-zorro-antd/input';
import { NzSelectModule } from 'ng-zorro-antd/select';
import { NzSpinModule } from 'ng-zorro-antd/spin';
import { NzMessageService } from 'ng-zorro-antd/message';
import { NzTabsModule } from 'ng-zorro-antd/tabs';
import { NzSwitchModule } from 'ng-zorro-antd/switch';
import { NzTagModule } from 'ng-zorro-antd/tag';
import { NzDividerModule } from 'ng-zorro-antd/divider';
import { NzPopconfirmModule } from 'ng-zorro-antd/popconfirm';
import { UserDepartmentService, UserDepartment, AssignUserRequest, BatchAssignRequest, UpdateUserDepartmentRequest } from '../../../services/user-department.service';
import { DepartmentService, Department } from '../../../services/department.service';
import { UserService, User } from '../../../services/user.service';

@Component({
  selector: 'app-user-departments',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    NzCardModule,
    NzTableModule,
    NzButtonModule,
    NzIconModule,
    NzModalModule,
    NzFormModule,
    NzInputModule,
    NzSelectModule,
    NzSpinModule,
    NzTabsModule,
    NzSwitchModule,
    NzTagModule,
    NzDividerModule,
    NzPopconfirmModule
  ],
  templateUrl: './user-departments.component.html',
  styleUrls: ['./user-departments.component.scss']
})
export class UserDepartmentsComponent implements OnInit {
  userDepartments: UserDepartment[] = [];
  departments: Department[] = [];
  users: User[] = [];
  loading = false;

  // 模态框状态
  isAssignModalVisible = false;
  isBatchAssignModalVisible = false;
  isEditModalVisible = false;
  currentUserDepartment: UserDepartment | null = null;

  // 分配用户表单
  assignForm = {
    user_id: null as number | null,
    department_id: null as number | null,
    position: '',
    is_primary: false
  };

  // 批量分配表单
  batchAssignForm = {
    user_ids: [] as number[],
    department_id: null as number | null,
    position: ''
  };

  // 编辑表单
  editForm = {
    position: '',
    is_primary: false
  };

  // 当前选中的标签页
  selectedTabIndex = 0;

  // 搜索条件
  searchForm = {
    user_id: null as number | null,
    department_id: null as number | null
  };

  constructor(
    private userDepartmentService: UserDepartmentService,
    private departmentService: DepartmentService,
    private userService: UserService,
    private message: NzMessageService
  ) {}

  ngOnInit(): void {
    this.loadData();
  }

  loadData(): void {
    this.loadDepartments();
    this.loadUsers();
    this.loadUserDepartments();
  }

  loadDepartments(): void {
    this.departmentService.getDepartments().subscribe({
      next: (response) => {
        this.departments = response.data;
      },
      error: (error) => {
        console.error('获取部门列表失败:', error);
        this.message.error('获取部门列表失败');
      }
    });
  }

  loadUsers(): void {
    this.userService.getUsers(1, 100).subscribe({
      next: (response) => {
        this.users = response.data;
      },
      error: (error) => {
        console.error('获取用户列表失败:', error);
        this.message.error('获取用户列表失败');
      }
    });
  }

  loadUserDepartments(): void {
    this.loading = true;
    // 这里暂时获取所有关联，实际项目中可能需要分页或搜索
    // 由于API没有提供获取所有关联的接口，我们需要根据搜索条件来获取
    this.loading = false;
    this.userDepartments = [];
  }

  // 根据用户ID搜索该用户的部门关联
  searchByUser(): void {
    if (!this.searchForm.user_id) {
      this.message.warning('请选择用户');
      return;
    }

    this.loading = true;
    this.userDepartmentService.getUserDepartments(this.searchForm.user_id).subscribe({
      next: (response) => {
        this.userDepartments = response.data;
        this.loading = false;
      },
      error: (error) => {
        console.error('获取用户部门关联失败:', error);
        this.message.error('获取用户部门关联失败');
        this.loading = false;
      }
    });
  }

  // 根据部门ID搜索该部门的用户关联
  searchByDepartment(): void {
    if (!this.searchForm.department_id) {
      this.message.warning('请选择部门');
      return;
    }

    this.loading = true;
    this.userDepartmentService.getDepartmentUsers(this.searchForm.department_id).subscribe({
      next: (response) => {
        this.userDepartments = response.data;
        this.loading = false;
      },
      error: (error) => {
        console.error('获取部门用户关联失败:', error);
        this.message.error('获取部门用户关联失败');
        this.loading = false;
      }
    });
  }

  clearSearch(): void {
    this.searchForm = {
      user_id: null,
      department_id: null
    };
    this.userDepartments = [];
  }

  getUserName(userId: number): string {
    const user = this.users.find(u => u.id === userId);
    return user ? user.username : '未知用户';
  }

  getDepartmentName(departmentId: number): string {
    const department = this.departments.find(d => d.id === departmentId);
    return department ? department.name : '未知部门';
  }

  showAssignModal(): void {
    this.resetAssignForm();
    this.isAssignModalVisible = true;
  }

  showBatchAssignModal(): void {
    this.resetBatchAssignForm();
    this.isBatchAssignModalVisible = true;
  }

  showEditModal(userDepartment: UserDepartment): void {
    this.currentUserDepartment = userDepartment;
    this.editForm = {
      position: userDepartment.position || '',
      is_primary: userDepartment.is_primary
    };
    this.isEditModalVisible = true;
  }

  handleAssignOk(): void {
    if (!this.assignForm.user_id || !this.assignForm.department_id) {
      this.message.warning('请选择用户和部门');
      return;
    }

    const request: AssignUserRequest = {
      user_id: this.assignForm.user_id,
      department_id: this.assignForm.department_id,
      position: this.assignForm.position || undefined,
      is_primary: this.assignForm.is_primary
    };

    this.userDepartmentService.assignUser(request).subscribe({
      next: (response) => {
        this.message.success('用户分配成功');
        this.isAssignModalVisible = false;
        this.refreshCurrentView();
      },
      error: (error) => {
        console.error('用户分配失败:', error);
        this.message.error(error.error?.error || '用户分配失败');
      }
    });
  }

  handleBatchAssignOk(): void {
    if (!this.batchAssignForm.user_ids.length || !this.batchAssignForm.department_id) {
      this.message.warning('请选择用户和部门');
      return;
    }

    const request: BatchAssignRequest = {
      user_ids: this.batchAssignForm.user_ids,
      department_id: this.batchAssignForm.department_id,
      position: this.batchAssignForm.position || undefined
    };

    this.userDepartmentService.batchAssignUsers(request).subscribe({
      next: (response) => {
        this.message.success(`批量分配成功，分配了 ${response.data.assigned_count} 个用户`);
        this.isBatchAssignModalVisible = false;
        this.refreshCurrentView();
      },
      error: (error) => {
        console.error('批量分配失败:', error);
        this.message.error(error.error?.error || '批量分配失败');
      }
    });
  }

  handleEditOk(): void {
    if (!this.currentUserDepartment) return;

    const request: UpdateUserDepartmentRequest = {
      position: this.editForm.position || undefined,
      is_primary: this.editForm.is_primary
    };

    this.userDepartmentService.updateUserDepartment(this.currentUserDepartment.id, request).subscribe({
      next: (response) => {
        this.message.success('更新成功');
        this.isEditModalVisible = false;
        this.refreshCurrentView();
      },
      error: (error) => {
        console.error('更新失败:', error);
        this.message.error(error.error?.error || '更新失败');
      }
    });
  }

  removeUserDepartment(userDepartment: UserDepartment): void {
    this.userDepartmentService.removeUserDepartment(userDepartment.id).subscribe({
      next: (response) => {
        this.message.success('移除成功');
        this.refreshCurrentView();
      },
      error: (error) => {
        console.error('移除失败:', error);
        this.message.error(error.error?.error || '移除失败');
      }
    });
  }

  refreshCurrentView(): void {
    if (this.searchForm.user_id) {
      this.searchByUser();
    } else if (this.searchForm.department_id) {
      this.searchByDepartment();
    }
  }

  resetAssignForm(): void {
    this.assignForm = {
      user_id: null,
      department_id: null,
      position: '',
      is_primary: false
    };
  }

  resetBatchAssignForm(): void {
    this.batchAssignForm = {
      user_ids: [],
      department_id: null,
      position: ''
    };
  }

  handleAssignCancel(): void {
    this.isAssignModalVisible = false;
    this.resetAssignForm();
  }

  handleBatchAssignCancel(): void {
    this.isBatchAssignModalVisible = false;
    this.resetBatchAssignForm();
  }

  handleEditCancel(): void {
    this.isEditModalVisible = false;
    this.currentUserDepartment = null;
  }
}

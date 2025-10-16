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
import { NzTreeModule } from 'ng-zorro-antd/tree';
import { NzSpinModule } from 'ng-zorro-antd/spin';
import { NzMessageService } from 'ng-zorro-antd/message';
import { NzTabsModule } from 'ng-zorro-antd/tabs';
import { NzInputNumberModule } from 'ng-zorro-antd/input-number';
import { NzDividerModule } from 'ng-zorro-antd/divider';
import { NzPopconfirmModule } from 'ng-zorro-antd/popconfirm';
import { NzTreeNodeOptions } from 'ng-zorro-antd/tree';
import { DepartmentService, Department, CreateDepartmentRequest, UpdateDepartmentRequest } from '../../../services/department.service';
import { UserService, User } from '../../../services/user.service';

@Component({
  selector: 'app-departments',
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
    NzTreeModule,
    NzSpinModule,
    NzTabsModule,
    NzInputNumberModule,
    NzDividerModule,
    NzPopconfirmModule
  ],
  templateUrl: './departments.component.html',
  styleUrls: ['./departments.component.scss']
})
export class DepartmentsComponent implements OnInit {
  departments: Department[] = [];
  treeData: NzTreeNodeOptions[] = [];
  users: User[] = [];
  loading = false;
  treeLoading = false;

  // 模态框状态
  isModalVisible = false;
  isEditMode = false;
  currentDepartment: Department | null = null;

  // 表单数据
  formData = {
    name: '',
    description: '',
    parent_id: null as number | null,
    manager_id: null as number | null,
    sort_order: 0
  };

  // 当前选中的标签页
  selectedTabIndex = 0;
  // 是否已经加载过树形数据
  treeDataLoaded = false;

  constructor(
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
    // 移除自动加载树形数据，改为点击时加载
  }

  loadDepartments(): void {
    this.loading = true;
    this.departmentService.getDepartments().subscribe({
      next: (response) => {
        this.departments = response.data;
        this.loading = false;
      },
      error: (error) => {
        console.error('获取部门列表失败:', error);
        this.message.error('获取部门列表失败');
        this.loading = false;
      }
    });
  }

  loadDepartmentTree(): void {
    this.treeLoading = true;
    this.departmentService.getDepartmentTree().subscribe({
      next: (response) => {
        this.treeData = this.convertToTreeNodes(response.data);
        this.treeLoading = false;
        this.treeDataLoaded = true;
      },
      error: (error) => {
        console.error('获取部门树形结构失败:', error);
        this.message.error('获取部门树形结构失败');
        this.treeLoading = false;
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
      }
    });
  }

  convertToTreeNodes(departments: Department[]): NzTreeNodeOptions[] {
    return departments.map(dept => ({
      title: dept.name,
      key: dept.id.toString(),
      isLeaf: !dept.children || dept.children.length === 0,
      children: dept.children ? this.convertToTreeNodes(dept.children) : [],
      expanded: true,
      selectable: true,
      origin: dept
    }));
  }

  getDepartmentOptions(): Array<{ label: string; value: number }> {
    const options: Array<{ label: string; value: number }> = [];
    const addOptions = (depts: Department[], prefix = '') => {
      depts.forEach(dept => {
        options.push({
          label: prefix + dept.name,
          value: dept.id
        });
        if (dept.children && dept.children.length > 0) {
          addOptions(dept.children, prefix + '├─ ');
        }
      });
    };

    // 使用树形数据来构建层级选项
    this.departmentService.getDepartmentTree().subscribe(response => {
      addOptions(response.data);
    });

    return options;
  }

  getParentDepartmentName(parentId: number | null | undefined): string {
    if (!parentId) return '无';
    const parent = this.departments.find(d => d.id === parentId);
    return parent ? parent.name : '未知';
  }

  getManagerName(managerId: number | null | undefined): string {
    if (!managerId) return '无';
    const manager = this.users.find(u => u.id === managerId);
    return manager ? manager.username : '未知';
  }

  showCreateModal(): void {
    this.isEditMode = false;
    this.currentDepartment = null;
    this.resetForm();
    this.isModalVisible = true;
  }

  showEditModal(department: Department): void {
    this.isEditMode = true;
    this.currentDepartment = department;
    this.formData = {
      name: department.name,
      description: department.description || '',
      parent_id: department.parent_id ?? null,
      manager_id: department.manager_id ?? null,
      sort_order: department.sort_order
    };
    this.isModalVisible = true;
  }

  handleOk(): void {
    if (this.isEditMode && this.currentDepartment) {
      this.updateDepartment();
    } else {
      this.createDepartment();
    }
  }

  handleCancel(): void {
    this.isModalVisible = false;
    this.resetForm();
  }

  createDepartment(): void {
    const request: CreateDepartmentRequest = {
      name: this.formData.name,
      description: this.formData.description || undefined,
      parent_id: this.formData.parent_id || undefined,
      manager_id: this.formData.manager_id || undefined,
      sort_order: this.formData.sort_order
    };

    this.departmentService.createDepartment(request).subscribe({
      next: (response) => {
        this.message.success('部门创建成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadData();
      },
      error: (error) => {
        console.error('创建部门失败:', error);
        this.message.error(error.error?.error || '创建部门失败');
      }
    });
  }

  updateDepartment(): void {
    if (!this.currentDepartment) return;

    const request: UpdateDepartmentRequest = {
      name: this.formData.name,
      description: this.formData.description || undefined,
      parent_id: this.formData.parent_id || undefined,
      manager_id: this.formData.manager_id || undefined,
      sort_order: this.formData.sort_order
    };

    this.departmentService.updateDepartment(this.currentDepartment.id, request).subscribe({
      next: (response) => {
        this.message.success('部门更新成功');
        this.isModalVisible = false;
        this.resetForm();
        this.loadData();
      },
      error: (error) => {
        console.error('更新部门失败:', error);
        this.message.error(error.error?.error || '更新部门失败');
      }
    });
  }

  deleteDepartment(department: Department): void {
    this.departmentService.deleteDepartment(department.id).subscribe({
      next: (response) => {
        this.message.success('部门删除成功');
        this.loadData();
      },
      error: (error) => {
        console.error('删除部门失败:', error);
        this.message.error(error.error?.error || '删除部门失败');
      }
    });
  }

  resetForm(): void {
    this.formData = {
      name: '',
      description: '',
      parent_id: null,
      manager_id: null,
      sort_order: 0
    };
  }

  onTreeNodeClick(event: any): void {
    if (event.node && event.node.origin) {
      const department = event.node.origin as Department;
      this.showEditModal(department);
    }
  }

  onTabChange(index: number): void {
    this.selectedTabIndex = index;
    // 当切换到树形视图（索引为1）且树形数据未加载时，加载树形数据
    if (index === 1 && !this.treeDataLoaded) {
      this.loadDepartmentTree();
    }
  }
}

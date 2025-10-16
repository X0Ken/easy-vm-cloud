import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ApiConfig } from '../config/api.config';

export interface Department {
  id: number;
  name: string;
  description?: string;
  parent_id: number | null;
  manager_id: number | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
  children?: Department[];
}

export interface CreateDepartmentRequest {
  name: string;
  description?: string;
  parent_id?: number | null;
  manager_id?: number | null;
  sort_order?: number;
}

export interface UpdateDepartmentRequest {
  name?: string;
  description?: string;
  parent_id?: number | null;
  manager_id?: number | null;
  sort_order?: number;
}

export interface DepartmentResponse {
  data: Department;
  message?: string;
}

export interface DepartmentListResponse {
  data: Department[];
}

export interface DepartmentTreeResponse {
  data: Department[];
}

@Injectable({
  providedIn: 'root'
})
export class DepartmentService {
  private apiUrl: string;

  constructor(private http: HttpClient, private apiConfig: ApiConfig) {
    this.apiUrl = this.apiConfig.buildUrl('departments');
  }

  /**
   * 获取部门列表
   */
  getDepartments(): Observable<DepartmentListResponse> {
    const url = this.apiUrl;
    return this.http.get<DepartmentListResponse>(url);
  }

  /**
   * 获取部门树形结构
   */
  getDepartmentTree(): Observable<DepartmentTreeResponse> {
    const url = this.apiUrl;
    return this.http.get<DepartmentTreeResponse>(url);
  }

  /**
   * 获取部门详情
   */
  getDepartment(id: number): Observable<DepartmentResponse> {
    const url = this.apiUrl + '/' + id;
    return this.http.get<DepartmentResponse>(url);
  }

  /**
   * 创建部门
   */
  createDepartment(department: CreateDepartmentRequest): Observable<DepartmentResponse> {
    const url = this.apiUrl;
    return this.http.post<DepartmentResponse>(url, department);
  }

  /**
   * 更新部门
   */
  updateDepartment(id: number, department: UpdateDepartmentRequest): Observable<DepartmentResponse> {
    const url = this.apiUrl + '/' + id;
    return this.http.put<DepartmentResponse>(url, department);
  }

  /**
   * 删除部门
   */
  deleteDepartment(id: number): Observable<{ message: string }> {
    const url = this.apiUrl + '/' + id;
    return this.http.delete<{ message: string }>(url);
  }
}

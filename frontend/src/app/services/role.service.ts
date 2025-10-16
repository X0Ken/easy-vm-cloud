import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ApiConfig } from '../config/api.config';

export interface Role {
  id: number;
  name: string;
  description: string;
  is_active: boolean;
  permissions: string[];
}

export interface PaginationInfo {
  current_page: number;
  per_page: number;
  total: number;
  total_pages: number;
  has_next: boolean;
  has_prev: boolean;
}

export interface PaginatedResponse<T> {
  data: T[];
  pagination: PaginationInfo;
}

export interface RolesResponse extends PaginatedResponse<Role> {}

export interface RoleResponse {
  role: Role;
}

export interface CreateRoleRequest {
  name: string;
  description: string;
}

export interface UpdateRoleRequest {
  name?: string;
  description?: string;
  is_active?: boolean;
}

export interface AssignPermissionRequest {
  permission_id: number;
}

@Injectable({
  providedIn: 'root'
})
export class RoleService {
  private apiUrl: string;
  constructor(private http: HttpClient, private apiConfig: ApiConfig) {
    this.apiUrl = this.apiConfig.buildUrl('roles');
  }

  getRoles(page: number = 1, perPage: number = 20): Observable<RolesResponse> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());
    
    return this.http.get<RolesResponse>(`${this.apiUrl}`, { params });
  }

  getRole(id: number): Observable<RoleResponse> {
    return this.http.get<RoleResponse>(`${this.apiUrl}/${id}`);
  }

  createRole(roleData: CreateRoleRequest): Observable<any> {
    return this.http.post(`${this.apiUrl}`, roleData);
  }

  updateRole(id: number, roleData: UpdateRoleRequest): Observable<any> {
    return this.http.put(`${this.apiUrl}/${id}`, roleData);
  }

  deleteRole(id: number): Observable<any> {
    return this.http.delete(`${this.apiUrl}/${id}`);
  }

  assignPermission(roleId: number, permissionId: number): Observable<any> {
    return this.http.post(`${this.apiUrl}/${roleId}/permissions`, { permission_id: permissionId });
  }

  removePermission(roleId: number, permissionId: number): Observable<any> {
    return this.http.delete(`${this.apiUrl}/${roleId}/permissions`, { 
      body: { permission_id: permissionId } 
    });
  }
}

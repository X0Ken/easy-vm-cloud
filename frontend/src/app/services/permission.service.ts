import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ApiConfig } from '../config/api.config';

export interface Permission {
  id: number;
  name: string;
  description: string;
  resource: string;
  action: string;
  is_active: boolean;
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

export interface PermissionsResponse extends PaginatedResponse<Permission> {}

export interface PermissionResponse {
  permission: Permission;
}

export interface CreatePermissionRequest {
  name: string;
  description: string;
  resource: string;
  action: string;
}

export interface UpdatePermissionRequest {
  name?: string;
  description?: string;
  resource?: string;
  action?: string;
  is_active?: boolean;
}

@Injectable({
  providedIn: 'root'
})
export class PermissionService {
  private apiUrl: string;
  constructor(private http: HttpClient, private apiConfig: ApiConfig) {
    this.apiUrl = this.apiConfig.buildUrl('permissions');
  }

  getPermissions(page: number = 1, perPage: number = 20): Observable<PermissionsResponse> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());
    
    return this.http.get<PermissionsResponse>(`${this.apiUrl}`, { params });
  }

  getPermission(id: number): Observable<PermissionResponse> {
    return this.http.get<PermissionResponse>(`${this.apiUrl}/${id}`);
  }

  createPermission(permissionData: CreatePermissionRequest): Observable<any> {
    return this.http.post(`${this.apiUrl}`, permissionData);
  }

  updatePermission(id: number, permissionData: UpdatePermissionRequest): Observable<any> {
    return this.http.put(`${this.apiUrl}/${id}`, permissionData);
  }

  deletePermission(id: number): Observable<any> {
    return this.http.delete(`${this.apiUrl}/${id}`);
  }
}

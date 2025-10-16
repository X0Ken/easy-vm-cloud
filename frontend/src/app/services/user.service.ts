import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ApiConfig } from '../config/api.config';

export interface User {
  id: number;
  username: string;
  email: string;
  is_active: boolean;
  permissions: string[];
  roles: string[];
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

export interface UsersResponse extends PaginatedResponse<User> {}

export interface UserResponse {
  user: User;
}

export interface CreateUserRequest {
  username: string;
  email: string;
  password: string;
}

export interface UpdateUserRequest {
  email?: string;
  is_active?: boolean;
}

export interface AssignRoleRequest {
  role_id: number;
}

@Injectable({
  providedIn: 'root'
})
export class UserService {
  private apiUrl: string;
  constructor(private http: HttpClient, private apiConfig: ApiConfig) {
    this.apiUrl = this.apiConfig.buildUrl('users');
  }

  getUsers(page: number = 1, perPage: number = 20): Observable<UsersResponse> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());
    
    return this.http.get<UsersResponse>(`${this.apiUrl}`, { params });
  }

  getUser(id: number): Observable<UserResponse> {
    return this.http.get<UserResponse>(`${this.apiUrl}/${id}`);
  }

  createUser(userData: CreateUserRequest): Observable<any> {
    return this.http.post(`${this.apiUrl}`, userData);
  }

  updateUser(id: number, userData: UpdateUserRequest): Observable<any> {
    return this.http.put(`${this.apiUrl}/${id}`, userData);
  }

  deleteUser(id: number): Observable<any> {
    return this.http.delete(`${this.apiUrl}/${id}`);
  }

  assignRole(userId: number, roleId: number): Observable<any> {
    return this.http.post(`${this.apiUrl}/${userId}/roles`, { role_id: roleId });
  }
}

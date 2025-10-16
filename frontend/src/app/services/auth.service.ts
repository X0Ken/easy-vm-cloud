import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { BehaviorSubject, Observable } from 'rxjs';
import { tap } from 'rxjs/operators';
import { ApiConfig } from '../config/api.config';

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  auth?: {
    expires_in: number;
    token: string;
    token_type: string;
  };
  message?: string;
  error?: string;
}

export interface CurrentUserResponse {
  user: {
    id: number;
    username: string;
    email: string;
    is_active: boolean;
    roles: string[];
    permissions: string[];
  };
}

@Injectable({
  providedIn: 'root'
})
export class AuthService {
  private isAuthenticatedSubject = new BehaviorSubject<boolean>(false);
  private tokenSubject = new BehaviorSubject<string | null>(null);
  private refreshTimer: any = null;
  private apiUrl: string;
  public isAuthenticated$ = this.isAuthenticatedSubject.asObservable();
  public token$ = this.tokenSubject.asObservable();

  constructor(private http: HttpClient, private apiConfig: ApiConfig) {
    // 检查本地存储中是否有token和过期时间
    const savedToken = localStorage.getItem('auth_token');
    const savedExpiry = localStorage.getItem('auth_token_expiry');
    this.apiUrl = this.apiConfig.buildUrl('auth');
    if (savedToken && savedExpiry) {
      const expiryTime = parseInt(savedExpiry);
      const now = Date.now();
      
      // 检查token是否仍然有效
      if (now < expiryTime) {
        this.tokenSubject.next(savedToken);
        this.isAuthenticatedSubject.next(true);
        this.scheduleTokenRefresh(expiryTime);
      } else {
        // token已过期，清除存储
        this.clearTokenStorage();
      }
    }
  }

  login(username: string, password: string): Promise<boolean> {
    const loginData: LoginRequest = { username, password };

    return new Promise((resolve) => {
      this.http.post<LoginResponse>(`${this.apiUrl}/login`, loginData)
        .subscribe({
          next: (response) => {
            if (response.auth && response.auth.token) {
              // 计算token过期时间
              const expiryTime = Date.now() + (response.auth.expires_in * 1000);
              
              // 保存token和过期时间到本地存储
              localStorage.setItem('auth_token', response.auth.token);
              localStorage.setItem('auth_token_expiry', expiryTime.toString());
              
              this.tokenSubject.next(response.auth.token);
              this.isAuthenticatedSubject.next(true);
              
              // 安排token刷新
              this.scheduleTokenRefresh(expiryTime);
              
              resolve(true);
            } else {
              resolve(false);
            }
          },
          error: (error) => {
            console.error('登录请求失败:', error);
            resolve(false);
          }
        });
    });
  }

  logout(): void {
    this.clearTokenStorage();
    this.clearRefreshTimer();
    this.tokenSubject.next(null);
    this.isAuthenticatedSubject.next(false);
  }

  getToken(): string | null {
    return this.tokenSubject.value;
  }

  isAuthenticated(): boolean {
    return this.isAuthenticatedSubject.value;
  }

  getCurrentUser(): Observable<CurrentUserResponse> {
    return this.http.get<CurrentUserResponse>(`${this.apiUrl}/me`);
  }

  hasPermission(permission: string): boolean {
    // 这里需要从当前用户信息中检查权限
    // 暂时返回true，实际应该检查用户权限
    return true;
  }

  /**
   * 安排token刷新
   * 当剩余时间少于一半时刷新token
   */
  private scheduleTokenRefresh(expiryTime: number): void {
    this.clearRefreshTimer();
    
    const now = Date.now();
    const totalTime = expiryTime - now;
    const halfTime = totalTime / 2;
    
    // 如果已经过了一半时间，立即刷新
    if (halfTime <= 0) {
      this.refreshToken();
      return;
    }
    
    // 在一半时间后刷新token
    this.refreshTimer = setTimeout(() => {
      this.refreshToken();
    }, halfTime);
  }

  /**
   * 刷新token
   */
  private refreshToken(): void {
    console.log('开始刷新token...');
    
    const currentToken = this.getToken();
    if (!currentToken) {
      console.log('没有当前token，无法刷新');
      return;
    }

    // 调用refresh接口获取新token
    this.http.post<LoginResponse>(`${this.apiUrl}/refresh`, {})
      .subscribe({
        next: (response) => {
          if (response.auth && response.auth.token) {
            console.log('Token刷新成功');
            
            // 计算新token过期时间
            const expiryTime = Date.now() + (response.auth.expires_in * 1000);
            
            // 更新token和过期时间
            localStorage.setItem('auth_token', response.auth.token);
            localStorage.setItem('auth_token_expiry', expiryTime.toString());
            
            this.tokenSubject.next(response.auth.token);
            this.isAuthenticatedSubject.next(true);
            
            // 安排下次刷新
            this.scheduleTokenRefresh(expiryTime);
          } else {
            console.log('刷新token失败，响应格式不正确');
            this.logout();
          }
        },
        error: (error) => {
          console.log('Token刷新失败，需要重新登录:', error);
          this.logout();
        }
      });
  }

  /**
   * 清除刷新定时器
   */
  private clearRefreshTimer(): void {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }
  }

  /**
   * 清除token存储
   */
  private clearTokenStorage(): void {
    localStorage.removeItem('auth_token');
    localStorage.removeItem('auth_token_expiry');
  }

  /**
   * 检查token是否即将过期（剩余时间少于5分钟）
   */
  isTokenExpiringSoon(): boolean {
    const savedExpiry = localStorage.getItem('auth_token_expiry');
    if (!savedExpiry) {
      return false;
    }
    
    const expiryTime = parseInt(savedExpiry);
    const now = Date.now();
    const fiveMinutes = 5 * 60 * 1000; // 5分钟
    
    return (expiryTime - now) < fiveMinutes;
  }

  /**
   * 获取token剩余时间（秒）
   */
  getTokenRemainingTime(): number {
    const savedExpiry = localStorage.getItem('auth_token_expiry');
    if (!savedExpiry) {
      return 0;
    }
    
    const expiryTime = parseInt(savedExpiry);
    const now = Date.now();
    const remaining = Math.max(0, expiryTime - now);
    
    return Math.floor(remaining / 1000);
  }
}

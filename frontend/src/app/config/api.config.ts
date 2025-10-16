import { Injectable } from '@angular/core';

/**
 * API配置服务
 * 提供基础API配置
 */
@Injectable({
  providedIn: 'root'
})
export class ApiConfig {
  /**
   * 基础API URL
   * 由于启用了代理，设置为空字符串
   */
  get baseUrl(): string {
    return '/api';
  }

  /**
   * 构建完整的API URL
   * @param endpoint 端点路径
   * @returns 完整的API URL
   */
  buildUrl(endpoint: string): string {
    // 确保endpoint以/开头
    if (!endpoint.startsWith('/')) {
      endpoint = '/' + endpoint;
    }
    return `${this.baseUrl}${endpoint}`;
  }
}


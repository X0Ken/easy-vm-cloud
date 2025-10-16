import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ApiConfig } from '../config/api.config';

export interface Node {
  id: string;
  hostname: string;
  ip_address: string;
  status: 'online' | 'offline' | 'maintenance';
  hypervisor_type?: string;
  hypervisor_version?: string;
  cpu_cores?: number | null;
  cpu_threads?: number | null;
  memory_total?: number | null;
  disk_total?: number | null;
  metadata?: any;
  last_heartbeat?: string | null;
  created_at: string;
  updated_at: string;
}

export interface NodeListResponse {
  data: Node[];
  total: number;
  page: number;
  page_size: number;
}

export interface NodeDetailResponse {
  data: Node;
}

export interface NodeStatsResponse {
  total_nodes: number;
  online_nodes: number;
  offline_nodes: number;
  total_cpu: number;
  total_memory: number;
  total_disk: number;
}

@Injectable({
  providedIn: 'root'
})
export class NodeService {
  constructor(
    private http: HttpClient,
    private apiConfig: ApiConfig
  ) {}

  /**
   * 创建节点
   */
  createNode(data: { hostname: string; ip_address: string }): Observable<NodeDetailResponse> {
    return this.http.post<NodeDetailResponse>(this.apiConfig.buildUrl('/nodes'), data);
  }

  /**
   * 获取节点列表
   */
  getNodes(params?: {
    page?: number;
    page_size?: number;
    status?: string;
    search?: string;
  }): Observable<NodeListResponse> {
    let httpParams = new HttpParams();
    
    if (params) {
      if (params.page) {
        httpParams = httpParams.set('page', params.page.toString());
      }
      if (params.page_size) {
        httpParams = httpParams.set('page_size', params.page_size.toString());
      }
      if (params.status) {
        httpParams = httpParams.set('status', params.status);
      }
      if (params.search) {
        httpParams = httpParams.set('search', params.search);
      }
    }

    return this.http.get<NodeListResponse>(this.apiConfig.buildUrl('/nodes'), { params: httpParams });
  }

  /**
   * 获取节点详情
   */
  getNode(id: string): Observable<NodeDetailResponse> {
    return this.http.get<NodeDetailResponse>(this.apiConfig.buildUrl(`/nodes/${id}`));
  }

  /**
   * 获取节点统计信息
   */
  getNodeStats(): Observable<NodeStatsResponse> {
    return this.http.get<NodeStatsResponse>(this.apiConfig.buildUrl('/nodes/stats'));
  }

  /**
   * 刷新节点状态
   */
  refreshNode(id: string): Observable<any> {
    return this.http.post(this.apiConfig.buildUrl(`/nodes/${id}/refresh`), {});
  }

  /**
   * 获取节点指标
   */
  getNodeMetrics(id: string, timeRange?: string): Observable<any> {
    let httpParams = new HttpParams();
    if (timeRange) {
      httpParams = httpParams.set('time_range', timeRange);
    }
    
    return this.http.get(this.apiConfig.buildUrl(`/nodes/${id}/metrics`), { params: httpParams });
  }

  /**
   * 获取节点虚拟机列表
   */
  getNodeVms(id: string, params?: {
    page?: number;
    page_size?: number;
    status?: string;
  }): Observable<any> {
    let httpParams = new HttpParams();
    
    if (params) {
      if (params.page) {
        httpParams = httpParams.set('page', params.page.toString());
      }
      if (params.page_size) {
        httpParams = httpParams.set('page_size', params.page_size.toString());
      }
      if (params.status) {
        httpParams = httpParams.set('status', params.status);
      }
    }

    return this.http.get(this.apiConfig.buildUrl(`/nodes/${id}/vms`), { params: httpParams });
  }

  /**
   * 获取节点存储信息
   */
  getNodeStorage(id: string): Observable<any> {
    return this.http.get(this.apiConfig.buildUrl(`/nodes/${id}/storage`));
  }

  /**
   * 获取节点网络信息
   */
  getNodeNetwork(id: string): Observable<any> {
    return this.http.get(this.apiConfig.buildUrl(`/nodes/${id}/network`));
  }

  /**
   * 设置节点维护模式
   */
  setMaintenanceMode(id: string, enabled: boolean): Observable<any> {
    return this.http.post(this.apiConfig.buildUrl(`/nodes/${id}/maintenance`), { enabled });
  }

  /**
   * 删除节点
   */
  deleteNode(id: string): Observable<any> {
    return this.http.delete(this.apiConfig.buildUrl(`/nodes/${id}`));
  }

  /**
   * 更新节点信息
   */
  updateNode(id: string, data: Partial<Node>): Observable<NodeDetailResponse> {
    return this.http.put<NodeDetailResponse>(this.apiConfig.buildUrl(`/nodes/${id}`), data);
  }

  /**
   * 获取节点日志
   */
  getNodeLogs(id: string, params?: {
    page?: number;
    page_size?: number;
    level?: string;
    start_time?: string;
    end_time?: string;
  }): Observable<any> {
    let httpParams = new HttpParams();
    
    if (params) {
      if (params.page) {
        httpParams = httpParams.set('page', params.page.toString());
      }
      if (params.page_size) {
        httpParams = httpParams.set('page_size', params.page_size.toString());
      }
      if (params.level) {
        httpParams = httpParams.set('level', params.level);
      }
      if (params.start_time) {
        httpParams = httpParams.set('start_time', params.start_time);
      }
      if (params.end_time) {
        httpParams = httpParams.set('end_time', params.end_time);
      }
    }

    return this.http.get(this.apiConfig.buildUrl(`/nodes/${id}/logs`), { params: httpParams });
  }

  /**
   * 获取节点健康检查
   */
  getNodeHealth(id: string): Observable<any> {
    return this.http.get(this.apiConfig.buildUrl(`/nodes/${id}/health`));
  }

  /**
   * 执行节点命令
   */
  executeNodeCommand(id: string, command: string, args?: string[]): Observable<any> {
    return this.http.post(this.apiConfig.buildUrl(`/nodes/${id}/execute`), {
      command,
      args: args || []
    });
  }
}

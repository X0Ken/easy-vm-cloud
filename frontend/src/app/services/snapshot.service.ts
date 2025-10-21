import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ApiConfig } from '../config/api.config';

/**
 * 快照响应接口
 */
export interface SnapshotResponse {
  id: string;
  name: string;
  volume_id: string;
  volume_name?: string;
  status: string;
  size_gb?: number;
  snapshot_tag?: string;
  description?: string;
  metadata?: any;
  created_at: string;
  updated_at: string;
}

/**
 * 快照列表响应接口
 */
export interface SnapshotListResponse {
  snapshots: SnapshotResponse[];
  total: number;
  page: number;
  page_size: number;
}

/**
 * 创建快照DTO
 */
export interface CreateSnapshotDto {
  name: string;
  volume_id: string;
  description?: string;
  metadata?: any;
}

/**
 * 更新快照DTO（仅允许更新名称和描述）
 */
export interface UpdateSnapshotDto {
  name?: string;
  description?: string;
}

/**
 * 快照查询参数
 */
export interface SnapshotQueryParams {
  page?: number;
  page_size?: number;
  volume_id?: string;
  status?: string;
}

/**
 * 快照恢复响应
 */
export interface RestoreSnapshotResponse {
  message: string;
}

/**
 * 快照服务
 * 提供存储卷快照相关的API操作
 */
@Injectable({
  providedIn: 'root',
})
export class SnapshotService {
  private apiUrl: string;

  constructor(
    private http: HttpClient,
    private apiConfig: ApiConfig,
  ) {
    this.apiUrl = this.apiConfig.buildUrl('/storage/snapshots');
  }

  /**
   * 创建快照
   * @param dto 创建快照DTO
   * @returns Observable<SnapshotResponse>
   */
  createSnapshot(dto: CreateSnapshotDto): Observable<SnapshotResponse> {
    return this.http.post<SnapshotResponse>(this.apiUrl, dto);
  }

  /**
   * 获取快照列表
   * @param params 查询参数
   * @returns Observable<SnapshotListResponse>
   */
  listSnapshots(params?: SnapshotQueryParams): Observable<SnapshotListResponse> {
    let httpParams = new HttpParams();

    if (params) {
      if (params.page) {
        httpParams = httpParams.set('page', params.page.toString());
      }
      if (params.page_size) {
        httpParams = httpParams.set('page_size', params.page_size.toString());
      }
      if (params.volume_id) {
        httpParams = httpParams.set('volume_id', params.volume_id);
      }
      if (params.status) {
        httpParams = httpParams.set('status', params.status);
      }
    }

    return this.http.get<SnapshotListResponse>(this.apiUrl, { params: httpParams });
  }

  /**
   * 获取单个快照详情
   * @param snapshotId 快照ID
   * @returns Observable<SnapshotResponse>
   */
  getSnapshot(snapshotId: string): Observable<SnapshotResponse> {
    return this.http.get<SnapshotResponse>(`${this.apiUrl}/${snapshotId}`);
  }

  /**
   * 更新快照（仅允许更新名称和描述）
   * @param snapshotId 快照ID
   * @param dto 更新快照DTO
   * @returns Observable<SnapshotResponse>
   */
  updateSnapshot(snapshotId: string, dto: UpdateSnapshotDto): Observable<SnapshotResponse> {
    return this.http.put<SnapshotResponse>(`${this.apiUrl}/${snapshotId}`, dto);
  }

  /**
   * 删除快照
   * @param snapshotId 快照ID
   * @returns Observable<void>
   */
  deleteSnapshot(snapshotId: string): Observable<void> {
    return this.http.delete<void>(`${this.apiUrl}/${snapshotId}`);
  }

  /**
   * 恢复快照
   * @param snapshotId 快照ID
   * @returns Observable<RestoreSnapshotResponse>
   */
  restoreSnapshot(snapshotId: string): Observable<RestoreSnapshotResponse> {
    return this.http.post<RestoreSnapshotResponse>(`${this.apiUrl}/${snapshotId}/restore`, {});
  }

  /**
   * 根据存储卷ID获取快照列表
   * @param volumeId 存储卷ID
   * @param page 页码
   * @param pageSize 每页大小
   * @returns Observable<SnapshotListResponse>
   */
  getSnapshotsByVolume(
    volumeId: string,
    page: number = 1,
    pageSize: number = 20,
  ): Observable<SnapshotListResponse> {
    return this.listSnapshots({
      volume_id: volumeId,
      page,
      page_size: pageSize,
    });
  }
}

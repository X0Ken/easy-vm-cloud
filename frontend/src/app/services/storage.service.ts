import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { ApiConfig } from '../config/api.config';

// 存储池数据模型
export interface StoragePool {
  id: number;
  name: string;
  type: 'lvm' | 'nfs' | 'ceph' | 'iscsi';
  status: 'active' | 'inactive' | 'error';
  total_size_gb: number;
  used_size_gb: number;
  available_size_gb: number;
  node_id?: string;
  node_name?: string;
  config?: any;  // 存储池配置
  metadata?: any;  // 元数据
  created_at: string;
  updated_at: string;
}

// 存储卷数据模型
export interface StorageVolume {
  id: number;
  name: string;
  pool_id: number;
  pool_name: string;
  size_gb: number;
  volume_type: 'qcow2' | 'raw';
  status: 'available' | 'in_use' | 'creating' | 'deleting' | 'error';
  node_id?: string;  // 从存储池获取
  node_name?: string;  // 从存储池获取
  vm_id?: number;
  vm_name?: string;
  metadata?: any;  // 包含source等元数据信息
  created_at: string;
  updated_at: string;
}

// 创建存储池请求
export interface CreateStoragePoolRequest {
  name: string;
  pool_type: 'lvm' | 'nfs' | 'ceph' | 'iscsi';
  capacity_gb?: number;
  node_id?: string;
  config: {
    // LVM 配置
    volume_group?: string;
    // NFS 配置
    nfs_server?: string;
    nfs_path?: string;
    // Ceph 配置
    ceph_pool?: string;
    ceph_monitors?: string[];
    // iSCSI 配置
    iscsi_target?: string;
    iscsi_portal?: string;
  };
  metadata?: any;
}

// 更新存储池请求
export interface UpdateStoragePoolRequest {
  name?: string;
  total_size_gb?: number;
  node_id?: string;
  config?: any;
}

// 创建存储卷请求
export interface CreateStorageVolumeRequest {
  name: string;
  pool_id: number;
  size_gb: number;
  volume_type: 'qcow2' | 'raw';
  source?: string | null;  // 外部URL，用于下载初始数据
}

// 更新存储卷请求
export interface UpdateStorageVolumeRequest {
  name?: string;
  size_gb?: number;
}

// 分页响应
export interface PaginatedResponse<T> {
  data: T[];
  pagination: {
    current_page: number;
    per_page: number;
    total: number;
    total_pages: number;
    has_next: boolean;
    has_prev: boolean;
  };
}

// 节点信息
export interface Node {
  id: number;
  hostname: string;
  ip: string;
  status: string;
}

@Injectable({
  providedIn: 'root'
})
export class StorageService {
  constructor(
    private http: HttpClient,
    private apiConfig: ApiConfig
  ) {}

  // 转换存储池API响应数据
  private transformStoragePoolsResponse(response: any): PaginatedResponse<StoragePool> {
    const transformedPools = response.pools.map((pool: any) => ({
      id: pool.id,
      name: pool.name,
      type: pool.pool_type,
      status: pool.status,
      total_size_gb: pool.capacity_gb,
      used_size_gb: pool.allocated_gb,
      available_size_gb: pool.available_gb,
      node_id: pool.node_id,
      node_name: pool.node_name,
      config: pool.config,
      metadata: pool.metadata,
      created_at: pool.created_at,
      updated_at: pool.updated_at
    }));

    return {
      data: transformedPools,
      pagination: {
        current_page: response.page || 1,
        per_page: response.page_size || 20,
        total: response.total || 0,
        total_pages: Math.ceil((response.total || 0) / (response.page_size || 20)),
        has_next: (response.page || 1) < Math.ceil((response.total || 0) / (response.page_size || 20)),
        has_prev: (response.page || 1) > 1
      }
    };
  }

  // 转换存储卷API响应数据
  private transformStorageVolumesResponse(response: any): PaginatedResponse<StorageVolume> {
    const transformedVolumes = (response.volumes || []).map((volume: any) => ({
      id: volume.id,
      name: volume.name,
      pool_id: volume.pool_id || 1,
      pool_name: volume.pool_name || 'Unknown',
      size_gb: volume.size_gb || volume.capacity_gb || 0,
      volume_type: volume.volume_type || 'qcow2',
      status: volume.status || 'available',
      node_id: volume.node_id,  // 从存储池获取
      node_name: volume.node_name,  // 从存储池获取
      vm_id: volume.vm_id,
      vm_name: volume.vm_name,
      metadata: volume.metadata,
      created_at: volume.created_at,
      updated_at: volume.updated_at
    }));

    return {
      data: transformedVolumes,
      pagination: {
        current_page: response.page || 1,
        per_page: response.page_size || 20,
        total: response.total || 0,
        total_pages: Math.ceil((response.total || 0) / (response.page_size || 20)),
        has_next: (response.page || 1) < Math.ceil((response.total || 0) / (response.page_size || 20)),
        has_prev: (response.page || 1) > 1
      }
    };
  }

  // 获取存储池列表
  getStoragePools(page: number = 1, perPage: number = 20): Observable<PaginatedResponse<StoragePool>> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());
    
    return this.http.get<any>(this.apiConfig.buildUrl('/storage/pools'), { params }).pipe(
      map(response => this.transformStoragePoolsResponse(response))
    );
  }

  // 获取存储池详情
  getStoragePool(id: number): Observable<StoragePool> {
    return this.http.get<StoragePool>(this.apiConfig.buildUrl(`/storage/pools/${id}`));
  }

  // 创建存储池
  createStoragePool(poolData: CreateStoragePoolRequest): Observable<StoragePool> {
    return this.http.post<StoragePool>(this.apiConfig.buildUrl('/storage/pools'), poolData);
  }

  // 更新存储池
  updateStoragePool(id: number, poolData: UpdateStoragePoolRequest): Observable<StoragePool> {
    return this.http.put<StoragePool>(this.apiConfig.buildUrl(`/storage/pools/${id}`), poolData);
  }

  // 删除存储池
  deleteStoragePool(id: number): Observable<void> {
    return this.http.delete<void>(this.apiConfig.buildUrl(`/storage/pools/${id}`));
  }

  // 获取存储卷列表
  getStorageVolumes(page: number = 1, perPage: number = 20): Observable<PaginatedResponse<StorageVolume>> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());
    
    return this.http.get<any>(this.apiConfig.buildUrl('/storage/volumes'), { params }).pipe(
      map(response => this.transformStorageVolumesResponse(response))
    );
  }

  // 获取存储卷详情
  getStorageVolume(id: number): Observable<StorageVolume> {
    return this.http.get<StorageVolume>(this.apiConfig.buildUrl(`/storage/volumes/${id}`));
  }

  // 创建存储卷
  createStorageVolume(volumeData: CreateStorageVolumeRequest): Observable<StorageVolume> {
    return this.http.post<StorageVolume>(this.apiConfig.buildUrl('/storage/volumes'), volumeData);
  }

  // 更新存储卷
  updateStorageVolume(id: number, volumeData: UpdateStorageVolumeRequest): Observable<StorageVolume> {
    return this.http.put<StorageVolume>(this.apiConfig.buildUrl(`/storage/volumes/${id}`), volumeData);
  }

  // 删除存储卷
  deleteStorageVolume(id: number): Observable<void> {
    return this.http.delete<void>(this.apiConfig.buildUrl(`/storage/volumes/${id}`));
  }

  // 克隆存储卷
  cloneVolume(id: number, newName: string): Observable<StorageVolume> {
    return this.http.post<StorageVolume>(this.apiConfig.buildUrl(`/storage/volumes/${id}/clone`), {
      source_volume_id: id.toString(),
      target_name: newName
    });
  }

  // 创建存储卷快照
  createSnapshot(id: number, snapshotName: string): Observable<StorageVolume> {
    return this.http.post<StorageVolume>(this.apiConfig.buildUrl(`/storage/volumes/${id}/snapshot`), {
      name: snapshotName
    });
  }

  // 恢复存储卷快照
  restoreSnapshot(volumeId: number, snapshotId: number): Observable<void> {
    return this.http.post<void>(this.apiConfig.buildUrl(`/storage/volumes/${volumeId}/restore`), {
      snapshot_id: snapshotId
    });
  }

  // 扩容存储卷
  resizeVolume(id: number, newSizeGb: number): Observable<StorageVolume> {
    return this.http.post<StorageVolume>(this.apiConfig.buildUrl(`/storage/volumes/${id}/resize`), {
      size_gb: newSizeGb
    });
  }

  // 获取存储池使用统计
  getPoolUsage(id: number): Observable<{
    total_size_gb: number;
    used_size_gb: number;
    available_size_gb: number;
    usage_percentage: number;
  }> {
    return this.http.get<any>(this.apiConfig.buildUrl(`/storage/pools/${id}/usage`));
  }

  // 获取存储卷性能指标
  getVolumeMetrics(id: number, timeRange: string = '1h'): Observable<any> {
    const params = new HttpParams().set('time_range', timeRange);
    return this.http.get<any>(this.apiConfig.buildUrl(`/storage/volumes/${id}/metrics`), { params });
  }

  // 获取节点列表
  getNodes(): Observable<Node[]> {
    return this.http.get<any>(this.apiConfig.buildUrl('/nodes')).pipe(
      map(response => {
        // 后端返回的是 NodeListResponse 格式
        const nodes = response.nodes || [];
        // 转换字段名称以匹配前端接口
        return nodes.map((node: any) => ({
          id: node.id, // 后端返回的是字符串ID，前端期望数字
          hostname: node.hostname,
          ip: node.ip_address, // 后端字段名是 ip_address
          status: node.status,
          cpu_total: node.cpu_cores || 0,
          cpu_used: 0, // 后端没有提供已使用CPU信息
          mem_total: node.memory_total || 0,
          mem_used: 0, // 后端没有提供已使用内存信息
          disk_total: node.disk_total || 0,
          disk_used: 0 // 后端没有提供已使用磁盘信息
        }));
      })
    );
  }
}

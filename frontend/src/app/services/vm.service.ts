import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { ApiConfig } from '../config/api.config';

// 虚拟机数据模型
export interface VM {
  id: string;
  uuid: string;
  name: string;
  node_id: string;
  node_name: string;
  status: 'running' | 'stopped' | 'stopping' | 'paused' | 'error';
  vcpu: number;
  memory_mb: number;
  os_type: string; // 操作系统类型
  disk_size_gb: number;
  created_at: string;
  updated_at: string;
}

// 磁盘总线类型
export type DiskBusType = 'virtio' | 'scsi' | 'ide';

// 磁盘设备类型
export type DiskDeviceType = 'disk' | 'cdrom';

// 磁盘规格
export interface DiskSpec {
  volume_id: string;
  bus_type?: DiskBusType;      // 总线类型，默认为 virtio
  device_type?: DiskDeviceType; // 设备类型，默认为 disk
}

// 网络接口规格
export interface NetworkInterfaceSpec {
  network_id: string;
  mac_address?: string | null;
  ip_address?: string | null;
  model: string;
  bridge_name?: string | null;
}

// 创建虚拟机请求
export interface CreateVMRequest {
  name: string;
  node_id: string; // 后端期望字符串类型
  vcpu: number;
  memory_mb: number;
  os_type?: string; // 操作系统类型: linux, windows
  disks?: DiskSpec[];
  networks?: NetworkInterfaceSpec[];
}

// 更新虚拟机请求
export interface UpdateVMRequest {
  name?: string;
  vcpu?: number;
  memory_mb?: number;
  disk_size_gb?: number;
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
  cpu_total: number;
  cpu_used: number;
  mem_total: number;
  mem_used: number;
  disk_total: number;
  disk_used: number;
}

@Injectable({
  providedIn: 'root'
})
export class VmService {
  constructor(
    private http: HttpClient,
    private apiConfig: ApiConfig
  ) {}

  // 获取虚拟机列表
  getVMs(page: number = 1, perPage: number = 20): Observable<PaginatedResponse<VM>> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());

    return this.http.get<any>(this.apiConfig.buildUrl('/vms'), { params }).pipe(
      map(response => this.transformVMsResponse(response))
    );
  }

  // 转换虚拟机API响应数据
  private transformVMsResponse(response: any): PaginatedResponse<VM> {
    const transformedVMs = (response.vms || []).map((vm: any) => ({
      id: vm.id,
      uuid: vm.uuid || '',
      name: vm.name,
      node_id: vm.node_id || '',
      node_name: vm.node_name || 'Unknown', // 使用后端返回的节点名称
      status: vm.status,
      vcpu: vm.vcpu,
      memory_mb: vm.memory_mb,
      os_type: vm.os_type || 'linux', // 默认操作系统类型
      disk_size_gb: 0, // 后端没有提供磁盘大小，使用默认值
      created_at: vm.created_at,
      updated_at: vm.updated_at
    }));

    return {
      data: transformedVMs,
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

  // 获取虚拟机详情
  getVM(id: string): Observable<VM> {
    return this.http.get<VM>(this.apiConfig.buildUrl(`/vms/${id}`));
  }

  // 创建虚拟机
  createVM(vmData: CreateVMRequest): Observable<VM> {
    return this.http.post<VM>(this.apiConfig.buildUrl('/vms'), vmData);
  }

  // 更新虚拟机
  updateVM(id: string, vmData: UpdateVMRequest): Observable<VM> {
    return this.http.put<VM>(this.apiConfig.buildUrl(`/vms/${id}`), vmData);
  }

  // 删除虚拟机
  deleteVM(id: string): Observable<void> {
    return this.http.delete<void>(this.apiConfig.buildUrl(`/vms/${id}`));
  }

  // 启动虚拟机
  startVM(id: string): Observable<void> {
    return this.http.post<void>(this.apiConfig.buildUrl(`/vms/${id}/start`), {});
  }

  // 停止虚拟机
  stopVM(id: string): Observable<void> {
    return this.http.post<void>(this.apiConfig.buildUrl(`/vms/${id}/stop`), {});
  }

  // 重启虚拟机
  restartVM(id: string): Observable<void> {
    return this.http.post<void>(this.apiConfig.buildUrl(`/vms/${id}/restart`), {});
  }



  // 迁移虚拟机
  migrateVM(id: string, targetNodeId: string, live: boolean = false): Observable<void> {
    return this.http.post<void>(this.apiConfig.buildUrl(`/vms/${id}/migrate`), {
      target_node_id: targetNodeId,
      live: live,
    });
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
          status: node.status
        }));
      })
    );
  }

  // 获取虚拟机控制台URL
  getConsoleUrl(id: string): Observable<{ url: string }> {
    return this.http.get<{ url: string }>(this.apiConfig.buildUrl(`/vms/${id}/console`));
  }

  // 获取虚拟机性能指标
  getVMMetrics(id: string, timeRange: string = '1h'): Observable<any> {
    const params = new HttpParams().set('time_range', timeRange);
    return this.http.get<any>(this.apiConfig.buildUrl(`/vms/${id}/metrics`), { params });
  }

  // 获取虚拟机存储卷信息
  getVMVolumes(id: string): Observable<any[]> {
    return this.http.get<any[]>(this.apiConfig.buildUrl(`/vms/${id}/volumes`));
  }

  // 获取虚拟机网络信息
  getVMNetworks(id: string): Observable<any[]> {
    return this.http.get<any[]>(this.apiConfig.buildUrl(`/vms/${id}/networks`));
  }

  // 挂载存储卷到虚拟机
  attachVolume(vmId: string, volumeId: string, busType?: DiskBusType, deviceType?: DiskDeviceType): Observable<any> {
    return this.http.post<any>(this.apiConfig.buildUrl(`/vms/${vmId}/volumes/attach`), {
      volume_id: volumeId,
      bus_type: busType || 'virtio',
      device_type: deviceType || 'disk'
    });
  }

  // 从虚拟机移除存储卷
  detachVolume(vmId: string, volumeId: string): Observable<any> {
    return this.http.post<any>(this.apiConfig.buildUrl(`/vms/${vmId}/volumes/detach`), {
      volume_id: volumeId
    });
  }
}

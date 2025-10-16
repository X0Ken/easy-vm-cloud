import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { ApiConfig } from '../config/api.config';

// 网络数据模型
export interface Network {
  id: number;
  name: string;
  type: 'bridge' | 'ovs';
  cidr: string;
  gateway: string;
  vlan_id?: number;
  mtu: number;
  status: 'active' | 'inactive' | 'error';
  node_id: string;
  node_name: string;
  created_at: string;
  updated_at: string;
}

// 子网数据模型
export interface Subnet {
  id: number;
  name: string;
  network_id: number;
  network_name: string;
  cidr: string;
  gateway: string;
  vlan_id: number;
  ip_pool_start: string;
  ip_pool_end: string;
  available_ips: number;
  total_ips: number;
  status: 'active' | 'inactive' | 'error';
  created_at: string;
  updated_at: string;
}

// IP 分配记录
export interface IPAllocation {
  id: number;
  ip_address: string;
  subnet_id: number;
  subnet_name: string;
  vm_id?: number;
  vm_name?: string;
  status: 'allocated' | 'available' | 'reserved';
  allocated_at?: string;
  expires_at?: string;
}

// 创建网络请求
export interface CreateNetworkRequest {
  name: string;
  network_type: 'bridge' | 'ovs';
  cidr: string;
  gateway: string;
  vlan_id?: number;
  mtu: number;
  node_id: string;
  config?: {
    // Bridge 配置
    bridge_name?: string;
    // OVS 配置
    ovs_bridge?: string;
    controller_url?: string;
  };
}

// 更新网络请求
export interface UpdateNetworkRequest {
  name?: string;
  cidr?: string;
  gateway?: string;
  vlan_id?: number;
  mtu?: number;
  config?: any;
}

// 创建子网请求
export interface CreateSubnetRequest {
  name: string;
  network_id: number;
  cidr: string;
  gateway: string;
  vlan_id: number;
  ip_pool_start: string;
  ip_pool_end: string;
}

// 更新子网请求
export interface UpdateSubnetRequest {
  name?: string;
  cidr?: string;
  gateway?: string;
  vlan_id?: number;
  ip_pool_start?: string;
  ip_pool_end?: string;
}

// 分配IP请求
export interface AllocateIPRequest {
  subnet_id: number;
  vm_id?: number;
  preferred_ip?: string;
  expires_at?: string;
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
export class NetworkService {
  constructor(
    private http: HttpClient,
    private apiConfig: ApiConfig
  ) {}

  // 转换网络API响应数据
  private transformNetworksResponse(response: any): PaginatedResponse<Network> {
    const transformedNetworks = (response.networks || []).map((network: any) => ({
      id: network.id,
      name: network.name,
      type: network.network_type || network.type || 'bridge',
      cidr: network.cidr || '',
      gateway: network.gateway || '',
      vlan_id: network.vlan_id,
      mtu: network.mtu || 1500,
      status: network.status || 'active',
      node_id: network.node_id || 1,
      node_name: network.node_name || 'Unknown',
      created_at: network.created_at,
      updated_at: network.updated_at
    }));

    return {
      data: transformedNetworks,
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

  // 转换子网API响应数据
  private transformSubnetsResponse(response: any): PaginatedResponse<Subnet> {
    const transformedSubnets = (response.subnets || []).map((subnet: any) => ({
      id: subnet.id,
      name: subnet.name,
      network_id: subnet.network_id || 1,
      network_name: subnet.network_name || 'Unknown',
      cidr: subnet.cidr || '',
      gateway: subnet.gateway || '',
      vlan_id: subnet.vlan_id || 0,
      ip_pool_start: subnet.ip_pool_start || '',
      ip_pool_end: subnet.ip_pool_end || '',
      available_ips: subnet.available_ips || 0,
      total_ips: subnet.total_ips || 0,
      status: subnet.status || 'active',
      created_at: subnet.created_at,
      updated_at: subnet.updated_at
    }));

    return {
      data: transformedSubnets,
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

  // 转换IP分配API响应数据 - 根据Rust后端响应格式
  private transformIPAllocationsResponse(response: any): PaginatedResponse<IPAllocation> {
    // 根据Rust后端的IpAllocationListResponse结构
    const allocations = response.allocations || [];
    const transformedAllocations = allocations.map((allocation: any) => ({
      id: allocation.id,
      ip_address: allocation.ip_address,
      subnet_id: allocation.network_id, // 后端使用network_id
      subnet_name: 'Network', // 后端没有提供网络名称，使用默认值
      vm_id: allocation.vm_id,
      vm_name: allocation.vm_name || 'Unknown', // 现在后端提供VM名称
      status: allocation.status,
      allocated_at: allocation.allocated_at,
      expires_at: allocation.expires_at // 后端没有过期时间字段
    }));

    return {
      data: transformedAllocations,
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

  // 获取网络列表
  getNetworks(page: number = 1, perPage: number = 20): Observable<PaginatedResponse<Network>> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());
    
    return this.http.get<any>(this.apiConfig.buildUrl('/networks'), { params }).pipe(
      map(response => this.transformNetworksResponse(response))
    );
  }

  // 获取网络详情
  getNetwork(id: number): Observable<Network> {
    return this.http.get<Network>(this.apiConfig.buildUrl(`/networks/${id}`));
  }

  // 创建网络
  createNetwork(networkData: CreateNetworkRequest): Observable<Network> {
    return this.http.post<Network>(this.apiConfig.buildUrl('/networks'), networkData);
  }

  // 更新网络
  updateNetwork(id: number, networkData: UpdateNetworkRequest): Observable<Network> {
    return this.http.put<Network>(this.apiConfig.buildUrl(`/networks/${id}`), networkData);
  }

  // 删除网络
  deleteNetwork(id: number): Observable<void> {
    return this.http.delete<void>(this.apiConfig.buildUrl(`/networks/${id}`));
  }

  // 获取子网列表
  getSubnets(page: number = 1, perPage: number = 20): Observable<PaginatedResponse<Subnet>> {
    const params = new HttpParams()
      .set('page', page.toString())
      .set('per_page', perPage.toString());
    
    return this.http.get<any>(this.apiConfig.buildUrl('/networks/subnets'), { params }).pipe(
      map(response => this.transformSubnetsResponse(response))
    );
  }

  // 获取子网详情
  getSubnet(id: number): Observable<Subnet> {
    return this.http.get<Subnet>(this.apiConfig.buildUrl(`/networks/subnets/${id}`));
  }

  // 创建子网
  createSubnet(subnetData: CreateSubnetRequest): Observable<Subnet> {
    return this.http.post<Subnet>(this.apiConfig.buildUrl('/networks/subnets'), subnetData);
  }

  // 更新子网
  updateSubnet(id: number, subnetData: UpdateSubnetRequest): Observable<Subnet> {
    return this.http.put<Subnet>(this.apiConfig.buildUrl(`/networks/subnets/${id}`), subnetData);
  }

  // 删除子网
  deleteSubnet(id: number): Observable<void> {
    return this.http.delete<void>(this.apiConfig.buildUrl(`/networks/subnets/${id}`));
  }

  // 获取IP分配列表 - 需要指定网络ID
  getIPAllocations(networkId: string, page: number = 1, perPage: number = 20, status?: string): Observable<PaginatedResponse<IPAllocation>> {
    let params = new HttpParams()
      .set('page', page.toString())
      .set('page_size', perPage.toString());
    
    if (status) {
      params = params.set('status', status);
    }
    
    return this.http.get<any>(this.apiConfig.buildUrl(`/networks/${networkId}/ips`), { params }).pipe(
      map(response => this.transformIPAllocationsResponse(response))
    );
  }

  // 分配IP地址 - 根据后端逻辑，只需要网络ID
  allocateIP(networkId: string): Observable<IPAllocation> {
    return this.http.post<IPAllocation>(this.apiConfig.buildUrl(`/networks/${networkId}/allocate-ip`), {});
  }

  // 更新IP分配的VM ID - 当VM创建成功后调用
  updateIPVMId(ipAllocationId: string, vmId: string): Observable<IPAllocation> {
    return this.http.put<IPAllocation>(this.apiConfig.buildUrl(`/networks/ip-allocations/${ipAllocationId}/vm`), {
      vm_id: vmId
    });
  }

  // 释放IP地址 - 根据后端逻辑，需要网络ID和VM ID
  releaseIP(networkId: string, vmId: string): Observable<void> {
    return this.http.post<void>(this.apiConfig.buildUrl(`/networks/${networkId}/release-ip`), {
      vm_id: vmId
    });
  }

  // 获取子网可用IP列表
  getAvailableIPs(subnetId: number): Observable<string[]> {
    return this.http.get<string[]>(this.apiConfig.buildUrl(`/networks/subnets/${subnetId}/available-ips`));
  }

  // 获取子网使用统计
  getSubnetUsage(subnetId: number): Observable<{
    total_ips: number;
    allocated_ips: number;
    available_ips: number;
    usage_percentage: number;
  }> {
    return this.http.get<any>(this.apiConfig.buildUrl(`/networks/subnets/${subnetId}/usage`));
  }

  // 获取网络拓扑
  getNetworkTopology(): Observable<{
    nodes: Array<{
      id: number;
      name: string;
      networks: Array<{
        id: number;
        name: string;
        type: string;
        status: string;
      }>;
    }>;
    connections: Array<{
      from_node: number;
      to_node: number;
      network_id: number;
    }>;
  }> {
    return this.http.get<any>(this.apiConfig.buildUrl('/networks/topology'));
  }

  // 获取网络性能指标
  getNetworkMetrics(networkId: number, timeRange: string = '1h'): Observable<any> {
    const params = new HttpParams().set('time_range', timeRange);
    return this.http.get<any>(this.apiConfig.buildUrl(`/networks/${networkId}/metrics`), { params });
  }

  // 获取节点列表
  getNodes(): Observable<Node[]> {
    return this.http.get<any>(this.apiConfig.buildUrl('/nodes')).pipe(
      map(response => {
        // 后端返回的是 NodeListResponse 格式
        const nodes = response.nodes || [];
        // 转换字段名称以匹配前端接口
        return nodes.map((node: any) => ({
          id: node.id || '1', // 后端返回的是字符串ID，前端期望数字
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

  // 测试网络连通性
  testConnectivity(networkId: number, targetIP: string): Observable<{
    success: boolean;
    latency?: number;
    error?: string;
  }> {
    return this.http.post<any>(this.apiConfig.buildUrl(`/networks/${networkId}/test-connectivity`), {
      target_ip: targetIP
    });
  }
}

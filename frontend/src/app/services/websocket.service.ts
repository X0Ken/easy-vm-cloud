import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, Subject } from 'rxjs';

export interface FrontendMessage {
  type: 'VmStatusUpdate' | 'NodeStatusUpdate' | 'TaskStatusUpdate' | 'SystemNotification' | 'Pong';
  vm_id?: string;
  node_id?: string;
  task_id?: string;
  status?: string;
  progress?: number;
  message?: string;
  title?: string;
  level?: string;
  timestamp?: number;
}

@Injectable({
  providedIn: 'root'
})
export class WebSocketService {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectInterval = 5000; // 5秒
  private heartbeatInterval: any;
  
  // 消息流
  private messageSubject = new Subject<FrontendMessage>();
  public messages$ = this.messageSubject.asObservable();
  
  // 连接状态
  private connectionStateSubject = new BehaviorSubject<'disconnected' | 'connecting' | 'connected'>('disconnected');
  public connectionState$ = this.connectionStateSubject.asObservable();
  
  // VM 状态更新流
  private vmStatusUpdateSubject = new Subject<{vm_id: string, status: string, message?: string}>();
  public vmStatusUpdates$ = this.vmStatusUpdateSubject.asObservable();
  
  // 节点状态更新流
  private nodeStatusUpdateSubject = new Subject<{node_id: string, status: string, message?: string}>();
  public nodeStatusUpdates$ = this.nodeStatusUpdateSubject.asObservable();
  
  // 任务状态更新流
  private taskStatusUpdateSubject = new Subject<{task_id: string, status: string, progress?: number, message?: string}>();
  public taskStatusUpdates$ = this.taskStatusUpdateSubject.asObservable();
  
  // 系统通知流
  private systemNotificationSubject = new Subject<{title: string, message: string, level: string}>();
  public systemNotifications$ = this.systemNotificationSubject.asObservable();

  constructor() {
    this.connect();
  }

  /**
   * 连接到 WebSocket 服务器
   */
  connect(): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return;
    }

    this.connectionStateSubject.next('connecting');
    
    // 获取 WebSocket URL
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const wsUrl = `${protocol}//${host}/ws/frontend`;
    
    console.log('正在连接到 WebSocket:', wsUrl);
    
    try {
      this.ws = new WebSocket(wsUrl);
      
      this.ws.onopen = () => {
        console.log('WebSocket 连接已建立');
        this.connectionStateSubject.next('connected');
        this.reconnectAttempts = 0;
        this.startHeartbeat();
      };
      
      this.ws.onmessage = (event) => {
        this.handleMessage(event);
      };
      
      this.ws.onclose = (event) => {
        console.log('WebSocket 连接已关闭:', event.code, event.reason);
        this.connectionStateSubject.next('disconnected');
        this.stopHeartbeat();
        this.handleReconnect();
      };
      
      this.ws.onerror = (error) => {
        console.error('WebSocket 错误:', error);
        this.connectionStateSubject.next('disconnected');
      };
      
    } catch (error) {
      console.error('创建 WebSocket 连接失败:', error);
      this.connectionStateSubject.next('disconnected');
      this.handleReconnect();
    }
  }

  /**
   * 断开 WebSocket 连接
   */
  disconnect(): void {
    this.stopHeartbeat();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.connectionStateSubject.next('disconnected');
  }

  /**
   * 处理收到的消息
   */
  private handleMessage(event: MessageEvent): void {
    try {
      const message: FrontendMessage = JSON.parse(event.data);
      console.log('收到 WebSocket 消息:', message);
      
      // 发送到通用消息流
      this.messageSubject.next(message);
      
      // 根据消息类型发送到特定流
      switch (message.type) {
        case 'VmStatusUpdate':
          if (message.vm_id && message.status) {
            this.vmStatusUpdateSubject.next({
              vm_id: message.vm_id,
              status: message.status,
              message: message.message
            });
          }
          break;
          
        case 'NodeStatusUpdate':
          if (message.node_id && message.status) {
            this.nodeStatusUpdateSubject.next({
              node_id: message.node_id,
              status: message.status,
              message: message.message
            });
          }
          break;
          
        case 'TaskStatusUpdate':
          if (message.task_id && message.status) {
            this.taskStatusUpdateSubject.next({
              task_id: message.task_id,
              status: message.status,
              progress: message.progress,
              message: message.message
            });
          }
          break;
          
        case 'SystemNotification':
          if (message.title && message.message && message.level) {
            this.systemNotificationSubject.next({
              title: message.title,
              message: message.message,
              level: message.level
            });
          }
          break;
          
        case 'Pong':
          console.log('收到心跳响应:', message.timestamp);
          break;
      }
      
    } catch (error) {
      console.error('解析 WebSocket 消息失败:', error, event.data);
    }
  }

  /**
   * 发送消息到服务器
   */
  send(message: any): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket 未连接，无法发送消息');
    }
  }

  /**
   * 开始心跳
   */
  private startHeartbeat(): void {
    this.heartbeatInterval = setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.send({ type: 'ping' });
      }
    }, 30000); // 每30秒发送一次心跳
  }

  /**
   * 停止心跳
   */
  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  /**
   * 处理重连
   */
  private handleReconnect(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      console.log(`WebSocket 重连尝试 ${this.reconnectAttempts}/${this.maxReconnectAttempts}`);
      
      setTimeout(() => {
        this.connect();
      }, this.reconnectInterval);
    } else {
      console.error('WebSocket 重连失败，已达到最大重试次数');
    }
  }

  /**
   * 获取连接状态
   */
  getConnectionState(): 'disconnected' | 'connecting' | 'connected' {
    return this.connectionStateSubject.value;
  }

  /**
   * 检查是否已连接
   */
  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }
}

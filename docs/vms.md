# 虚拟机设计

## 操作流程

### 1. 创建虚拟机
```
API -> Server保存数据到DB -> UI提示成功
```
- Server 仅保存元数据到数据库
- Agent 无需操作
- 虚拟机状态为 "stopped"

### 2. 启动虚拟机
```
API -> Server记录DB -> UI提示进行中
--(notify)-> agent 重新define xml，启动虚拟机 --(notify)-> Server更新db记录 -> UI提示完成
```
- Server 更新状态为 "starting"
- 异步通知 Agent 启动虚拟机
- Agent 重新定义 XML 配置，确保与数据库一致
- Agent 启动虚拟机后通知 Server
- Server 更新状态为 "running"

### 3. 关机虚拟机
```
API -> Server记录DB -> UI提示进行中
--(notify)-> agent 关机并undefine xml --(notify)-> Server更新db记录 -> UI提示完成
```
- Server 更新状态为 "stopping"
- 异步通知 Agent 停止虚拟机
- Agent 停止虚拟机并取消定义
- Agent 通知 Server 操作完成
- Server 更新状态为 "stopped"

### 4. 删除虚拟机
```
API -> Server清理DB
```
- Server 仅清理数据库记录
- 释放相关资源（IP、存储卷等）
- Agent 无需操作

### 5. 挂载存储卷
```
API -> Server记录DB -> UI提示进行中
--(notify)-> agent 热挂载磁盘，并标记持久 --(notify)-> Server更新db记录 -> UI提示完成
```
- Server 更新虚拟机磁盘配置
- 如果虚拟机运行中，异步通知 Agent 热挂载
- 如果虚拟机未运行，仅更新数据库（启动时自动挂载）

### 6. 移除存储卷
```
API -> Server记录DB -> UI提示进行中
--(notify)-> agent 热解除磁盘，并标记持久 --(notify)-> Server更新db记录 -> UI提示完成
```
- Server 更新虚拟机磁盘配置
- 如果虚拟机运行中，异步通知 Agent 热分离
- 如果虚拟机未运行，仅更新数据库

### 7. 重启虚拟机
```
API -> Server记录DB -> UI提示进行中
--(notify)-> agent 尝试软关机并启动，否则强制关机并启动 --(notify)-> Server更新db记录 -> UI提示完成
```
- Server 更新状态为 "restarting"
- 异步通知 Agent 重启虚拟机
- Agent 停止虚拟机并重新启动
- Agent 通知 Server 操作完成
- Server 更新状态为 "running"
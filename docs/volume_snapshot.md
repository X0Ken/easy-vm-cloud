# 存储卷快照设计

## 操作流程

### 1. 创建快照
```
API -> Server记录DB -> UI提示进行中
       --(notify)-> agent
            --(volume in use) 调用libvirt创建快照 --(notify)-> Server更新db记录 -> UI提示完成
            --(volume not in use) 调用qemu创建快照 --(notify)-> Server更新db记录 -> UI提示完成
```

### 2. 删除快照
```
API -> Server记录DB -> UI提示进行中
       --(notify)-> agent
            --(volume in use) 调用libvirt删除快照 --(notify)-> Server更新db记录 -> UI提示完成
            --(volume not in use) 调用qemu删除快照 --(notify)-> Server更新db记录 -> UI提示完成
```

### 3. 恢复快照
```
API -> Server
        --(volume in use)  -> UI提示需要先停止虚拟机
        --(volume not in use) -> UI提示进行中
            --(notify)-> agent 调用qemu恢复快照 --(notify)-> Server更新db记录 -> UI提示完成
```

### 4. 更新快照
```
API -> Server更新DB -> UI提示完成
```
更新快照仅允许更新名称和描述

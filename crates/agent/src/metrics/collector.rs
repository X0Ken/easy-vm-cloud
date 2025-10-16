/// 指标收集器
/// 
/// 使用 sysinfo 收集系统资源信息

use sysinfo::System;

pub struct MetricsCollector {
    system: System,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    /// 刷新系统信息
    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    /// 获取 CPU 使用率
    pub fn cpu_usage(&self) -> f64 {
        // TODO: 实现 CPU 使用率计算
        0.0
    }

    /// 获取内存使用率
    pub fn memory_usage(&self) -> (u64, u64) {
        let total = self.system.total_memory();
        let available = self.system.available_memory();
        (total, available)
    }

    /// 获取磁盘使用率
    pub fn disk_usage(&self) -> (u64, u64) {
        // TODO: 实现磁盘使用率计算
        (0, 0)
    }
}


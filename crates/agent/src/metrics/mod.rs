/// 指标收集
/// 
/// 收集节点资源使用情况并暴露 Prometheus 指标

pub mod collector;

pub use collector::MetricsCollector;


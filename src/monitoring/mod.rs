//! Monitoring Module
//!
//! Provides observability for HIDO:
//! - Prometheus metrics
//! - Alerting system
//! - Structured logging

pub mod alerts;
pub mod logging;
pub mod metrics;

pub use alerts::{Alert, AlertManager, AlertRule, AlertSeverity};
pub use logging::{LogEntry, LogLevel, Logger};
pub use metrics::{Counter, Gauge, Histogram, MetricsCollector};

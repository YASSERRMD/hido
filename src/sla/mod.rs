//! SLA Module
//!
//! Provides Service Level Agreement tracking:
//! - SLA agreements and contracts
//! - Metrics tracking
//! - Reporting and breach detection

pub mod agreement;
pub mod reporter;
pub mod tracker;

pub use agreement::{SLAContract, SLOMetric, SLOTarget};
pub use reporter::{SLAReport, SLAReporter};
pub use tracker::{SLATracker, TrackedMetric};

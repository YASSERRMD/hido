//! Flexible Audit Layer
//!
//! Trait-based audit backend supporting:
//! - Blockchain (existing BAL)
//! - PostgreSQL
//! - Kafka+S3
//! - Hybrid (multi-backend)

pub mod backend;
pub mod backends;
pub mod config;
pub mod entry;
pub mod factory;
pub mod filter;

pub use backend::{AuditBackend, BackendType, VerificationResult};
pub use config::AuditConfig;
pub use entry::{AuditEntry, EntryId};
pub use factory::create_audit_backend;
pub use filter::AuditFilter;

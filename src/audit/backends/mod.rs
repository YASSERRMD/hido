//! Audit backend implementations.
//!
//! Four pluggable backends:
//! - Blockchain (wraps existing BAL)
//! - PostgreSQL
//! - Kafka+S3
//! - Hybrid

pub mod blockchain;
pub mod hybrid;
pub mod kafka_s3;
pub mod postgres;

pub use blockchain::BlockchainBackend;
pub use hybrid::HybridBackend;
pub use kafka_s3::KafkaS3Backend;
pub use postgres::PostgresBackend;

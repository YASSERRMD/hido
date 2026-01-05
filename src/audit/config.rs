//! Audit backend configuration.
//!
//! Configuration-driven backend selection.

use crate::audit::backend::BackendType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Audit layer configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Backend type to use
    pub backend: BackendType,
    /// Blockchain-specific config
    pub blockchain: Option<BlockchainConfig>,
    /// PostgreSQL-specific config
    pub postgresql: Option<PostgresConfig>,
    /// Kafka+S3-specific config
    pub kafka_s3: Option<KafkaS3Config>,
    /// Hybrid-specific config
    pub hybrid: Option<HybridConfig>,
}

impl AuditConfig {
    /// Create default config (blockchain).
    pub fn blockchain() -> Self {
        Self {
            backend: BackendType::Blockchain,
            blockchain: Some(BlockchainConfig::default()),
            postgresql: None,
            kafka_s3: None,
            hybrid: None,
        }
    }

    /// Create PostgreSQL config.
    pub fn postgresql(url: &str) -> Self {
        Self {
            backend: BackendType::PostgreSQL,
            blockchain: None,
            postgresql: Some(PostgresConfig {
                url: url.to_string(),
                ..Default::default()
            }),
            kafka_s3: None,
            hybrid: None,
        }
    }

    /// Create Kafka+S3 config.
    pub fn kafka_s3(brokers: &str, bucket: &str) -> Self {
        Self {
            backend: BackendType::KafkaS3,
            blockchain: None,
            postgresql: None,
            kafka_s3: Some(KafkaS3Config {
                kafka_brokers: brokers.to_string(),
                s3_bucket: bucket.to_string(),
                ..Default::default()
            }),
            hybrid: None,
        }
    }

    /// Create hybrid config.
    pub fn hybrid(primary: BackendType, secondary: BackendType) -> Self {
        Self {
            backend: BackendType::Hybrid,
            blockchain: None,
            postgresql: None,
            kafka_s3: None,
            hybrid: Some(HybridConfig {
                primary,
                secondary,
                sync_mode: SyncMode::Async,
            }),
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self::blockchain()
    }
}

/// Blockchain backend configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockchainConfig {
    /// RPC URL (if external blockchain)
    pub rpc_url: Option<String>,
    /// Contract address (if external)
    pub contract_address: Option<String>,
    /// Use internal BAL
    pub use_internal: bool,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            rpc_url: None,
            contract_address: None,
            use_internal: true,
        }
    }
}

/// PostgreSQL backend configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Connection URL
    pub url: String,
    /// Table name
    pub table: String,
    /// Max connections
    pub max_connections: u32,
    /// Enable SSL
    pub ssl: bool,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/hido_audit".to_string(),
            table: "audit_entries".to_string(),
            max_connections: 10,
            ssl: false,
        }
    }
}

/// Kafka+S3 backend configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KafkaS3Config {
    /// Kafka broker addresses
    pub kafka_brokers: String,
    /// Kafka topic
    pub kafka_topic: String,
    /// S3 bucket
    pub s3_bucket: String,
    /// S3 region
    pub s3_region: String,
    /// Archive after (seconds)
    pub archive_after_seconds: u64,
}

impl Default for KafkaS3Config {
    fn default() -> Self {
        Self {
            kafka_brokers: "localhost:9092".to_string(),
            kafka_topic: "hido-audit".to_string(),
            s3_bucket: "hido-audit-archive".to_string(),
            s3_region: "us-east-1".to_string(),
            archive_after_seconds: 86400, // 24 hours
        }
    }
}

/// Hybrid backend configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HybridConfig {
    /// Primary backend (fast reads)
    pub primary: BackendType,
    /// Secondary backend (immutable)
    pub secondary: BackendType,
    /// Sync mode
    pub sync_mode: SyncMode,
}

/// Sync mode for hybrid backend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMode {
    /// Synchronous writes to both
    Sync,
    /// Async secondary writes
    Async,
    /// Batch secondary writes
    Batch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuditConfig::default();
        assert_eq!(config.backend, BackendType::Blockchain);
        assert!(config.blockchain.is_some());
    }

    #[test]
    fn test_postgresql_config() {
        let config = AuditConfig::postgresql("postgresql://localhost/test");
        assert_eq!(config.backend, BackendType::PostgreSQL);
        assert!(config.postgresql.is_some());
        assert_eq!(config.postgresql.unwrap().url, "postgresql://localhost/test");
    }

    #[test]
    fn test_kafka_s3_config() {
        let config = AuditConfig::kafka_s3("localhost:9092", "my-bucket");
        assert_eq!(config.backend, BackendType::KafkaS3);
        assert!(config.kafka_s3.is_some());
    }

    #[test]
    fn test_hybrid_config() {
        let config = AuditConfig::hybrid(BackendType::PostgreSQL, BackendType::Blockchain);
        assert_eq!(config.backend, BackendType::Hybrid);
        assert!(config.hybrid.is_some());
        
        let hybrid = config.hybrid.unwrap();
        assert_eq!(hybrid.primary, BackendType::PostgreSQL);
        assert_eq!(hybrid.secondary, BackendType::Blockchain);
    }
}

//! Backend factory.
//!
//! Creates audit backends based on configuration.

use crate::audit::backend::{AuditBackend, BackendType};
use crate::audit::backends::{BlockchainBackend, HybridBackend, KafkaS3Backend, PostgresBackend};
use crate::audit::config::AuditConfig;
use crate::core::Result;
use futures::future::BoxFuture;
use std::sync::Arc;

/// Create an audit backend from configuration.
///
/// Returns an Arc-wrapped backend for shared ownership.
/// Returns BoxFuture to handle recursion with HybridBackend.
pub fn create_audit_backend(config: &AuditConfig) -> BoxFuture<'_, Result<Arc<dyn AuditBackend>>> {
    let config = config.clone();
    Box::pin(async move {
        match config.backend {
            BackendType::Blockchain => {
                let blockchain_config = config.blockchain.clone().unwrap_or_default();
                let backend = BlockchainBackend::new(blockchain_config)?;
                Ok(Arc::new(backend) as Arc<dyn AuditBackend>)
            }
            BackendType::PostgreSQL => {
                let pg_config = config.postgresql.clone().unwrap_or_default();
                let backend = PostgresBackend::new(pg_config).await?;
                Ok(Arc::new(backend) as Arc<dyn AuditBackend>)
            }
            BackendType::KafkaS3 => {
                let kafka_config = config.kafka_s3.clone().unwrap_or_default();
                let backend = KafkaS3Backend::new(kafka_config).await?;
                Ok(Arc::new(backend) as Arc<dyn AuditBackend>)
            }
            BackendType::Hybrid => {
                let hybrid_config = config.hybrid.clone().ok_or_else(|| {
                    crate::core::Error::Internal("Hybrid config required for hybrid backend".to_string())
                })?;
                let backend = HybridBackend::new(hybrid_config, &config).await?;
                Ok(Arc::new(backend) as Arc<dyn AuditBackend>)
            }
        }
    })
}

/// Create a simple blockchain backend (convenience function).
pub fn create_blockchain_backend() -> Result<Arc<dyn AuditBackend>> {
    let config = AuditConfig::blockchain();
    let blockchain_config = config.blockchain.clone().unwrap_or_default();
    let backend = BlockchainBackend::new(blockchain_config)?;
    Ok(Arc::new(backend))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_blockchain_backend() {
        let backend = create_blockchain_backend().unwrap();
        assert_eq!(backend.backend_type(), BackendType::Blockchain);
    }

    #[tokio::test]
    async fn test_factory_blockchain() {
        let config = AuditConfig::blockchain();
        let backend = create_audit_backend(&config).await.unwrap();
        assert_eq!(backend.backend_type(), BackendType::Blockchain);
    }
}

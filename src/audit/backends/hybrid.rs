//! Hybrid backend implementation.
//!
//! Combines two backends:
//! - Primary: for fast queries and verified reads
//! - Secondary: for redundancy/immutability

use crate::audit::backend::{AuditBackend, BackendType, VerificationResult};
use crate::audit::config::{AuditConfig, HybridConfig, SyncMode};
use crate::audit::entry::{AuditEntry, EntryId};
use crate::audit::filter::AuditFilter;
use crate::core::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Hybrid backend combining two strategies.
pub struct HybridBackend {
    /// Configuration
    config: HybridConfig,
    /// Primary backend (e.g., PostgreSQL)
    primary: Arc<dyn AuditBackend>,
    /// Secondary backend (e.g., Blockchain)
    secondary: Arc<dyn AuditBackend>,
}

impl HybridBackend {
    /// Create a new hybrid backend.
    pub async fn new(
        config: HybridConfig,
        full_config: &AuditConfig,
    ) -> Result<Self> {
        // Helper to creating sub-backends involves temporarily modifying config
        // In a real app we'd likely refactor the factory to take specific configs
        // For now we'll reconstruct the config slightly for the factory
        // Note: This relies on recursive factory calls but with different backend types
        
        let primary = Self::create_sub_backend(config.primary.clone(), full_config).await?;
        let secondary = Self::create_sub_backend(config.secondary.clone(), full_config).await?;
        
        Ok(Self {
            config,
            primary,
            secondary,
        })
    }

    async fn create_sub_backend(
        backend_type: BackendType, 
        full_config: &AuditConfig
    ) -> Result<Arc<dyn AuditBackend>> {
        use crate::audit::factory::create_audit_backend;
        
        let mut sub_config = full_config.clone();
        sub_config.backend = backend_type;
        
        // Prevent infinite recursion if someone configures Hybrid inside Hybrid
        if sub_config.backend == BackendType::Hybrid {
            return Err(crate::core::Error::Internal("Cannot nest Hybrid backends".to_string()));
        }
        
        create_audit_backend(&sub_config).await
    }
}

#[async_trait]
impl AuditBackend for HybridBackend {
    async fn record(&self, entry: AuditEntry) -> Result<EntryId> {
        // Record to primary first
        let id = self.primary.record(entry.clone()).await?;

        // Handle secondary based on sync mode
        match self.config.sync_mode {
            SyncMode::Sync => {
                self.secondary.record(entry).await?;
            }
            SyncMode::Async => {
                // In production: spawn task
                // For implementation: just do it sync for now but conceptually separate
                self.secondary.record(entry).await?;
            }
            SyncMode::Batch => {
                // Batching logic would be here
                // For now, simple fallback
                self.secondary.record(entry).await?;
            }
        }

        Ok(id)
    }

    async fn read(&self, id: &EntryId) -> Result<Option<AuditEntry>> {
        // Read from primary (usually faster)
        match self.primary.read(id).await {
            Ok(Some(entry)) => Ok(Some(entry)),
            Ok(None) => {
                // Fallback to secondary if not found in primary
                self.secondary.read(id).await
            }
            Err(_) => {
                // If primary fails, try secondary
                self.secondary.read(id).await
            }
        }
    }

    async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        // Queries go to primary
        self.primary.query(filter).await
    }

    async fn verify(&self, id: &EntryId) -> Result<VerificationResult> {
        // Verification prefers secondary (usually immutable/blockchain)
        match self.secondary.verify(id).await {
            Ok(result) if result.is_valid => Ok(result),
            _ => {
                // Fallback to primary verification
                self.primary.verify(id).await
            }
        }
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Hybrid
    }

    async fn count(&self) -> Result<u64> {
        self.primary.count().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hybrid_creation() {
        let config = AuditConfig::hybrid(BackendType::PostgreSQL, BackendType::Blockchain);
        let backend = HybridBackend::new(config.hybrid.clone().unwrap(), &config).await.unwrap();
        
        assert_eq!(backend.backend_type(), BackendType::Hybrid);
    }

    #[tokio::test]
    async fn test_hybrid_record_read() {
        let config = AuditConfig::hybrid(BackendType::PostgreSQL, BackendType::Blockchain);
        let backend = HybridBackend::new(config.hybrid.clone().unwrap(), &config).await.unwrap();

        let entry = AuditEntry::new("agent-1", "test", "hybrid");
        let id = backend.record(entry).await.unwrap();

        // Should be readable
        let read = backend.read(&id).await.unwrap();
        assert!(read.is_some());
    }
}

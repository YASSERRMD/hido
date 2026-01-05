//! AuditBackend trait definition.
//!
//! Core trait that all audit backends must implement.

use crate::audit::entry::{AuditEntry, EntryId};
use crate::audit::filter::AuditFilter;
use crate::core::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Backend type identifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    /// Blockchain-based audit (existing BAL)
    Blockchain,
    /// PostgreSQL with JSONB
    PostgreSQL,
    /// Kafka streaming + S3 archival
    KafkaS3,
    /// Hybrid: combines multiple backends
    Hybrid,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Blockchain => write!(f, "blockchain"),
            BackendType::PostgreSQL => write!(f, "postgresql"),
            BackendType::KafkaS3 => write!(f, "kafka_s3"),
            BackendType::Hybrid => write!(f, "hybrid"),
        }
    }
}

/// Result of entry verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Entry ID verified
    pub entry_id: EntryId,
    /// Is verified/valid
    pub is_valid: bool,
    /// Hash matches
    pub hash_valid: bool,
    /// Signature valid (if signed)
    pub signature_valid: Option<bool>,
    /// Verification message
    pub message: String,
}

impl VerificationResult {
    /// Create a valid result.
    pub fn valid(entry_id: EntryId) -> Self {
        Self {
            entry_id,
            is_valid: true,
            hash_valid: true,
            signature_valid: None,
            message: "Entry verified successfully".to_string(),
        }
    }

    /// Create an invalid result.
    pub fn invalid(entry_id: EntryId, message: &str) -> Self {
        Self {
            entry_id,
            is_valid: false,
            hash_valid: false,
            signature_valid: None,
            message: message.to_string(),
        }
    }
}

/// Core trait for audit backends.
///
/// All audit storage implementations must implement this trait.
/// Enables zero-code switching between backends via configuration.
#[async_trait]
pub trait AuditBackend: Send + Sync {
    /// Record a new audit entry.
    ///
    /// Returns the unique entry ID on success.
    async fn record(&self, entry: AuditEntry) -> Result<EntryId>;

    /// Read an entry by ID.
    ///
    /// Returns None if entry doesn't exist.
    async fn read(&self, id: &EntryId) -> Result<Option<AuditEntry>>;

    /// Query entries matching a filter.
    ///
    /// Returns matching entries up to optional limit.
    async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>>;

    /// Verify an entry's integrity.
    ///
    /// Checks hash, signature, and chain validity.
    async fn verify(&self, id: &EntryId) -> Result<VerificationResult>;

    /// Get the backend type.
    fn backend_type(&self) -> BackendType;

    /// Health check for the backend.
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    /// Get entry count (if supported).
    async fn count(&self) -> Result<u64> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_type_display() {
        assert_eq!(BackendType::Blockchain.to_string(), "blockchain");
        assert_eq!(BackendType::PostgreSQL.to_string(), "postgresql");
        assert_eq!(BackendType::KafkaS3.to_string(), "kafka_s3");
        assert_eq!(BackendType::Hybrid.to_string(), "hybrid");
    }

    #[test]
    fn test_verification_result_valid() {
        let result = VerificationResult::valid(EntryId::new("test-id"));
        assert!(result.is_valid);
        assert!(result.hash_valid);
    }

    #[test]
    fn test_verification_result_invalid() {
        let result = VerificationResult::invalid(EntryId::new("test-id"), "Hash mismatch");
        assert!(!result.is_valid);
        assert!(!result.hash_valid);
        assert!(result.message.contains("Hash mismatch"));
    }
}

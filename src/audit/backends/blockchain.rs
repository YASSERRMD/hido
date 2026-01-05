//! Blockchain backend implementation.
//!
//! Wraps existing BAL (AgentBlockchain) with AuditBackend trait.
//! No breaking changes to existing code.

use crate::audit::backend::{AuditBackend, BackendType, VerificationResult};
use crate::audit::config::BlockchainConfig;
use crate::audit::entry::{AuditEntry, EntryId, EntryType};
use crate::audit::filter::AuditFilter;
use crate::bal::block::{AgentAction, AgentActionBlock};
use crate::bal::chain::AgentBlockchain;
use crate::core::Result;
use crate::uail::crypto::CryptoSuite;
use crate::uail::DIDKey;
use async_trait::async_trait;
use tokio::sync::RwLock;

/// Blockchain backend wrapping existing AgentBlockchain.
pub struct BlockchainBackend {
    /// Internal blockchain
    chain: RwLock<AgentBlockchain>,
    /// System DID for signing
    system_did: DIDKey,
    /// Configuration
    config: BlockchainConfig,
}

impl BlockchainBackend {
    /// Create a new blockchain backend.
    pub fn new(config: BlockchainConfig) -> Result<Self> {
        let chain = AgentBlockchain::new()?;
        let crypto = CryptoSuite::new();
        let system_did = DIDKey::new(&crypto);

        Ok(Self {
            chain: RwLock::new(chain),
            system_did,
            config,
        })
    }

    /// Get the underlying blockchain (for direct access if needed).
    pub async fn blockchain(&self) -> tokio::sync::RwLockReadGuard<AgentBlockchain> {
        self.chain.read().await
    }

    /// Convert AuditEntry to AgentAction.
    fn entry_to_action(entry: &AuditEntry) -> AgentAction {
        let action_type = match &entry.entry_type {
            EntryType::AgentAction => "agent_action",
            EntryType::Decision => "decision",
            EntryType::Access => "access",
            EntryType::ConfigChange => "config_change",
            EntryType::Compliance => "compliance",
            EntryType::Custom(s) => s.as_str(),
        };

        AgentAction::new(action_type, &entry.target)
    }

    /// Convert AgentActionBlock to AuditEntry.
    fn block_to_entry(block: &AgentActionBlock) -> AuditEntry {
        // Need to compute hash since it's not stored in the block struct
        let mut block_clone = block.clone();
        let hash = block_clone.compute_hash();

        AuditEntry::new(
            &block.agent_id,
            &block.action.action_type,
            &block.action.target,
        )
        .with_metadata("block_height", block.block_height)
        .with_metadata("block_hash", hash.to_hex())
    }
}

#[async_trait]
impl AuditBackend for BlockchainBackend {
    async fn record(&self, entry: AuditEntry) -> Result<EntryId> {
        let action = Self::entry_to_action(&entry);
        
        let hash = {
            let mut chain = self.chain.write().await;
            // Await the add_action call
            chain.add_action(&self.system_did, action, &entry.action).await?
        };

        Ok(EntryId::new(&hash.to_hex()))
    }

    async fn read(&self, id: &EntryId) -> Result<Option<AuditEntry>> {
        let chain = self.chain.read().await;
        
        // Try to parse ID as block hash
        if let Ok(hash) = crate::core::Hash256::from_hex(id.as_str()) {
            if let Some(block) = chain.get_block_by_hash(&hash) {
                return Ok(Some(Self::block_to_entry(block)));
            }
        }
        
        Ok(None)
    }

    async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        let chain = self.chain.read().await;
        let mut results = Vec::new();

        for block in chain.all_blocks() {
            let entry = Self::block_to_entry(block);
            
            if filter.matches(&entry) {
                results.push(entry);
            }

            if let Some(limit) = filter.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    async fn verify(&self, id: &EntryId) -> Result<VerificationResult> {
        let chain = self.chain.read().await;

        if let Ok(hash) = crate::core::Hash256::from_hex(id.as_str()) {
            if let Some(block) = chain.get_block_by_hash(&hash) {
                // Get parent block for verification
                let parent = if block.block_height > 0 {
                    chain.get_block(block.block_height - 1)
                } else {
                    None
                };

                match block.verify(parent) {
                    Ok(verification) => {
                        // Assuming BlockVerification has: valid, tamper_detected, missing_approvals
                        return Ok(VerificationResult {
                            entry_id: id.clone(),
                            is_valid: verification.valid,
                            hash_valid: verification.valid,
                            signature_valid: None,
                            message: if verification.valid {
                                "Block verified successfully".to_string()
                            } else {
                                "Block verification failed".to_string()
                            },
                        });
                    }
                    Err(e) => {
                        return Ok(VerificationResult::invalid(id.clone(), &e.to_string()));
                    }
                }
            }
        }

        Ok(VerificationResult::invalid(id.clone(), "Entry not found"))
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Blockchain
    }

    async fn health_check(&self) -> Result<bool> {
        let chain = self.chain.read().await;
        let verification = chain.verify_chain()?;
        Ok(verification.valid)
    }

    async fn count(&self) -> Result<u64> {
        let chain = self.chain.read().await;
        Ok(chain.height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blockchain_backend_creation() {
        let backend = BlockchainBackend::new(BlockchainConfig::default()).unwrap();
        assert_eq!(backend.backend_type(), BackendType::Blockchain);
    }

    #[tokio::test]
    async fn test_record_and_read() {
        let backend = BlockchainBackend::new(BlockchainConfig::default()).unwrap();
        
        let entry = AuditEntry::new("agent-1", "execute", "task-1");
        let id = backend.record(entry).await.unwrap();

        let read_entry = backend.read(&id).await.unwrap();
        assert!(read_entry.is_some());
    }

    #[tokio::test]
    async fn test_query() {
        let backend = BlockchainBackend::new(BlockchainConfig::default()).unwrap();
        
        backend.record(AuditEntry::new("agent-1", "execute", "task-1")).await.unwrap();
        backend.record(AuditEntry::new("agent-2", "execute", "task-2")).await.unwrap();

        let filter = AuditFilter::new().with_limit(10);
        let results = backend.query(&filter).await.unwrap();
        
        // Genesis block + 2 entries
        assert!(results.len() >= 2);
    }

    #[tokio::test]
    async fn test_verify() {
        let backend = BlockchainBackend::new(BlockchainConfig::default()).unwrap();
        
        let entry = AuditEntry::new("agent-1", "execute", "task-1");
        let id = backend.record(entry).await.unwrap();

        let result = backend.verify(&id).await.unwrap();
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_health_check() {
        let backend = BlockchainBackend::new(BlockchainConfig::default()).unwrap();
        let healthy = backend.health_check().await.unwrap();
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_count() {
        let backend = BlockchainBackend::new(BlockchainConfig::default()).unwrap();
        
        let initial = backend.count().await.unwrap();
        backend.record(AuditEntry::new("agent-1", "test", "target")).await.unwrap();
        let after = backend.count().await.unwrap();

        assert_eq!(after, initial + 1);
    }
}

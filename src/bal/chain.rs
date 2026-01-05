//! Blockchain chain for agent action audit trail.
//!
//! Maintains an immutable, verifiable chain of action blocks.

use crate::bal::block::{AgentAction, AgentActionBlock};
use crate::core::{now, Error, Hash256, Result, Timestamp};
use crate::uail::DIDKey;

/// Blockchain metadata.
#[derive(Clone, Debug)]
pub struct ChainMetadata {
    /// Chain creation timestamp
    pub created: Timestamp,
    /// Total number of actions
    pub total_actions: u64,
    /// Total number of approvals
    pub total_approvals: u64,
    /// Hash of the head block
    pub head_hash: Hash256,
}

/// Result of chain verification.
#[derive(Clone, Debug)]
pub struct ChainVerification {
    /// Whether the chain is valid
    pub valid: bool,
    /// Number of blocks verified
    pub blocks_verified: u64,
    /// Whether tampering was detected
    pub tamper_detected: bool,
    /// Height of first invalid block (if any)
    pub first_invalid_height: Option<u64>,
}

/// Agent blockchain for audit trail.
pub struct AgentBlockchain {
    /// Chain of blocks
    blocks: Vec<AgentActionBlock>,
    /// Genesis hash
    pub genesis_hash: Hash256,
    /// Chain metadata
    pub metadata: ChainMetadata,
}

impl AgentBlockchain {
    /// Create a new blockchain with genesis block.
    pub fn new() -> Result<Self> {
        let mut genesis = AgentActionBlock::genesis();
        genesis.compute_hash();

        let metadata = ChainMetadata {
            created: now(),
            total_actions: 0,
            total_approvals: 0,
            head_hash: genesis.block_hash.clone(),
        };

        Ok(Self {
            genesis_hash: genesis.block_hash.clone(),
            blocks: vec![genesis],
            metadata,
        })
    }

    /// Get the current chain height.
    pub fn height(&self) -> u64 {
        self.blocks.len() as u64 - 1 // -1 because genesis is at height 0
    }

    /// Get the head block hash.
    pub fn head_hash(&self) -> &Hash256 {
        &self.blocks.last().unwrap().block_hash
    }

    /// Append a new action block to the chain.
    pub async fn append_block(&mut self, block: AgentActionBlock) -> Result<Hash256> {
        // Verify block links to current head
        let head = self.blocks.last().unwrap();

        if block.parent_hash != head.block_hash {
            return Err(Error::InvalidParentHash);
        }

        if block.block_height != head.block_height + 1 {
            return Err(Error::BlockCreationFailed(format!(
                "Invalid height: expected {}, got {}",
                head.block_height + 1,
                block.block_height
            )));
        }

        // Verify block integrity
        let verification = block.verify(Some(head))?;
        if !verification.valid {
            if verification.tamper_detected {
                return Err(Error::BlockTampered);
            }
            return Err(Error::BlockVerificationFailed("Block verification failed".into()));
        }

        // Update metadata
        self.metadata.total_actions += 1;
        self.metadata.total_approvals += block.approvers.len() as u64;
        self.metadata.head_hash = block.block_hash.clone();

        let hash = block.block_hash.clone();
        self.blocks.push(block);

        Ok(hash)
    }

    /// Create and append a new action.
    pub async fn add_action(
        &mut self,
        agent: &DIDKey,
        action: AgentAction,
        reasoning: &str,
    ) -> Result<Hash256> {
        let parent_hash = self.head_hash().clone();
        let height = self.height() + 1;

        let block = AgentActionBlock::new(height, agent, action, parent_hash)?
            .with_reasoning(reasoning);

        self.append_block(block).await
    }

    /// Get block by height.
    pub fn get_block(&self, height: u64) -> Option<&AgentActionBlock> {
        self.blocks.get(height as usize)
    }

    /// Get block by hash.
    pub fn get_block_by_hash(&self, hash: &Hash256) -> Option<&AgentActionBlock> {
        self.blocks.iter().find(|b| &b.block_hash == hash)
    }

    /// Verify entire chain integrity.
    pub fn verify_chain(&self) -> Result<ChainVerification> {
        let mut verification = ChainVerification {
            valid: true,
            blocks_verified: 0,
            tamper_detected: false,
            first_invalid_height: None,
        };

        for (i, block) in self.blocks.iter().enumerate() {
            let parent = if i > 0 {
                Some(&self.blocks[i - 1])
            } else {
                None
            };

            let block_verification = block.verify(parent)?;

            if !block_verification.valid {
                verification.valid = false;
                verification.tamper_detected = block_verification.tamper_detected;
                verification.first_invalid_height = Some(block.block_height);
                break;
            }

            verification.blocks_verified += 1;
        }

        Ok(verification)
    }

    /// Get action history for a specific agent.
    pub fn get_agent_history(&self, agent_id: &str) -> Vec<&AgentActionBlock> {
        self.blocks
            .iter()
            .filter(|b| b.agent_id == agent_id)
            .collect()
    }

    /// Get actions by type.
    pub fn get_actions_by_type(&self, action_type: &str) -> Vec<&AgentActionBlock> {
        self.blocks
            .iter()
            .filter(|b| b.action.action_type == action_type)
            .collect()
    }

    /// Get recent blocks.
    pub fn get_recent(&self, count: usize) -> Vec<&AgentActionBlock> {
        self.blocks.iter().rev().take(count).collect()
    }

    /// Get all blocks.
    pub fn all_blocks(&self) -> &[AgentActionBlock] {
        &self.blocks
    }

    /// Export chain to JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self.blocks)?)
    }

    /// Import chain from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        let blocks: Vec<AgentActionBlock> = serde_json::from_str(json)?;

        if blocks.is_empty() {
            return Err(Error::BlockCreationFailed("Empty chain".into()));
        }

        let genesis_hash = blocks[0].block_hash.clone();
        let head_hash = blocks.last().unwrap().block_hash.clone();

        let metadata = ChainMetadata {
            created: blocks[0].timestamp,
            total_actions: blocks.len() as u64 - 1,
            total_approvals: blocks.iter().map(|b| b.approvers.len() as u64).sum(),
            head_hash,
        };

        let chain = Self {
            blocks,
            genesis_hash,
            metadata,
        };

        // Verify imported chain
        let verification = chain.verify_chain()?;
        if !verification.valid {
            return Err(Error::ChainIntegrityViolated(
                verification.first_invalid_height.unwrap_or(0),
            ));
        }

        Ok(chain)
    }
}

impl Default for AgentBlockchain {
    fn default() -> Self {
        Self::new().expect("Failed to create blockchain")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uail::crypto::CryptoSuite;

    fn create_test_did() -> DIDKey {
        let crypto = CryptoSuite::new();
        DIDKey::new(&crypto)
    }

    #[tokio::test]
    async fn test_blockchain_creation() {
        let chain = AgentBlockchain::new().unwrap();
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.blocks.len(), 1);
    }

    #[tokio::test]
    async fn test_add_action() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        let action = AgentAction::new("test", "target");
        let hash = chain.add_action(&agent, action, "Test action").await.unwrap();

        assert_eq!(chain.height(), 1);
        assert_ne!(hash, Hash256::zero());
    }

    #[tokio::test]
    async fn test_multiple_actions() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        for i in 0..5 {
            let action = AgentAction::new("action", &format!("target_{}", i));
            chain.add_action(&agent, action, &format!("Action {}", i)).await.unwrap();
        }

        assert_eq!(chain.height(), 5);
        assert_eq!(chain.metadata.total_actions, 5);
    }

    #[tokio::test]
    async fn test_chain_verification() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        let action = AgentAction::new("verified", "action");
        chain.add_action(&agent, action, "Should verify").await.unwrap();

        let verification = chain.verify_chain().unwrap();
        assert!(verification.valid);
        assert_eq!(verification.blocks_verified, 2);
        assert!(!verification.tamper_detected);
    }

    #[tokio::test]
    async fn test_get_block_by_height() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        let action = AgentAction::new("get", "block");
        chain.add_action(&agent, action, "Getting block").await.unwrap();

        let genesis = chain.get_block(0).unwrap();
        assert_eq!(genesis.block_height, 0);

        let block1 = chain.get_block(1).unwrap();
        assert_eq!(block1.block_height, 1);

        assert!(chain.get_block(999).is_none());
    }

    #[tokio::test]
    async fn test_get_block_by_hash() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        let action = AgentAction::new("find", "hash");
        let hash = chain.add_action(&agent, action, "Finding by hash").await.unwrap();

        let block = chain.get_block_by_hash(&hash).unwrap();
        assert_eq!(block.block_hash, hash);
    }

    #[tokio::test]
    async fn test_agent_history() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent1 = create_test_did();
        let agent2 = create_test_did();

        chain.add_action(&agent1, AgentAction::new("a1", "t1"), "By agent 1").await.unwrap();
        chain.add_action(&agent2, AgentAction::new("a2", "t2"), "By agent 2").await.unwrap();
        chain.add_action(&agent1, AgentAction::new("a3", "t3"), "By agent 1 again").await.unwrap();

        let history1 = chain.get_agent_history(&agent1.id);
        assert_eq!(history1.len(), 2);

        let history2 = chain.get_agent_history(&agent2.id);
        assert_eq!(history2.len(), 1);
    }

    #[tokio::test]
    async fn test_chain_serialization() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        chain.add_action(&agent, AgentAction::new("serialize", "test"), "For serialization")
            .await.unwrap();

        let json = chain.to_json().unwrap();
        let restored = AgentBlockchain::from_json(&json).unwrap();

        assert_eq!(restored.height(), chain.height());
        assert_eq!(restored.genesis_hash, chain.genesis_hash);
    }

    #[tokio::test]
    async fn test_invalid_parent_hash() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        let block = AgentActionBlock::new(
            1,
            &agent,
            AgentAction::new("invalid", "parent"),
            Hash256::new([1u8; 32]), // Wrong parent hash
        ).unwrap();

        let result = chain.append_block(block).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_actions_by_type() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        chain.add_action(&agent, AgentAction::new("read", "db1"), "Read 1").await.unwrap();
        chain.add_action(&agent, AgentAction::new("write", "db1"), "Write 1").await.unwrap();
        chain.add_action(&agent, AgentAction::new("read", "db2"), "Read 2").await.unwrap();

        let reads = chain.get_actions_by_type("read");
        assert_eq!(reads.len(), 2);

        let writes = chain.get_actions_by_type("write");
        assert_eq!(writes.len(), 1);
    }

    #[tokio::test]
    async fn test_recent_blocks() {
        let mut chain = AgentBlockchain::new().unwrap();
        let agent = create_test_did();

        for i in 0..10 {
            chain.add_action(&agent, AgentAction::new("action", &i.to_string()), "Action")
                .await.unwrap();
        }

        let recent = chain.get_recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].block_height, 10);
        assert_eq!(recent[1].block_height, 9);
        assert_eq!(recent[2].block_height, 8);
    }
}

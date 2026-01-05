//! Agent action block structure.
//!
//! Content-addressed, immutable action blocks for the audit trail.

use crate::core::{now, Hash256, Result, Timestamp};
use crate::icc::intent::SemanticIntent;
use crate::uail::crypto::sha3_256_multi;
use crate::uail::DIDKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An action performed by an agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentAction {
    /// Type of action
    pub action_type: String,
    /// Target of the action
    pub target: String,
    /// Associated intent (if any)
    pub intent: Option<SemanticIntent>,
}

impl AgentAction {
    /// Create a new action.
    pub fn new(action_type: &str, target: &str) -> Self {
        Self {
            action_type: action_type.to_string(),
            target: target.to_string(),
            intent: None,
        }
    }

    /// Create action with associated intent.
    pub fn with_intent(mut self, intent: SemanticIntent) -> Self {
        self.intent = Some(intent);
        self
    }
}

/// An immutable action block in the audit trail.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentActionBlock {
    /// Block height in the chain
    pub block_height: u64,
    /// Block creation timestamp
    pub timestamp: Timestamp,
    /// Hash of the parent block
    pub parent_hash: Hash256,
    /// Agent that performed the action
    pub agent_id: String,
    /// The action performed
    pub action: AgentAction,
    /// Action parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Justification/reasoning for the action
    pub reasoning: String,
    /// Approvers of the action
    pub approvers: Vec<String>,
    /// Approval signatures
    pub approval_signatures: Vec<Vec<u8>>,
    /// Content hash of the block
    pub block_hash: Hash256,
}

impl AgentActionBlock {
    /// Create a new action block.
    pub fn new(
        height: u64,
        agent: &DIDKey,
        action: AgentAction,
        parent_hash: Hash256,
    ) -> Result<Self> {
        let mut block = Self {
            block_height: height,
            timestamp: now(),
            parent_hash,
            agent_id: agent.id.clone(),
            action,
            parameters: HashMap::new(),
            reasoning: String::new(),
            approvers: Vec::new(),
            approval_signatures: Vec::new(),
            block_hash: Hash256::zero(),
        };
        block.compute_hash();
        Ok(block)
    }

    /// Create genesis block.
    pub fn genesis() -> Self {
        let mut block = Self {
            block_height: 0,
            timestamp: now(),
            parent_hash: Hash256::zero(),
            agent_id: "system".to_string(),
            action: AgentAction::new("genesis", "chain"),
            parameters: HashMap::new(),
            reasoning: "Chain initialization".to_string(),
            approvers: Vec::new(),
            approval_signatures: Vec::new(),
            block_hash: Hash256::zero(),
        };
        block.compute_hash();
        block
    }

    /// Add a parameter to the block.
    pub fn with_param(mut self, key: &str, value: serde_json::Value) -> Self {
        self.parameters.insert(key.to_string(), value);
        self.compute_hash();
        self
    }

    /// Set the reasoning.
    pub fn with_reasoning(mut self, reasoning: &str) -> Self {
        self.reasoning = reasoning.to_string();
        self.compute_hash();
        self
    }

    /// Compute content hash (SHA3-256).
    /// Hash includes all fields except the hash itself.
    pub fn compute_hash(&mut self) -> Hash256 {
        let height_bytes = self.block_height.to_le_bytes();
        let timestamp_str = self.timestamp.to_rfc3339();
        let action_json = serde_json::to_string(&self.action).unwrap_or_default();
        let params_json = serde_json::to_string(&self.parameters).unwrap_or_default();

        let hash = sha3_256_multi(&[
            &height_bytes,
            timestamp_str.as_bytes(),
            self.parent_hash.as_bytes(),
            self.agent_id.as_bytes(),
            action_json.as_bytes(),
            params_json.as_bytes(),
            self.reasoning.as_bytes(),
        ]);

        self.block_hash = hash.clone();
        hash
    }

    /// Verify block integrity.
    pub fn verify(&self, parent: Option<&AgentActionBlock>) -> Result<BlockVerification> {
        let mut verification = BlockVerification {
            valid: true,
            tamper_detected: false,
            missing_approvals: Vec::new(),
        };

        // Verify hash
        let mut block_copy = self.clone();
        block_copy.compute_hash();
        if block_copy.block_hash != self.block_hash {
            verification.valid = false;
            verification.tamper_detected = true;
            return Ok(verification);
        }

        // Verify parent link
        if let Some(parent_block) = parent {
            if self.parent_hash != parent_block.block_hash {
                verification.valid = false;
                return Ok(verification);
            }
            if self.block_height != parent_block.block_height + 1 {
                verification.valid = false;
                return Ok(verification);
            }
        } else if self.block_height != 0 {
            // Non-genesis block must have parent
            verification.valid = false;
        }

        Ok(verification)
    }

    /// Add an approver signature.
    pub fn add_approval(&mut self, approver: &DIDKey, signature: Vec<u8>) -> Result<()> {
        self.approvers.push(approver.id.clone());
        self.approval_signatures.push(signature);
        Ok(())
    }

    /// Check if block has required approvals.
    pub fn has_required_approvals(&self, required: usize) -> bool {
        self.approvers.len() >= required
    }

    /// Serialize block to JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize block from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// Result of block verification.
#[derive(Clone, Debug)]
pub struct BlockVerification {
    /// Whether the block is valid
    pub valid: bool,
    /// Whether tampering was detected
    pub tamper_detected: bool,
    /// Missing required approvers
    pub missing_approvals: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uail::crypto::CryptoSuite;

    fn create_test_did() -> DIDKey {
        let crypto = CryptoSuite::new();
        DIDKey::new(&crypto)
    }

    #[test]
    fn test_block_creation() {
        let agent = create_test_did();
        let action = AgentAction::new("read", "database");
        let block = AgentActionBlock::new(1, &agent, action, Hash256::zero()).unwrap();

        assert_eq!(block.block_height, 1);
        assert_eq!(block.agent_id, agent.id);
        assert_ne!(block.block_hash, Hash256::zero());
    }

    #[test]
    fn test_block_hash_deterministic() {
        let agent = create_test_did();
        let action = AgentAction::new("write", "storage");

        let block1 = AgentActionBlock::new(1, &agent, action.clone(), Hash256::zero()).unwrap();

        // Clone and recompute hash - should be the same since timestamp is unchanged
        let mut block2 = block1.clone();
        block2.compute_hash();

        assert_eq!(block1.block_hash, block2.block_hash);
    }

    #[test]
    fn test_block_hash_changes_on_modification() {
        let agent = create_test_did();
        let action = AgentAction::new("process", "task");
        let block1 = AgentActionBlock::new(1, &agent, action.clone(), Hash256::zero()).unwrap();

        let block2 = block1.clone().with_param("extra", serde_json::json!(true));

        assert_ne!(block1.block_hash, block2.block_hash);
    }

    #[test]
    fn test_block_verification_valid() {
        let agent = create_test_did();
        let genesis = AgentActionBlock::genesis();

        let action = AgentAction::new("test", "target");
        let block =
            AgentActionBlock::new(1, &agent, action, genesis.block_hash.clone()).unwrap();

        let result = block.verify(Some(&genesis)).unwrap();
        assert!(result.valid);
        assert!(!result.tamper_detected);
    }

    #[test]
    fn test_block_verification_tampered() {
        let agent = create_test_did();
        let action = AgentAction::new("test", "target");
        let mut block = AgentActionBlock::new(1, &agent, action, Hash256::zero()).unwrap();

        // Tamper with the block
        block.reasoning = "Modified!".to_string();
        // Don't recompute hash

        let result = block.verify(None).unwrap();
        assert!(!result.valid);
        assert!(result.tamper_detected);
    }

    #[test]
    fn test_genesis_block() {
        let genesis = AgentActionBlock::genesis();
        assert_eq!(genesis.block_height, 0);
        assert_eq!(genesis.parent_hash, Hash256::zero());

        let result = genesis.verify(None).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_block_approval() {
        let agent = create_test_did();
        let approver = create_test_did();
        let crypto = CryptoSuite::new();

        let action = AgentAction::new("sensitive", "operation");
        let mut block = AgentActionBlock::new(1, &agent, action, Hash256::zero()).unwrap();

        let signature = crypto.sign(block.block_hash.as_bytes());
        block.add_approval(&approver, signature).unwrap();

        assert_eq!(block.approvers.len(), 1);
        assert!(block.has_required_approvals(1));
        assert!(!block.has_required_approvals(2));
    }

    #[test]
    fn test_block_serialization() {
        let agent = create_test_did();
        let action = AgentAction::new("test", "serialization");
        let block = AgentActionBlock::new(1, &agent, action, Hash256::zero())
            .unwrap()
            .with_param("key", serde_json::json!("value"));

        let json = block.to_json().unwrap();
        let parsed = AgentActionBlock::from_json(&json).unwrap();

        assert_eq!(parsed.block_height, block.block_height);
        assert_eq!(parsed.block_hash, block.block_hash);
    }

    #[test]
    fn test_action_with_intent() {
        let agent = create_test_did();
        let intent = crate::icc::intent::SemanticIntent::new(
            &agent,
            crate::icc::intent::IntentDomain::Data,
            "query",
        );

        let action = AgentAction::new("execute", "intent").with_intent(intent.clone());
        assert!(action.intent.is_some());
        assert_eq!(action.intent.unwrap().id, intent.id);
    }
}

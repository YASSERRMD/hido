//! Semantic Intent representation.
//!
//! Structured format for agent communication with domain taxonomy.

use crate::core::{now, Result, Timestamp};
use crate::uail::DIDKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Domain taxonomy for intent classification.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentDomain {
    /// Data operations (read, write, transform)
    Data,
    /// Compute operations (process, analyze, inference)
    Compute,
    /// Communication operations (send, receive, broadcast)
    Communication,
    /// Coordination operations (orchestrate, schedule, delegate)
    Coordination,
    /// Custom domain
    Custom(String),
}

impl Default for IntentDomain {
    fn default() -> Self {
        Self::Data
    }
}

/// Priority levels for intent processing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IntentPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for IntentPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Constraint on intent execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentConstraint {
    /// Constraint type (e.g., "timeout", "location", "capability")
    pub constraint_type: String,
    /// Constraint value
    pub value: serde_json::Value,
    /// Whether constraint is required
    pub required: bool,
}

/// A semantic intent for agent communication.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticIntent {
    /// Unique intent identifier
    pub id: String,
    /// Intent domain
    pub domain: IntentDomain,
    /// Action to perform
    pub action: String,
    /// Target of the action (optional)
    pub target: Option<String>,
    /// Action parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Execution constraints
    pub constraints: Vec<IntentConstraint>,
    /// Priority level
    pub priority: IntentPriority,
    /// Sender DID
    pub sender: String,
    /// Intended recipients (empty = broadcast)
    pub recipients: Vec<String>,
    /// Creation timestamp
    pub created: Timestamp,
    /// Expiration timestamp (optional)
    pub expires: Option<Timestamp>,
    /// Parent intent ID (for sub-intents)
    pub parent_id: Option<String>,
    /// Correlation ID (for request-response)
    pub correlation_id: Option<String>,
}

impl SemanticIntent {
    /// Create a new semantic intent.
    pub fn new(sender: &DIDKey, domain: IntentDomain, action: &str) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id: id.clone(),
            domain,
            action: action.to_string(),
            target: None,
            parameters: HashMap::new(),
            constraints: Vec::new(),
            priority: IntentPriority::Normal,
            sender: sender.id.clone(),
            recipients: Vec::new(),
            created: now(),
            expires: None,
            parent_id: None,
            correlation_id: Some(id),
        }
    }

    /// Set the target of the intent.
    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    /// Add a parameter.
    pub fn with_param(mut self, key: &str, value: serde_json::Value) -> Self {
        self.parameters.insert(key.to_string(), value);
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: IntentPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add a recipient.
    pub fn with_recipient(mut self, recipient: &DIDKey) -> Self {
        self.recipients.push(recipient.id.clone());
        self
    }

    /// Add a constraint.
    pub fn with_constraint(mut self, constraint_type: &str, value: serde_json::Value, required: bool) -> Self {
        self.constraints.push(IntentConstraint {
            constraint_type: constraint_type.to_string(),
            value,
            required,
        });
        self
    }

    /// Set expiration.
    pub fn with_expiration(mut self, expires: Timestamp) -> Self {
        self.expires = Some(expires);
        self
    }

    /// Set parent intent ID.
    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    /// Check if intent is expired.
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires {
            now() > exp
        } else {
            false
        }
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serialize to binary.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    /// Deserialize from binary.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }

    /// Get a parameter value.
    pub fn get_param<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.parameters
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Builder for creating intents with fluent API.
pub struct IntentBuilder {
    intent: SemanticIntent,
}

impl IntentBuilder {
    /// Create a new intent builder.
    pub fn new(sender: &DIDKey, domain: IntentDomain, action: &str) -> Self {
        Self {
            intent: SemanticIntent::new(sender, domain, action),
        }
    }

    /// Set target.
    pub fn target(mut self, target: &str) -> Self {
        self.intent = self.intent.with_target(target);
        self
    }

    /// Add parameter.
    pub fn param(mut self, key: &str, value: serde_json::Value) -> Self {
        self.intent = self.intent.with_param(key, value);
        self
    }

    /// Set priority.
    pub fn priority(mut self, priority: IntentPriority) -> Self {
        self.intent = self.intent.with_priority(priority);
        self
    }

    /// Build the intent.
    pub fn build(self) -> SemanticIntent {
        self.intent
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

    #[test]
    fn test_intent_creation() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");
        
        assert!(!intent.id.is_empty());
        assert_eq!(intent.domain, IntentDomain::Data);
        assert_eq!(intent.action, "read");
        assert_eq!(intent.sender, sender.id);
    }

    #[test]
    fn test_intent_with_params() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Compute, "process")
            .with_param("input", serde_json::json!("data"))
            .with_param("count", serde_json::json!(10));

        assert_eq!(intent.parameters.len(), 2);
        let count: Option<i32> = intent.get_param("count");
        assert_eq!(count, Some(10));
    }

    #[test]
    fn test_intent_serialization() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Communication, "send")
            .with_target("did:hido:target")
            .with_priority(IntentPriority::High);

        let json = intent.to_json().unwrap();
        let parsed = SemanticIntent::from_json(&json).unwrap();
        
        assert_eq!(parsed.id, intent.id);
        assert_eq!(parsed.action, intent.action);
        assert_eq!(parsed.priority, IntentPriority::High);
    }

    #[test]
    fn test_intent_binary_serialization() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Coordination, "delegate");

        let bytes = intent.to_bytes().unwrap();
        let parsed = SemanticIntent::from_bytes(&bytes).unwrap();
        
        assert_eq!(parsed.id, intent.id);
    }

    #[test]
    fn test_intent_builder() {
        let sender = create_test_did();
        let intent = IntentBuilder::new(&sender, IntentDomain::Data, "transform")
            .target("dataset")
            .param("format", serde_json::json!("json"))
            .priority(IntentPriority::Critical)
            .build();

        assert_eq!(intent.target, Some("dataset".to_string()));
        assert_eq!(intent.priority, IntentPriority::Critical);
    }

    #[test]
    fn test_intent_domain_custom() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Custom("ml".to_string()), "train");
        
        assert_eq!(intent.domain, IntentDomain::Custom("ml".to_string()));
    }
}

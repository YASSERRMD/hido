//! Audit entry structure.
//!
//! Backend-agnostic audit record.

use crate::core::{now, Hash256, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique entry identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(pub String);

impl EntryId {
    /// Create a new entry ID.
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    /// Generate a unique ID.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Get the ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Entry type/category.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    /// Agent action
    AgentAction,
    /// Decision/consensus
    Decision,
    /// Access event
    Access,
    /// Configuration change
    ConfigChange,
    /// Compliance event
    Compliance,
    /// Custom type
    Custom(String),
}

/// Audit entry severity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntrySeverity {
    /// Debug/trace level
    Debug,
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Critical
    Critical,
}

/// An audit entry (backend-agnostic).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: EntryId,
    /// Entry type
    pub entry_type: EntryType,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Actor (agent/user ID)
    pub actor: String,
    /// Action performed
    pub action: String,
    /// Target resource
    pub target: String,
    /// Outcome (success/failure)
    pub success: bool,
    /// Severity level
    pub severity: EntrySeverity,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Content hash (for verification)
    pub hash: Option<Hash256>,
    /// Signature (optional)
    pub signature: Option<Vec<u8>>,
    /// Parent entry ID (for chaining)
    pub parent_id: Option<EntryId>,
}

impl AuditEntry {
    /// Create a new audit entry.
    pub fn new(actor: &str, action: &str, target: &str) -> Self {
        Self {
            id: EntryId::generate(),
            entry_type: EntryType::AgentAction,
            timestamp: now(),
            actor: actor.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            success: true,
            severity: EntrySeverity::Info,
            metadata: HashMap::new(),
            hash: None,
            signature: None,
            parent_id: None,
        }
    }

    /// Set entry type.
    pub fn with_type(mut self, entry_type: EntryType) -> Self {
        self.entry_type = entry_type;
        self
    }

    /// Mark as failure.
    pub fn failed(mut self) -> Self {
        self.success = false;
        self
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: EntrySeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: &str, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.metadata.insert(key.to_string(), v);
        }
        self
    }

    /// Set parent ID (for chaining).
    pub fn with_parent(mut self, parent_id: EntryId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Compute and set content hash.
    pub fn compute_hash(&mut self) -> Hash256 {
        use crate::uail::crypto::sha3_256_multi;
        
        let data = format!(
            "{}:{}:{}:{}:{}:{}",
            self.id, self.actor, self.action, self.target, self.timestamp, self.success
        );
        
        let hash = sha3_256_multi(&[data.as_bytes()]);
        self.hash = Some(hash.clone());
        hash
    }

    /// Verify the entry's hash.
    pub fn verify_hash(&self) -> bool {
        if let Some(stored_hash) = &self.hash {
            use crate::uail::crypto::sha3_256_multi;
            
            let data = format!(
                "{}:{}:{}:{}:{}:{}",
                self.id, self.actor, self.action, self.target, self.timestamp, self.success
            );
            
            let computed = sha3_256_multi(&[data.as_bytes()]);
            &computed == stored_hash
        } else {
            false
        }
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> crate::core::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> crate::core::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_id() {
        let id = EntryId::new("test-id");
        assert_eq!(id.as_str(), "test-id");
        assert_eq!(id.to_string(), "test-id");
    }

    #[test]
    fn test_entry_id_generate() {
        let id1 = EntryId::generate();
        let id2 = EntryId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_audit_entry_creation() {
        let entry = AuditEntry::new("agent-1", "execute", "task-1");
        assert_eq!(entry.actor, "agent-1");
        assert_eq!(entry.action, "execute");
        assert!(entry.success);
    }

    #[test]
    fn test_entry_with_metadata() {
        let entry = AuditEntry::new("agent-1", "execute", "task-1")
            .with_metadata("key", "value")
            .with_severity(EntrySeverity::Warning);

        assert!(entry.metadata.contains_key("key"));
        assert_eq!(entry.severity, EntrySeverity::Warning);
    }

    #[test]
    fn test_entry_failed() {
        let entry = AuditEntry::new("agent-1", "execute", "task-1").failed();
        assert!(!entry.success);
    }

    #[test]
    fn test_entry_hash() {
        let mut entry = AuditEntry::new("agent-1", "execute", "task-1");
        entry.compute_hash();
        
        assert!(entry.hash.is_some());
        assert!(entry.verify_hash());
    }

    #[test]
    fn test_entry_serialization() {
        let entry = AuditEntry::new("agent-1", "execute", "task-1");
        let json = entry.to_json().unwrap();
        let parsed = AuditEntry::from_json(&json).unwrap();
        
        assert_eq!(parsed.actor, entry.actor);
        assert_eq!(parsed.action, entry.action);
    }
}

//! Audit logging for compliance.
//!
//! Provides immutable audit trail.

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Audit action types.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditAction {
    /// Agent registered
    AgentRegistered,
    /// Agent removed
    AgentRemoved,
    /// Action executed
    ActionExecuted,
    /// Consensus reached
    ConsensusReached,
    /// Access granted
    AccessGranted,
    /// Access denied
    AccessDenied,
    /// Data accessed
    DataAccessed,
    /// Data modified
    DataModified,
    /// Data deleted
    DataDeleted,
    /// Configuration changed
    ConfigChanged,
    /// Compliance violation
    ComplianceViolation,
    /// Custom action
    Custom(String),
}

/// An audit log entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: String,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Actor (who performed the action)
    pub actor: String,
    /// Action type
    pub action: AuditAction,
    /// Resource affected
    pub resource: String,
    /// Outcome (success/failure)
    pub success: bool,
    /// Additional details
    pub details: HashMap<String, serde_json::Value>,
    /// IP address (if applicable)
    pub ip_address: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry.
    pub fn new(actor: &str, action: AuditAction, resource: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now(),
            actor: actor.to_string(),
            action,
            resource: resource.to_string(),
            success: true,
            details: HashMap::new(),
            ip_address: None,
            session_id: None,
        }
    }

    /// Mark as failure.
    pub fn failed(mut self) -> Self {
        self.success = false;
        self
    }

    /// Add detail.
    pub fn with_detail(mut self, key: &str, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.details.insert(key.to_string(), v);
        }
        self
    }

    /// Set IP address.
    pub fn with_ip(mut self, ip: &str) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }

    /// Set session ID.
    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }
}

/// Audit filter for queries.
#[derive(Clone, Debug, Default)]
pub struct AuditFilter {
    /// Filter by actor
    pub actor: Option<String>,
    /// Filter by action
    pub action: Option<AuditAction>,
    /// Filter by resource
    pub resource: Option<String>,
    /// Date from
    pub date_from: Option<Timestamp>,
    /// Date to
    pub date_to: Option<Timestamp>,
    /// Success only
    pub success_only: Option<bool>,
}

impl AuditFilter {
    /// Create a new filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by actor.
    pub fn by_actor(mut self, actor: &str) -> Self {
        self.actor = Some(actor.to_string());
        self
    }

    /// Filter by action.
    pub fn by_action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Check if an entry matches this filter.
    pub fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(actor) = &self.actor {
            if &entry.actor != actor {
                return false;
            }
        }

        if let Some(action) = &self.action {
            if &entry.action != action {
                return false;
            }
        }

        if let Some(resource) = &self.resource {
            if &entry.resource != resource {
                return false;
            }
        }

        if let Some(from) = self.date_from {
            if entry.timestamp < from {
                return false;
            }
        }

        if let Some(to) = self.date_to {
            if entry.timestamp > to {
                return false;
            }
        }

        if let Some(success_only) = self.success_only {
            if success_only && !entry.success {
                return false;
            }
        }

        true
    }
}

/// Audit logger.
pub struct AuditLogger {
    /// Log entries
    entries: RwLock<Vec<AuditEntry>>,
    /// Maximum entries to keep
    max_entries: usize,
}

impl AuditLogger {
    /// Create a new logger.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            max_entries,
        }
    }

    /// Log an entry.
    pub fn log(&self, entry: AuditEntry) {
        let mut entries = self.entries.write().unwrap();
        
        // Trim if at capacity
        if entries.len() >= self.max_entries {
            entries.remove(0);
        }
        
        entries.push(entry);
    }

    /// Query entries.
    pub fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        self.entries
            .read()
            .unwrap()
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }

    /// Get all entries.
    pub fn all(&self) -> Vec<AuditEntry> {
        self.entries.read().unwrap().clone()
    }

    /// Get entry count.
    pub fn count(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Get entries for a specific actor.
    pub fn entries_for_actor(&self, actor: &str) -> Vec<AuditEntry> {
        self.query(&AuditFilter::new().by_actor(actor))
    }

    /// Get recent entries.
    pub fn recent(&self, n: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter().rev().take(n).cloned().collect()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry() {
        let entry = AuditEntry::new("user-1", AuditAction::DataAccessed, "resource-1")
            .with_detail("field", "value")
            .with_ip("192.168.1.1");

        assert!(entry.success);
        assert_eq!(entry.actor, "user-1");
        assert!(entry.ip_address.is_some());
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new(100);

        logger.log(AuditEntry::new("user-1", AuditAction::DataAccessed, "res-1"));
        logger.log(AuditEntry::new("user-2", AuditAction::DataModified, "res-2"));

        assert_eq!(logger.count(), 2);
    }

    #[test]
    fn test_query_filter() {
        let logger = AuditLogger::default();

        logger.log(AuditEntry::new("user-1", AuditAction::DataAccessed, "res-1"));
        logger.log(AuditEntry::new("user-2", AuditAction::DataAccessed, "res-2"));
        logger.log(AuditEntry::new("user-1", AuditAction::DataModified, "res-3"));

        let entries = logger.query(&AuditFilter::new().by_actor("user-1"));
        assert_eq!(entries.len(), 2);

        let entries = logger.query(&AuditFilter::new().by_action(AuditAction::DataModified));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_max_entries() {
        let logger = AuditLogger::new(3);

        for i in 0..5 {
            logger.log(AuditEntry::new(&format!("user-{}", i), AuditAction::DataAccessed, "res"));
        }

        assert_eq!(logger.count(), 3);
    }

    #[test]
    fn test_recent() {
        let logger = AuditLogger::default();

        for i in 0..5 {
            logger.log(AuditEntry::new(&format!("user-{}", i), AuditAction::DataAccessed, "res"));
        }

        let recent = logger.recent(2);
        assert_eq!(recent.len(), 2);
    }
}

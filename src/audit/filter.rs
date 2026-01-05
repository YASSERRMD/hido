//! Query filters for audit entries.
//!
//! Enables flexible querying across backends.

use crate::audit::entry::{EntryId, EntryType, EntrySeverity};
use crate::core::Timestamp;
use serde::{Deserialize, Serialize};

/// Filter for querying audit entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    /// Filter by actor
    pub actor: Option<String>,
    /// Filter by action
    pub action: Option<String>,
    /// Filter by target
    pub target: Option<String>,
    /// Filter by entry type
    pub entry_type: Option<EntryType>,
    /// Filter by severity (minimum)
    pub min_severity: Option<EntrySeverity>,
    /// Filter by date from
    pub date_from: Option<Timestamp>,
    /// Filter by date to
    pub date_to: Option<Timestamp>,
    /// Filter by success/failure
    pub success: Option<bool>,
    /// Filter by parent ID
    pub parent_id: Option<EntryId>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl AuditFilter {
    /// Create a new empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by actor.
    pub fn by_actor(mut self, actor: &str) -> Self {
        self.actor = Some(actor.to_string());
        self
    }

    /// Filter by action.
    pub fn by_action(mut self, action: &str) -> Self {
        self.action = Some(action.to_string());
        self
    }

    /// Filter by target.
    pub fn by_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    /// Filter by entry type.
    pub fn by_type(mut self, entry_type: EntryType) -> Self {
        self.entry_type = Some(entry_type);
        self
    }

    /// Filter by minimum severity.
    pub fn by_severity(mut self, severity: EntrySeverity) -> Self {
        self.min_severity = Some(severity);
        self
    }

    /// Filter by date range.
    pub fn by_date_range(mut self, from: Timestamp, to: Timestamp) -> Self {
        self.date_from = Some(from);
        self.date_to = Some(to);
        self
    }

    /// Filter successful only.
    pub fn successful_only(mut self) -> Self {
        self.success = Some(true);
        self
    }

    /// Filter failed only.
    pub fn failed_only(mut self) -> Self {
        self.success = Some(false);
        self
    }

    /// Filter by parent chain.
    pub fn by_parent(mut self, parent_id: EntryId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set result limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set pagination offset.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Check if an entry matches this filter.
    pub fn matches(&self, entry: &crate::audit::entry::AuditEntry) -> bool {
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

        if let Some(target) = &self.target {
            if &entry.target != target {
                return false;
            }
        }

        if let Some(entry_type) = &self.entry_type {
            if &entry.entry_type != entry_type {
                return false;
            }
        }

        if let Some(success) = self.success {
            if entry.success != success {
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

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::entry::AuditEntry;

    #[test]
    fn test_filter_creation() {
        let filter = AuditFilter::new();
        assert!(filter.actor.is_none());
        assert!(filter.limit.is_none());
    }

    #[test]
    fn test_filter_by_actor() {
        let filter = AuditFilter::new().by_actor("agent-1");
        assert_eq!(filter.actor, Some("agent-1".to_string()));
    }

    #[test]
    fn test_filter_chain() {
        let filter = AuditFilter::new()
            .by_actor("agent-1")
            .by_action("execute")
            .successful_only()
            .with_limit(10);

        assert!(filter.actor.is_some());
        assert!(filter.action.is_some());
        assert_eq!(filter.success, Some(true));
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn test_filter_matches() {
        let entry = AuditEntry::new("agent-1", "execute", "task-1");
        
        let filter_match = AuditFilter::new().by_actor("agent-1");
        assert!(filter_match.matches(&entry));

        let filter_no_match = AuditFilter::new().by_actor("agent-2");
        assert!(!filter_no_match.matches(&entry));
    }

    #[test]
    fn test_filter_matches_multiple() {
        let entry = AuditEntry::new("agent-1", "execute", "task-1");
        
        let filter = AuditFilter::new()
            .by_actor("agent-1")
            .by_action("execute");
        assert!(filter.matches(&entry));

        let filter_wrong_action = AuditFilter::new()
            .by_actor("agent-1")
            .by_action("delete");
        assert!(!filter_wrong_action.matches(&entry));
    }
}

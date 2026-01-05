//! Compliance rules engine.
//!
//! Checks compliance with various frameworks.

use crate::compliance::audit::{AuditAction, AuditEntry};
use crate::core::now;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compliance frameworks.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceFramework {
    /// GDPR
    GDPR,
    /// SOC2
    SOC2,
    /// HIPAA
    HIPAA,
    /// PCI DSS
    PCIDSS,
    /// ISO 27001
    ISO27001,
    /// CCPA
    CCPA,
}

/// Violation severity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Critical
    Critical,
}

/// A compliance rule.
#[derive(Clone, Debug)]
pub struct ComplianceRule {
    /// Rule ID
    pub rule_id: String,
    /// Framework
    pub framework: ComplianceFramework,
    /// Requirement description
    pub requirement: String,
    /// Severity if violated
    pub severity: ViolationSeverity,
    /// Check function
    check_fn: fn(&AuditEntry) -> bool,
}

impl ComplianceRule {
    /// Create a new rule.
    pub fn new(
        rule_id: &str,
        framework: ComplianceFramework,
        requirement: &str,
        check_fn: fn(&AuditEntry) -> bool,
    ) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            framework,
            requirement: requirement.to_string(),
            severity: ViolationSeverity::Warning,
            check_fn,
        }
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: ViolationSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Check if an entry complies.
    pub fn check(&self, entry: &AuditEntry) -> bool {
        (self.check_fn)(entry)
    }
}

/// A compliance violation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceViolation {
    /// Rule ID
    pub rule_id: String,
    /// Framework
    pub framework: ComplianceFramework,
    /// Timestamp
    pub timestamp: crate::core::Timestamp,
    /// Entry that violated
    pub entry_id: String,
    /// Severity
    pub severity: ViolationSeverity,
    /// Remediation suggestion
    pub remediation: String,
}

/// Compliance status.
#[derive(Clone, Debug)]
pub struct ComplianceStatus {
    /// Framework
    pub framework: ComplianceFramework,
    /// Is compliant
    pub compliant: bool,
    /// Violation count
    pub violation_count: usize,
    /// Coverage (% of rules checked)
    pub coverage: f32,
}

/// Compliance engine.
pub struct ComplianceEngine {
    /// Rules
    pub rules: Vec<ComplianceRule>,
    /// Violations
    pub violations: Vec<ComplianceViolation>,
    /// Entries checked
    entries_checked: usize,
}

impl ComplianceEngine {
    /// Create a new engine.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            violations: Vec::new(),
            entries_checked: 0,
        }
    }

    /// Create with default rules.
    pub fn with_defaults() -> Self {
        let mut engine = Self::new();

        // GDPR: Data access must be logged
        engine.add_rule(
            ComplianceRule::new(
                "GDPR-001",
                ComplianceFramework::GDPR,
                "All data access must be logged with purpose",
                |entry| {
                    if matches!(entry.action, AuditAction::DataAccessed) {
                        entry.details.contains_key("purpose")
                    } else {
                        true
                    }
                },
            )
            .with_severity(ViolationSeverity::Critical),
        );

        // SOC2: Failed access attempts must be logged
        engine.add_rule(ComplianceRule::new(
            "SOC2-001",
            ComplianceFramework::SOC2,
            "Failed access attempts must be logged",
            |entry| {
                if matches!(entry.action, AuditAction::AccessDenied) {
                    entry.details.contains_key("reason")
                } else {
                    true
                }
            },
        ));

        // HIPAA: PHI access requires authorization
        engine.add_rule(
            ComplianceRule::new(
                "HIPAA-001",
                ComplianceFramework::HIPAA,
                "PHI access requires documented authorization",
                |entry| {
                    if let Some(resource_type) = entry.details.get("resource_type") {
                        if resource_type == "PHI" {
                            return entry.details.contains_key("authorization");
                        }
                    }
                    true
                },
            )
            .with_severity(ViolationSeverity::Critical),
        );

        engine
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: ComplianceRule) {
        self.rules.push(rule);
    }

    /// Check an entry against all rules.
    pub fn check_entry(&mut self, entry: &AuditEntry) -> Vec<ComplianceViolation> {
        self.entries_checked += 1;
        let mut violations = Vec::new();

        for rule in &self.rules {
            if !rule.check(entry) {
                let violation = ComplianceViolation {
                    rule_id: rule.rule_id.clone(),
                    framework: rule.framework.clone(),
                    timestamp: now(),
                    entry_id: entry.id.clone(),
                    severity: rule.severity.clone(),
                    remediation: format!("Review and ensure: {}", rule.requirement),
                };
                violations.push(violation.clone());
                self.violations.push(violation);
            }
        }

        violations
    }

    /// Get status for a framework.
    pub fn get_status(&self, framework: ComplianceFramework) -> ComplianceStatus {
        let violation_count = self
            .violations
            .iter()
            .filter(|v| v.framework == framework)
            .count();

        let rule_count = self
            .rules
            .iter()
            .filter(|r| r.framework == framework)
            .count();

        ComplianceStatus {
            framework,
            compliant: violation_count == 0,
            violation_count,
            coverage: if rule_count > 0 {
                100.0
            } else {
                0.0
            },
        }
    }

    /// Get all violations.
    pub fn violations(&self) -> &[ComplianceViolation] {
        &self.violations
    }

    /// Get violations for a framework.
    pub fn violations_for(&self, framework: ComplianceFramework) -> Vec<&ComplianceViolation> {
        self.violations
            .iter()
            .filter(|v| v.framework == framework)
            .collect()
    }

    /// Clear violations.
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }

    /// Get entries checked count.
    pub fn entries_checked(&self) -> usize {
        self.entries_checked
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ComplianceEngine::new();
        assert!(engine.rules.is_empty());
    }

    #[test]
    fn test_with_defaults() {
        let engine = ComplianceEngine::with_defaults();
        assert!(!engine.rules.is_empty());
    }

    #[test]
    fn test_check_entry_compliant() {
        let mut engine = ComplianceEngine::with_defaults();

        let entry = AuditEntry::new("user-1", AuditAction::DataAccessed, "resource-1")
            .with_detail("purpose", "legitimate business use");

        let violations = engine.check_entry(&entry);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_check_entry_violation() {
        let mut engine = ComplianceEngine::with_defaults();

        // Data access without purpose (GDPR violation)
        let entry = AuditEntry::new("user-1", AuditAction::DataAccessed, "resource-1");

        let violations = engine.check_entry(&entry);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_get_status() {
        let engine = ComplianceEngine::with_defaults();
        let status = engine.get_status(ComplianceFramework::GDPR);

        assert!(status.compliant);
        assert_eq!(status.violation_count, 0);
    }

    #[test]
    fn test_violation_severity() {
        let rule = ComplianceRule::new(
            "TEST-001",
            ComplianceFramework::SOC2,
            "Test rule",
            |_| true,
        )
        .with_severity(ViolationSeverity::Critical);

        assert_eq!(rule.severity, ViolationSeverity::Critical);
    }
}

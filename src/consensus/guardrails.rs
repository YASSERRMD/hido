//! Ethical guardrails for decision validation.
//!
//! Enforces transparency, fairness, safety, privacy, and legality rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of ethical rules.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleType {
    /// Action must be explainable
    Transparency,
    /// No bias against agents
    Fairness,
    /// Risk assessment below threshold
    Safety,
    /// No sensitive data exposure
    Privacy,
    /// Complies with regulations
    Legality,
}

/// Action to take when guardrail is triggered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardrailAction {
    /// Allow the action
    Approve,
    /// Block the action
    Reject,
    /// Require human review
    RequireApproval,
    /// Escalate to higher authority
    Escalate,
}

/// A condition for a guardrail rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Condition {
    /// Field to check
    pub field: String,
    /// Operator (eq, ne, gt, lt, contains)
    pub operator: String,
    /// Value to compare
    pub value: serde_json::Value,
}

impl Condition {
    /// Create a new condition.
    pub fn new(field: &str, operator: &str, value: serde_json::Value) -> Self {
        Self {
            field: field.to_string(),
            operator: operator.to_string(),
            value,
        }
    }

    /// Evaluate condition against a context.
    pub fn evaluate(&self, context: &HashMap<String, serde_json::Value>) -> bool {
        let actual = match context.get(&self.field) {
            Some(v) => v,
            None => return false,
        };

        match self.operator.as_str() {
            "eq" => actual == &self.value,
            "ne" => actual != &self.value,
            "gt" => {
                if let (Some(a), Some(b)) = (actual.as_f64(), self.value.as_f64()) {
                    a > b
                } else {
                    false
                }
            }
            "lt" => {
                if let (Some(a), Some(b)) = (actual.as_f64(), self.value.as_f64()) {
                    a < b
                } else {
                    false
                }
            }
            "contains" => {
                if let (Some(a), Some(b)) = (actual.as_str(), self.value.as_str()) {
                    a.contains(b)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// A guardrail rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardrailRule {
    /// Rule ID
    pub rule_id: String,
    /// Rule type
    pub rule_type: RuleType,
    /// Conditions that trigger the rule
    pub conditions: Vec<Condition>,
    /// Action to take when triggered
    pub action: GuardrailAction,
    /// Rule description
    pub description: String,
    /// Severity (1-10)
    pub severity: u8,
}

impl GuardrailRule {
    /// Create a new rule.
    pub fn new(
        rule_id: &str,
        rule_type: RuleType,
        action: GuardrailAction,
        description: &str,
    ) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            rule_type,
            conditions: Vec::new(),
            action,
            description: description.to_string(),
            severity: 5,
        }
    }

    /// Add a condition.
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: u8) -> Self {
        self.severity = severity.clamp(1, 10);
        self
    }

    /// Check if rule is violated by context.
    pub fn is_violated(&self, context: &HashMap<String, serde_json::Value>) -> bool {
        // All conditions must be true for the rule to be triggered
        !self.conditions.is_empty() && self.conditions.iter().all(|c| c.evaluate(context))
    }
}

/// Record of a guardrail violation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViolationRecord {
    /// Rule that was violated
    pub rule_id: String,
    /// Timestamp
    pub timestamp: crate::core::Timestamp,
    /// Context at time of violation
    pub context: HashMap<String, serde_json::Value>,
    /// Action taken
    pub action_taken: GuardrailAction,
}

/// Result of ethical evaluation.
#[derive(Clone, Debug)]
pub struct EthicalEvaluation {
    /// Whether all guardrails passed
    pub passes: bool,
    /// Violated rules
    pub violations: Vec<GuardrailRule>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Overall risk score (0-1)
    pub risk_score: f32,
    /// Required action
    pub required_action: Option<GuardrailAction>,
}

/// Ethical guardrail system.
pub struct EthicalGuardrail {
    /// Active rules
    pub rules: Vec<GuardrailRule>,
    /// Violation log
    pub violation_log: Vec<ViolationRecord>,
    /// Statistics
    stats: GuardrailStats,
}

/// Statistics for guardrail system.
#[derive(Clone, Debug, Default)]
pub struct GuardrailStats {
    pub total_evaluations: u64,
    pub violations_detected: u64,
    pub violations_by_type: HashMap<RuleType, u64>,
}

impl EthicalGuardrail {
    /// Create a new guardrail system.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            violation_log: Vec::new(),
            stats: GuardrailStats::default(),
        }
    }

    /// Create with default rules.
    pub fn with_defaults() -> Self {
        let mut guardrail = Self::new();

        // Transparency rule
        guardrail.add_rule(
            GuardrailRule::new(
                "transparency-1",
                RuleType::Transparency,
                GuardrailAction::RequireApproval,
                "High-impact decisions require explanation",
            )
            .with_condition(Condition::new("impact", "gt", serde_json::json!(0.8)))
            .with_condition(Condition::new("has_explanation", "eq", serde_json::json!(false)))
            .with_severity(7),
        );

        // Safety rule
        guardrail.add_rule(
            GuardrailRule::new(
                "safety-1",
                RuleType::Safety,
                GuardrailAction::Reject,
                "High-risk actions blocked",
            )
            .with_condition(Condition::new("risk_score", "gt", serde_json::json!(0.9)))
            .with_severity(10),
        );

        // Privacy rule
        guardrail.add_rule(
            GuardrailRule::new(
                "privacy-1",
                RuleType::Privacy,
                GuardrailAction::Reject,
                "Sensitive data exposure blocked",
            )
            .with_condition(Condition::new("contains_pii", "eq", serde_json::json!(true)))
            .with_severity(9),
        );

        guardrail
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: GuardrailRule) {
        self.rules.push(rule);
    }

    /// Remove a rule by ID.
    pub fn remove_rule(&mut self, rule_id: &str) {
        self.rules.retain(|r| r.rule_id != rule_id);
    }

    /// Evaluate decision against guardrails.
    pub fn evaluate(&mut self, context: &HashMap<String, serde_json::Value>) -> EthicalEvaluation {
        self.stats.total_evaluations += 1;

        let mut violations = Vec::new();
        let mut recommendations = Vec::new();
        let mut max_severity = 0u8;
        let mut required_action: Option<GuardrailAction> = None;

        for rule in &self.rules {
            if rule.is_violated(context) {
                violations.push(rule.clone());
                self.stats.violations_detected += 1;
                *self.stats.violations_by_type.entry(rule.rule_type.clone()).or_insert(0) += 1;

                // Record violation
                self.violation_log.push(ViolationRecord {
                    rule_id: rule.rule_id.clone(),
                    timestamp: crate::core::now(),
                    context: context.clone(),
                    action_taken: rule.action.clone(),
                });

                // Generate recommendation
                recommendations.push(format!(
                    "[{}] {}: {}",
                    rule.rule_type_str(),
                    rule.rule_id,
                    rule.description
                ));

                // Track highest severity action
                if rule.severity > max_severity {
                    max_severity = rule.severity;
                    required_action = Some(rule.action.clone());
                }
            }
        }

        let risk_score = if !violations.is_empty() {
            max_severity as f32 / 10.0
        } else {
            0.0
        };

        EthicalEvaluation {
            passes: violations.is_empty(),
            violations,
            recommendations,
            risk_score,
            required_action,
        }
    }

    /// Get statistics.
    pub fn stats(&self) -> &GuardrailStats {
        &self.stats
    }

    /// Get violation log.
    pub fn violations(&self) -> &[ViolationRecord] {
        &self.violation_log
    }

    /// Clear violation log.
    pub fn clear_log(&mut self) {
        self.violation_log.clear();
    }
}

impl Default for EthicalGuardrail {
    fn default() -> Self {
        Self::new()
    }
}

impl GuardrailRule {
    fn rule_type_str(&self) -> &'static str {
        match self.rule_type {
            RuleType::Transparency => "TRANSPARENCY",
            RuleType::Fairness => "FAIRNESS",
            RuleType::Safety => "SAFETY",
            RuleType::Privacy => "PRIVACY",
            RuleType::Legality => "LEGALITY",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardrail_creation() {
        let guardrail = EthicalGuardrail::new();
        assert!(guardrail.rules.is_empty());
    }

    #[test]
    fn test_add_rule() {
        let mut guardrail = EthicalGuardrail::new();
        guardrail.add_rule(GuardrailRule::new(
            "test-1",
            RuleType::Safety,
            GuardrailAction::Reject,
            "Test rule",
        ));
        assert_eq!(guardrail.rules.len(), 1);
    }

    #[test]
    fn test_condition_evaluation() {
        let mut context = HashMap::new();
        context.insert("score".to_string(), serde_json::json!(0.9));

        let condition = Condition::new("score", "gt", serde_json::json!(0.5));
        assert!(condition.evaluate(&context));

        let condition2 = Condition::new("score", "lt", serde_json::json!(0.5));
        assert!(!condition2.evaluate(&context));
    }

    #[test]
    fn test_rule_violation() {
        let rule = GuardrailRule::new(
            "high-risk",
            RuleType::Safety,
            GuardrailAction::Reject,
            "Block high risk",
        )
        .with_condition(Condition::new("risk", "gt", serde_json::json!(0.8)));

        let mut context = HashMap::new();
        context.insert("risk".to_string(), serde_json::json!(0.9));
        assert!(rule.is_violated(&context));

        context.insert("risk".to_string(), serde_json::json!(0.5));
        assert!(!rule.is_violated(&context));
    }

    #[test]
    fn test_evaluate_passes() {
        let mut guardrail = EthicalGuardrail::with_defaults();
        let mut context = HashMap::new();
        context.insert("risk_score".to_string(), serde_json::json!(0.3));
        context.insert("contains_pii".to_string(), serde_json::json!(false));
        context.insert("has_explanation".to_string(), serde_json::json!(true));

        let result = guardrail.evaluate(&context);
        assert!(result.passes);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_evaluate_fails() {
        let mut guardrail = EthicalGuardrail::with_defaults();
        let mut context = HashMap::new();
        context.insert("risk_score".to_string(), serde_json::json!(0.95));

        let result = guardrail.evaluate(&context);
        assert!(!result.passes);
        assert!(!result.violations.is_empty());
        assert_eq!(result.required_action, Some(GuardrailAction::Reject));
    }

    #[test]
    fn test_stats_tracking() {
        let mut guardrail = EthicalGuardrail::with_defaults();
        let mut context = HashMap::new();
        context.insert("risk_score".to_string(), serde_json::json!(0.95));

        guardrail.evaluate(&context);
        guardrail.evaluate(&context);

        let stats = guardrail.stats();
        assert_eq!(stats.total_evaluations, 2);
        assert!(stats.violations_detected > 0);
    }

    #[test]
    fn test_with_severity() {
        let rule = GuardrailRule::new(
            "test",
            RuleType::Safety,
            GuardrailAction::Reject,
            "Test",
        )
        .with_severity(8);

        assert_eq!(rule.severity, 8);
    }

    #[test]
    fn test_privacy_rule() {
        let mut guardrail = EthicalGuardrail::with_defaults();
        let mut context = HashMap::new();
        context.insert("contains_pii".to_string(), serde_json::json!(true));

        let result = guardrail.evaluate(&context);
        assert!(!result.passes);
        assert!(result.violations.iter().any(|v| v.rule_type == RuleType::Privacy));
    }
}

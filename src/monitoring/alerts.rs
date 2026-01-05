//! Alerting system for monitoring.
//!
//! Provides alert rules, thresholds, and notifications.

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alert severity level.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Critical
    Critical,
    /// Emergency
    Emergency,
}

/// Alert state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertState {
    /// Not firing
    Inactive,
    /// Threshold exceeded but pending
    Pending,
    /// Actively firing
    Firing,
    /// Resolved
    Resolved,
}

/// An alert rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Metric to check
    pub metric: String,
    /// Condition
    pub condition: AlertCondition,
    /// Threshold value
    pub threshold: f64,
    /// Duration before firing
    pub for_duration_seconds: u64,
    /// Severity
    pub severity: AlertSeverity,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Annotations
    pub annotations: HashMap<String, String>,
}

impl AlertRule {
    /// Create a new alert rule.
    pub fn new(name: &str, metric: &str, condition: AlertCondition, threshold: f64) -> Self {
        Self {
            name: name.to_string(),
            metric: metric.to_string(),
            condition,
            threshold,
            for_duration_seconds: 0,
            severity: AlertSeverity::Warning,
            labels: HashMap::new(),
            annotations: HashMap::new(),
        }
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: AlertSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set duration.
    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.for_duration_seconds = seconds;
        self
    }

    /// Add annotation.
    pub fn with_annotation(mut self, key: &str, value: &str) -> Self {
        self.annotations.insert(key.to_string(), value.to_string());
        self
    }

    /// Check if value triggers this rule.
    pub fn evaluate(&self, value: f64) -> bool {
        match self.condition {
            AlertCondition::GreaterThan => value > self.threshold,
            AlertCondition::LessThan => value < self.threshold,
            AlertCondition::Equal => (value - self.threshold).abs() < 1e-10,
            AlertCondition::NotEqual => (value - self.threshold).abs() >= 1e-10,
            AlertCondition::GreaterOrEqual => value >= self.threshold,
            AlertCondition::LessOrEqual => value <= self.threshold,
        }
    }
}

/// Alert condition operators.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterOrEqual,
    LessOrEqual,
}

/// An active alert instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    /// Alert name (from rule)
    pub name: String,
    /// Current state
    pub state: AlertState,
    /// Severity
    pub severity: AlertSeverity,
    /// Current metric value
    pub value: f64,
    /// Threshold that was exceeded
    pub threshold: f64,
    /// When alert started pending
    pub started_at: Option<Timestamp>,
    /// When alert started firing
    pub fired_at: Option<Timestamp>,
    /// When alert resolved
    pub resolved_at: Option<Timestamp>,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Annotations
    pub annotations: HashMap<String, String>,
}

impl Alert {
    /// Create from a rule.
    pub fn from_rule(rule: &AlertRule, value: f64) -> Self {
        Self {
            name: rule.name.clone(),
            state: AlertState::Pending,
            severity: rule.severity.clone(),
            value,
            threshold: rule.threshold,
            started_at: Some(now()),
            fired_at: None,
            resolved_at: None,
            labels: rule.labels.clone(),
            annotations: rule.annotations.clone(),
        }
    }

    /// Transition to firing.
    pub fn fire(&mut self) {
        self.state = AlertState::Firing;
        self.fired_at = Some(now());
    }

    /// Resolve the alert.
    pub fn resolve(&mut self) {
        self.state = AlertState::Resolved;
        self.resolved_at = Some(now());
    }

    /// Check if firing.
    pub fn is_firing(&self) -> bool {
        self.state == AlertState::Firing
    }
}

/// Notification channel for alerts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationChannel {
    /// Email notification
    Email { to: Vec<String> },
    /// Slack notification
    Slack { webhook_url: String, channel: String },
    /// PagerDuty
    PagerDuty { service_key: String },
    /// Webhook
    Webhook { url: String },
}

/// Alert manager for handling alerts.
pub struct AlertManager {
    /// Alert rules
    rules: Vec<AlertRule>,
    /// Active alerts
    alerts: HashMap<String, Alert>,
    /// Notification channels
    channels: Vec<NotificationChannel>,
    /// Alert history
    history: Vec<Alert>,
}

impl AlertManager {
    /// Create a new alert manager.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            alerts: HashMap::new(),
            channels: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Add an alert rule.
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    /// Add a notification channel.
    pub fn add_channel(&mut self, channel: NotificationChannel) {
        self.channels.push(channel);
    }

    /// Evaluate a metric value against all rules.
    pub fn evaluate(&mut self, metric: &str, value: f64) {
        for rule in &self.rules {
            if rule.metric != metric {
                continue;
            }

            let triggered = rule.evaluate(value);
            let alert_key = rule.name.clone();

            if triggered {
                if let Some(alert) = self.alerts.get_mut(&alert_key) {
                    // Update existing alert
                    alert.value = value;
                    if alert.state == AlertState::Pending {
                        // Check if pending duration exceeded
                        if let Some(started) = alert.started_at {
                            let elapsed = now() - started;
                            if elapsed.num_seconds() >= rule.for_duration_seconds as i64 {
                                alert.fire();
                            }
                        }
                    }
                } else {
                    // Create new alert
                    let alert = Alert::from_rule(rule, value);
                    self.alerts.insert(alert_key, alert);
                }
            } else if let Some(alert) = self.alerts.get_mut(&alert_key) {
                // Resolve alert
                alert.resolve();
                self.history.push(alert.clone());
                self.alerts.remove(&alert_key);
            }
        }
    }

    /// Get firing alerts.
    pub fn firing_alerts(&self) -> Vec<&Alert> {
        self.alerts.values().filter(|a| a.is_firing()).collect()
    }

    /// Get all active alerts.
    pub fn active_alerts(&self) -> Vec<&Alert> {
        self.alerts.values().collect()
    }

    /// Get alert by name.
    pub fn get_alert(&self, name: &str) -> Option<&Alert> {
        self.alerts.get(name)
    }

    /// Get rules.
    pub fn rules(&self) -> &[AlertRule] {
        &self.rules
    }

    /// Get history.
    pub fn history(&self) -> &[Alert] {
        &self.history
    }

    /// Clear resolved alerts from history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_rule_creation() {
        let rule = AlertRule::new("high_cpu", "cpu_usage", AlertCondition::GreaterThan, 80.0)
            .with_severity(AlertSeverity::Critical)
            .with_duration(60);

        assert_eq!(rule.name, "high_cpu");
        assert_eq!(rule.threshold, 80.0);
        assert_eq!(rule.severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_rule_evaluate() {
        let rule = AlertRule::new("test", "metric", AlertCondition::GreaterThan, 50.0);
        assert!(rule.evaluate(60.0));
        assert!(!rule.evaluate(40.0));

        let rule2 = AlertRule::new("test", "metric", AlertCondition::LessThan, 50.0);
        assert!(!rule2.evaluate(60.0));
        assert!(rule2.evaluate(40.0));
    }

    #[test]
    fn test_alert_manager() {
        let mut manager = AlertManager::new();
        manager.add_rule(
            AlertRule::new("high_cpu", "cpu_usage", AlertCondition::GreaterThan, 80.0)
        );

        manager.evaluate("cpu_usage", 90.0);
        assert_eq!(manager.active_alerts().len(), 1);

        manager.evaluate("cpu_usage", 50.0);
        assert_eq!(manager.active_alerts().len(), 0);
        assert_eq!(manager.history().len(), 1);
    }

    #[test]
    fn test_alert_firing() {
        let rule = AlertRule::new("test", "metric", AlertCondition::GreaterThan, 50.0);
        let mut alert = Alert::from_rule(&rule, 60.0);

        assert_eq!(alert.state, AlertState::Pending);

        alert.fire();
        assert!(alert.is_firing());

        alert.resolve();
        assert_eq!(alert.state, AlertState::Resolved);
    }

    #[test]
    fn test_notification_channels() {
        let mut manager = AlertManager::new();
        manager.add_channel(NotificationChannel::Email {
            to: vec!["admin@example.com".to_string()],
        });
        manager.add_channel(NotificationChannel::Slack {
            webhook_url: "https://hooks.slack.com/...".to_string(),
            channel: "#alerts".to_string(),
        });

        assert_eq!(manager.channels.len(), 2);
    }
}

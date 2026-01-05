//! SLA metrics tracking.
//!
//! Tracks metrics and detects breaches.

use crate::core::{now, Timestamp};
use crate::sla::agreement::{SLAContract, SLOMetric, SLOTarget};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tracked metric value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackedMetric {
    /// Metric type
    pub metric: SLOMetric,
    /// Current value
    pub value: f64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Sample count
    pub sample_count: u64,
}

/// SLA metric tracker.
pub struct SLATracker {
    /// Contract being tracked
    contract_id: String,
    /// Current metric values
    metrics: HashMap<SLOMetric, TrackedMetric>,
    /// Metric history (for averaging)
    history: HashMap<SLOMetric, Vec<(Timestamp, f64)>>,
    /// Breach events
    breaches: Vec<BreachEvent>,
    /// Window size for calculations
    window_size: usize,
}

/// An SLA breach event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BreachEvent {
    /// Contract ID
    pub contract_id: String,
    /// Breached metric
    pub metric: SLOMetric,
    /// Target value
    pub target: f64,
    /// Actual value
    pub actual: f64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Duration of breach (seconds)
    pub duration_seconds: u64,
}

impl SLATracker {
    /// Create a new tracker.
    pub fn new(contract_id: &str) -> Self {
        Self {
            contract_id: contract_id.to_string(),
            metrics: HashMap::new(),
            history: HashMap::new(),
            breaches: Vec::new(),
            window_size: 100,
        }
    }

    /// Record a metric value.
    pub fn record(&mut self, metric: SLOMetric, value: f64) {
        let timestamp = now();

        // Update current metric
        let tracked = self.metrics.entry(metric.clone()).or_insert(TrackedMetric {
            metric: metric.clone(),
            value: 0.0,
            timestamp,
            sample_count: 0,
        });

        tracked.value = value;
        tracked.timestamp = timestamp;
        tracked.sample_count += 1;

        // Add to history
        let history = self.history.entry(metric).or_insert_with(Vec::new);
        history.push((timestamp, value));

        // Trim history
        if history.len() > self.window_size {
            history.remove(0);
        }
    }

    /// Check a target against current metrics.
    pub fn check_target(&mut self, target: &SLOTarget) -> bool {
        if let Some(tracked) = self.metrics.get(&target.metric) {
            let met = target.is_met(tracked.value);

            if !met {
                // Record breach
                self.breaches.push(BreachEvent {
                    contract_id: self.contract_id.clone(),
                    metric: target.metric.clone(),
                    target: target.target,
                    actual: tracked.value,
                    timestamp: now(),
                    duration_seconds: 0,
                });
            }

            met
        } else {
            true // No data yet, assume OK
        }
    }

    /// Check all targets in a contract.
    pub fn check_contract<'a>(&mut self, contract: &'a SLAContract) -> Vec<&'a SLOTarget> {
        let mut failed = Vec::new();

        for target in &contract.targets {
            if !self.check_target(target) {
                failed.push(target);
            }
        }

        failed
    }

    /// Get current value for a metric.
    pub fn get_value(&self, metric: &SLOMetric) -> Option<f64> {
        self.metrics.get(metric).map(|m| m.value)
    }

    /// Get average value over window.
    pub fn get_average(&self, metric: &SLOMetric) -> Option<f64> {
        self.history.get(metric).map(|h| {
            if h.is_empty() {
                0.0
            } else {
                h.iter().map(|(_, v)| v).sum::<f64>() / h.len() as f64
            }
        })
    }

    /// Calculate availability over the tracking period.
    pub fn calculate_availability(&self) -> f64 {
        if let Some(history) = self.history.get(&SLOMetric::Availability) {
            if history.is_empty() {
                return 100.0;
            }
            history.iter().map(|(_, v)| v).sum::<f64>() / history.len() as f64
        } else {
            100.0
        }
    }

    /// Get breach count.
    pub fn breach_count(&self) -> usize {
        self.breaches.len()
    }

    /// Get all breaches.
    pub fn breaches(&self) -> &[BreachEvent] {
        &self.breaches
    }

    /// Get breaches for a specific metric.
    pub fn breaches_for(&self, metric: &SLOMetric) -> Vec<&BreachEvent> {
        self.breaches.iter().filter(|b| &b.metric == metric).collect()
    }

    /// Clear breach history.
    pub fn clear_breaches(&mut self) {
        self.breaches.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sla::agreement::Percentile;

    #[test]
    fn test_tracker_creation() {
        let tracker = SLATracker::new("sla-1");
        assert_eq!(tracker.breach_count(), 0);
    }

    #[test]
    fn test_record_metric() {
        let mut tracker = SLATracker::new("sla-1");
        tracker.record(SLOMetric::Availability, 99.9);

        assert_eq!(tracker.get_value(&SLOMetric::Availability), Some(99.9));
    }

    #[test]
    fn test_check_target_passing() {
        let mut tracker = SLATracker::new("sla-1");
        tracker.record(SLOMetric::Availability, 99.95);

        let target = SLOTarget::new(SLOMetric::Availability, 99.9, "%");
        assert!(tracker.check_target(&target));
        assert_eq!(tracker.breach_count(), 0);
    }

    #[test]
    fn test_check_target_failing() {
        let mut tracker = SLATracker::new("sla-1");
        tracker.record(SLOMetric::Availability, 99.0);

        let target = SLOTarget::new(SLOMetric::Availability, 99.9, "%");
        assert!(!tracker.check_target(&target));
        assert_eq!(tracker.breach_count(), 1);
    }

    #[test]
    fn test_average() {
        let mut tracker = SLATracker::new("sla-1");
        tracker.record(SLOMetric::Availability, 100.0);
        tracker.record(SLOMetric::Availability, 99.0);
        tracker.record(SLOMetric::Availability, 98.0);

        let avg = tracker.get_average(&SLOMetric::Availability).unwrap();
        assert!((avg - 99.0).abs() < 0.01);
    }

    #[test]
    fn test_check_contract() {
        let mut tracker = SLATracker::new("sla-1");
        tracker.record(SLOMetric::Availability, 99.95);
        tracker.record(SLOMetric::Latency(Percentile::P99), 50.0);

        let contract = SLAContract::enterprise_standard("sla-1", "Test", "tenant-1");
        let failed = tracker.check_contract(&contract);

        assert!(failed.is_empty());
    }
}

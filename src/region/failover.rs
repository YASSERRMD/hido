//! Failover management for high availability.
//!
//! Handles automatic failover and rebalancing.

use crate::core::{now, Timestamp};
use crate::region::manager::{Region, RegionStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Failover strategy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailoverStrategy {
    /// Use nearest healthy region
    NearestHealthy,
    /// Use predefined priority list
    Priority(Vec<String>),
    /// Use region with most capacity
    HighestCapacity,
    /// Manual failover only
    Manual,
}

/// Failover event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailoverEvent {
    /// Event ID
    pub id: String,
    /// Failed region
    pub failed_region: String,
    /// Target region
    pub target_region: String,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Reason for failover
    pub reason: String,
    /// Was automatic
    pub automatic: bool,
    /// Duration of failover (ms)
    pub duration_ms: u64,
}

/// Failover manager.
pub struct FailoverManager {
    /// Failover strategy
    strategy: FailoverStrategy,
    /// Failover history
    history: Vec<FailoverEvent>,
    /// Health thresholds
    health_thresholds: HealthThresholds,
    /// Current failover mappings
    failover_map: HashMap<String, String>,
}

/// Health thresholds for triggering failover.
#[derive(Clone, Debug)]
pub struct HealthThresholds {
    /// Max latency before degraded (ms)
    pub max_latency_ms: u32,
    /// Max consecutive failures
    pub max_failures: u32,
    /// Timeout for health check (ms)
    pub health_check_timeout_ms: u32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_latency_ms: 1000,
            max_failures: 3,
            health_check_timeout_ms: 5000,
        }
    }
}

impl FailoverManager {
    /// Create a new failover manager.
    pub fn new(strategy: FailoverStrategy) -> Self {
        Self {
            strategy,
            history: Vec::new(),
            health_thresholds: HealthThresholds::default(),
            failover_map: HashMap::new(),
        }
    }

    /// Set health thresholds.
    pub fn with_thresholds(mut self, thresholds: HealthThresholds) -> Self {
        self.health_thresholds = thresholds;
        self
    }

    /// Determine failover target for a failed region.
    pub fn select_target(&self, failed_region: &str, available: &[&Region]) -> Option<String> {
        if available.is_empty() {
            return None;
        }

        match &self.strategy {
            FailoverStrategy::NearestHealthy => {
                // Select by lowest latency
                available
                    .iter()
                    .filter(|r| r.id != failed_region && r.status == RegionStatus::Healthy)
                    .min_by_key(|r| r.latency_ms)
                    .map(|r| r.id.clone())
            }
            FailoverStrategy::Priority(priority) => {
                // Select first available in priority order
                priority
                    .iter()
                    .find(|id| {
                        *id != failed_region
                            && available.iter().any(|r| &r.id == *id && r.is_available())
                    })
                    .cloned()
            }
            FailoverStrategy::HighestCapacity => {
                // Select by node count
                available
                    .iter()
                    .filter(|r| r.id != failed_region && r.is_available())
                    .max_by_key(|r| r.node_count)
                    .map(|r| r.id.clone())
            }
            FailoverStrategy::Manual => {
                // Check for manual mapping
                self.failover_map.get(failed_region).cloned()
            }
        }
    }

    /// Execute failover.
    pub fn execute_failover(
        &mut self,
        failed_region: &str,
        target_region: &str,
        reason: &str,
        automatic: bool,
    ) -> FailoverEvent {
        let event = FailoverEvent {
            id: uuid::Uuid::new_v4().to_string(),
            failed_region: failed_region.to_string(),
            target_region: target_region.to_string(),
            timestamp: now(),
            reason: reason.to_string(),
            automatic,
            duration_ms: 0, // Would be measured in real implementation
        };

        self.failover_map
            .insert(failed_region.to_string(), target_region.to_string());
        self.history.push(event.clone());

        event
    }

    /// Clear failover mapping (region recovered).
    pub fn clear_failover(&mut self, region_id: &str) {
        self.failover_map.remove(region_id);
    }

    /// Get current failover target for a region.
    pub fn get_failover_target(&self, region_id: &str) -> Option<&str> {
        self.failover_map.get(region_id).map(|s| s.as_str())
    }

    /// Check if region is in failover.
    pub fn is_failed_over(&self, region_id: &str) -> bool {
        self.failover_map.contains_key(region_id)
    }

    /// Get failover history.
    pub fn history(&self) -> &[FailoverEvent] {
        &self.history
    }

    /// Get recent failovers (last n).
    pub fn recent_failovers(&self, n: usize) -> Vec<&FailoverEvent> {
        self.history.iter().rev().take(n).collect()
    }

    /// Set manual failover mapping.
    pub fn set_manual_failover(&mut self, from: &str, to: &str) {
        self.failover_map.insert(from.to_string(), to.to_string());
    }
}

impl Default for FailoverManager {
    fn default() -> Self {
        Self::new(FailoverStrategy::NearestHealthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_regions() -> Vec<Region> {
        vec![
            Region::new("us-west", "US West", "Oregon"),
            Region::new("eu-central", "EU Central", "Frankfurt"),
            Region::new("ap-south", "AP South", "Mumbai"),
        ]
    }

    #[test]
    fn test_failover_manager_creation() {
        let manager = FailoverManager::new(FailoverStrategy::NearestHealthy);
        assert!(manager.history().is_empty());
    }

    #[test]
    fn test_select_target_nearest() {
        let mut manager = FailoverManager::new(FailoverStrategy::NearestHealthy);
        let mut regions = create_test_regions();
        regions[1].latency_ms = 50;  // eu-central is closest
        regions[2].latency_ms = 100;

        let refs: Vec<&Region> = regions.iter().collect();
        let target = manager.select_target("us-west", &refs);

        assert_eq!(target, Some("eu-central".to_string()));
    }

    #[test]
    fn test_select_target_priority() {
        let manager = FailoverManager::new(FailoverStrategy::Priority(vec![
            "ap-south".to_string(),
            "eu-central".to_string(),
        ]));
        let regions = create_test_regions();
        let refs: Vec<&Region> = regions.iter().collect();

        let target = manager.select_target("us-west", &refs);
        assert_eq!(target, Some("ap-south".to_string()));
    }

    #[test]
    fn test_execute_failover() {
        let mut manager = FailoverManager::default();
        let event = manager.execute_failover("us-west", "eu-central", "Health check failed", true);

        assert_eq!(event.failed_region, "us-west");
        assert_eq!(event.target_region, "eu-central");
        assert!(manager.is_failed_over("us-west"));
    }

    #[test]
    fn test_clear_failover() {
        let mut manager = FailoverManager::default();
        manager.execute_failover("us-west", "eu-central", "Test", true);

        assert!(manager.is_failed_over("us-west"));
        manager.clear_failover("us-west");
        assert!(!manager.is_failed_over("us-west"));
    }

    #[test]
    fn test_get_failover_target() {
        let mut manager = FailoverManager::default();
        manager.execute_failover("us-west", "eu-central", "Test", true);

        assert_eq!(manager.get_failover_target("us-west"), Some("eu-central"));
        assert_eq!(manager.get_failover_target("ap-south"), None);
    }
}

//! Region manager for multi-region deployments.
//!
//! Coordinates HIDO nodes across geographic regions.

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Region status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionStatus {
    /// Region is healthy
    Healthy,
    /// Region is degraded
    Degraded,
    /// Region is unreachable
    Unreachable,
    /// Region is in maintenance
    Maintenance,
}

/// A geographic region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    /// Region ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Geographic location
    pub location: String,
    /// Is primary region
    pub is_primary: bool,
    /// Current status
    pub status: RegionStatus,
    /// Number of nodes
    pub node_count: u32,
    /// Latency to this region (ms)
    pub latency_ms: u32,
    /// Last health check
    pub last_health_check: Timestamp,
}

impl Region {
    /// Create a new region.
    pub fn new(id: &str, name: &str, location: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            location: location.to_string(),
            is_primary: false,
            status: RegionStatus::Healthy,
            node_count: 0,
            latency_ms: 0,
            last_health_check: now(),
        }
    }

    /// Set as primary.
    pub fn as_primary(mut self) -> Self {
        self.is_primary = true;
        self
    }

    /// Set node count.
    pub fn with_nodes(mut self, count: u32) -> Self {
        self.node_count = count;
        self
    }

    /// Check if region is available.
    pub fn is_available(&self) -> bool {
        matches!(self.status, RegionStatus::Healthy | RegionStatus::Degraded)
    }

    /// Update health status.
    pub fn update_health(&mut self, status: RegionStatus, latency_ms: u32) {
        self.status = status;
        self.latency_ms = latency_ms;
        self.last_health_check = now();
    }
}

/// Manager for multiple regions.
pub struct RegionManager {
    /// All regions
    regions: HashMap<String, Region>,
    /// Primary region ID
    primary_id: Option<String>,
    /// Health check interval (seconds)
    health_check_interval: u64,
}

impl RegionManager {
    /// Create a new region manager.
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            primary_id: None,
            health_check_interval: 30,
        }
    }

    /// Add a region.
    pub fn add_region(&mut self, region: Region) {
        if region.is_primary {
            self.primary_id = Some(region.id.clone());
        }
        self.regions.insert(region.id.clone(), region);
    }

    /// Remove a region.
    pub fn remove_region(&mut self, region_id: &str) {
        self.regions.remove(region_id);
        if self.primary_id.as_deref() == Some(region_id) {
            self.primary_id = None;
        }
    }

    /// Get a region.
    pub fn get_region(&self, region_id: &str) -> Option<&Region> {
        self.regions.get(region_id)
    }

    /// Get the primary region.
    pub fn primary(&self) -> Option<&Region> {
        self.primary_id.as_ref().and_then(|id| self.regions.get(id))
    }

    /// Set primary region.
    pub fn set_primary(&mut self, region_id: &str) -> bool {
        if self.regions.contains_key(region_id) {
            // Clear old primary
            if let Some(old_id) = &self.primary_id {
                if let Some(old) = self.regions.get_mut(old_id) {
                    old.is_primary = false;
                }
            }
            // Set new primary
            if let Some(region) = self.regions.get_mut(region_id) {
                region.is_primary = true;
                self.primary_id = Some(region_id.to_string());
                return true;
            }
        }
        false
    }

    /// Get all healthy regions.
    pub fn healthy_regions(&self) -> Vec<&Region> {
        self.regions
            .values()
            .filter(|r| r.status == RegionStatus::Healthy)
            .collect()
    }

    /// Get available regions (healthy or degraded).
    pub fn available_regions(&self) -> Vec<&Region> {
        self.regions.values().filter(|r| r.is_available()).collect()
    }

    /// Get region with lowest latency.
    pub fn lowest_latency_region(&self) -> Option<&Region> {
        self.available_regions()
            .into_iter()
            .min_by_key(|r| r.latency_ms)
    }

    /// Update region status.
    pub fn update_status(&mut self, region_id: &str, status: RegionStatus, latency_ms: u32) {
        if let Some(region) = self.regions.get_mut(region_id) {
            region.update_health(status, latency_ms);
        }
    }

    /// Get region count.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get total node count.
    pub fn total_nodes(&self) -> u32 {
        self.regions.values().map(|r| r.node_count).sum()
    }

    /// List all regions.
    pub fn list_regions(&self) -> Vec<&Region> {
        self.regions.values().collect()
    }
}

impl Default for RegionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let region = Region::new("us-west-1", "US West", "Oregon, USA");
        assert_eq!(region.id, "us-west-1");
        assert!(!region.is_primary);
        assert!(region.is_available());
    }

    #[test]
    fn test_region_as_primary() {
        let region = Region::new("eu-central-1", "EU Central", "Frankfurt, Germany").as_primary();
        assert!(region.is_primary);
    }

    #[test]
    fn test_region_manager() {
        let mut manager = RegionManager::new();
        manager.add_region(Region::new("us-west", "US West", "Oregon").as_primary().with_nodes(5));
        manager.add_region(Region::new("eu-central", "EU Central", "Frankfurt").with_nodes(3));

        assert_eq!(manager.region_count(), 2);
        assert_eq!(manager.total_nodes(), 8);
    }

    #[test]
    fn test_primary_region() {
        let mut manager = RegionManager::new();
        manager.add_region(Region::new("r1", "Region 1", "Loc 1").as_primary());
        manager.add_region(Region::new("r2", "Region 2", "Loc 2"));

        assert_eq!(manager.primary().unwrap().id, "r1");

        manager.set_primary("r2");
        assert_eq!(manager.primary().unwrap().id, "r2");
    }

    #[test]
    fn test_healthy_regions() {
        let mut manager = RegionManager::new();
        manager.add_region(Region::new("r1", "R1", "L1"));
        manager.add_region(Region::new("r2", "R2", "L2"));

        manager.update_status("r1", RegionStatus::Unreachable, 0);

        assert_eq!(manager.healthy_regions().len(), 1);
        assert_eq!(manager.available_regions().len(), 1);
    }

    #[test]
    fn test_lowest_latency() {
        let mut manager = RegionManager::new();
        manager.add_region(Region::new("r1", "R1", "L1"));
        manager.add_region(Region::new("r2", "R2", "L2"));

        manager.update_status("r1", RegionStatus::Healthy, 100);
        manager.update_status("r2", RegionStatus::Healthy, 50);

        assert_eq!(manager.lowest_latency_region().unwrap().id, "r2");
    }
}

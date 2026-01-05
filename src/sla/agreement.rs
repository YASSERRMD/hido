//! SLA agreements and contracts.
//!
//! Defines SLAs with SLO targets.

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An SLO (Service Level Objective) metric type.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SLOMetric {
    /// Uptime percentage
    Availability,
    /// Latency percentile
    Latency(Percentile),
    /// Error rate
    ErrorRate,
    /// Throughput
    Throughput,
    /// Custom metric
    Custom(String),
}

/// Percentile for latency SLOs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Percentile {
    P50,
    P90,
    P95,
    P99,
    P999,
}

/// Target for an SLO.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLOTarget {
    /// Metric
    pub metric: SLOMetric,
    /// Target value
    pub target: f64,
    /// Unit (ms, %, req/s)
    pub unit: String,
    /// Period (monthly, weekly)
    pub period: SLAPeriod,
}

impl SLOTarget {
    /// Create a new SLO target.
    pub fn new(metric: SLOMetric, target: f64, unit: &str) -> Self {
        Self {
            metric,
            target,
            unit: unit.to_string(),
            period: SLAPeriod::Monthly,
        }
    }

    /// Set period.
    pub fn with_period(mut self, period: SLAPeriod) -> Self {
        self.period = period;
        self
    }

    /// Check if a value meets this target.
    pub fn is_met(&self, value: f64) -> bool {
        match self.metric {
            SLOMetric::Availability | SLOMetric::Throughput => value >= self.target,
            SLOMetric::Latency(_) | SLOMetric::ErrorRate => value <= self.target,
            SLOMetric::Custom(_) => value >= self.target,
        }
    }
}

/// SLA measurement period.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SLAPeriod {
    /// Hourly
    Hourly,
    /// Daily
    Daily,
    /// Weekly
    Weekly,
    /// Monthly
    Monthly,
    /// Quarterly
    Quarterly,
}

/// SLA contract status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractStatus {
    /// Active
    Active,
    /// Suspended
    Suspended,
    /// Expired
    Expired,
    /// Breached
    Breached,
}

/// An SLA contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLAContract {
    /// Contract ID
    pub id: String,
    /// Contract name
    pub name: String,
    /// Customer/tenant ID
    pub tenant_id: String,
    /// SLO targets
    pub targets: Vec<SLOTarget>,
    /// Status
    pub status: ContractStatus,
    /// Start date
    pub start_date: Timestamp,
    /// End date (optional)
    pub end_date: Option<Timestamp>,
    /// Credits for breaches
    pub credit_percentage: f32,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl SLAContract {
    /// Create a new SLA contract.
    pub fn new(id: &str, name: &str, tenant_id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            tenant_id: tenant_id.to_string(),
            targets: Vec::new(),
            status: ContractStatus::Active,
            start_date: now(),
            end_date: None,
            credit_percentage: 10.0,
            metadata: HashMap::new(),
        }
    }

    /// Add an SLO target.
    pub fn add_target(&mut self, target: SLOTarget) {
        self.targets.push(target);
    }

    /// Create with common enterprise SLOs.
    pub fn enterprise_standard(id: &str, name: &str, tenant_id: &str) -> Self {
        let mut contract = Self::new(id, name, tenant_id);
        
        // 99.9% availability
        contract.add_target(SLOTarget::new(SLOMetric::Availability, 99.9, "%"));
        
        // P99 latency < 1s
        contract.add_target(SLOTarget::new(
            SLOMetric::Latency(Percentile::P99),
            1000.0,
            "ms",
        ));
        
        // Error rate < 0.1%
        contract.add_target(SLOTarget::new(SLOMetric::ErrorRate, 0.1, "%"));
        
        contract
    }

    /// Check if contract is active.
    pub fn is_active(&self) -> bool {
        self.status == ContractStatus::Active
    }

    /// Get target for a metric.
    pub fn get_target(&self, metric: &SLOMetric) -> Option<&SLOTarget> {
        self.targets.iter().find(|t| &t.metric == metric)
    }

    /// Suspend the contract.
    pub fn suspend(&mut self) {
        self.status = ContractStatus::Suspended;
    }

    /// Mark as breached.
    pub fn mark_breached(&mut self) {
        self.status = ContractStatus::Breached;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slo_target() {
        let target = SLOTarget::new(SLOMetric::Availability, 99.9, "%");
        assert!(target.is_met(99.95));
        assert!(!target.is_met(99.5));
    }

    #[test]
    fn test_latency_target() {
        let target = SLOTarget::new(SLOMetric::Latency(Percentile::P99), 100.0, "ms");
        assert!(target.is_met(80.0));
        assert!(!target.is_met(150.0));
    }

    #[test]
    fn test_contract_creation() {
        let contract = SLAContract::new("sla-1", "Enterprise SLA", "tenant-1");
        assert!(contract.is_active());
        assert!(contract.targets.is_empty());
    }

    #[test]
    fn test_enterprise_standard() {
        let contract = SLAContract::enterprise_standard("sla-1", "Standard", "tenant-1");
        assert_eq!(contract.targets.len(), 3);
        assert!(contract.get_target(&SLOMetric::Availability).is_some());
    }

    #[test]
    fn test_contract_status() {
        let mut contract = SLAContract::new("sla-1", "Test", "tenant-1");
        assert!(contract.is_active());

        contract.suspend();
        assert!(!contract.is_active());
        assert_eq!(contract.status, ContractStatus::Suspended);
    }
}

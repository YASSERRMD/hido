//! SLA reporting.
//!
//! Generates SLA reports and summaries.

use crate::core::{now, Timestamp};
use crate::sla::agreement::{SLAContract, SLOMetric};
use crate::sla::tracker::{BreachEvent, SLATracker};
use serde::{Deserialize, Serialize};

/// An SLA report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLAReport {
    /// Contract ID
    pub contract_id: String,
    /// Contract name
    pub contract_name: String,
    /// Report period start
    pub period_start: Timestamp,
    /// Report period end
    pub period_end: Timestamp,
    /// Generated timestamp
    pub generated: Timestamp,
    /// Overall compliance
    pub compliant: bool,
    /// SLO summaries
    pub slo_summaries: Vec<SLOSummary>,
    /// Breach count
    pub breach_count: usize,
    /// Credit percentage owed
    pub credit_owed: f32,
}

/// Summary for a single SLO.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLOSummary {
    /// Metric
    pub metric: SLOMetric,
    /// Target value
    pub target: f64,
    /// Actual value
    pub actual: f64,
    /// Unit
    pub unit: String,
    /// Met target
    pub met: bool,
    /// Breach count for this SLO
    pub breach_count: usize,
}

/// SLA reporter.
pub struct SLAReporter {
    /// Reports generated
    reports: Vec<SLAReport>,
}

impl SLAReporter {
    /// Create a new reporter.
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
        }
    }

    /// Generate a report for a contract.
    pub fn generate_report(
        &mut self,
        contract: &SLAContract,
        tracker: &SLATracker,
        period_start: Timestamp,
        period_end: Timestamp,
    ) -> SLAReport {
        let mut slo_summaries = Vec::new();
        let mut all_met = true;
        let mut total_breaches = 0;

        for target in &contract.targets {
            let actual = tracker.get_average(&target.metric).unwrap_or(0.0);
            let met = target.is_met(actual);
            let breach_count = tracker.breaches_for(&target.metric).len();

            if !met {
                all_met = false;
            }
            total_breaches += breach_count;

            slo_summaries.push(SLOSummary {
                metric: target.metric.clone(),
                target: target.target,
                actual,
                unit: target.unit.clone(),
                met,
                breach_count,
            });
        }

        // Calculate credit owed
        let credit_owed = if !all_met {
            contract.credit_percentage
        } else {
            0.0
        };

        let report = SLAReport {
            contract_id: contract.id.clone(),
            contract_name: contract.name.clone(),
            period_start,
            period_end,
            generated: now(),
            compliant: all_met,
            slo_summaries,
            breach_count: total_breaches,
            credit_owed,
        };

        self.reports.push(report.clone());
        report
    }

    /// Get all generated reports.
    pub fn reports(&self) -> &[SLAReport] {
        &self.reports
    }

    /// Get reports for a contract.
    pub fn reports_for_contract(&self, contract_id: &str) -> Vec<&SLAReport> {
        self.reports
            .iter()
            .filter(|r| r.contract_id == contract_id)
            .collect()
    }

    /// Generate report as JSON.
    pub fn to_json(report: &SLAReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }

    /// Generate report as text.
    pub fn to_text(report: &SLAReport) -> String {
        let mut output = String::new();

        output.push_str(&format!("SLA Report: {}\n", report.contract_name));
        output.push_str(&format!("Contract ID: {}\n", report.contract_id));
        output.push_str(&format!(
            "Period: {} - {}\n",
            report.period_start, report.period_end
        ));
        output.push_str(&format!(
            "Status: {}\n",
            if report.compliant { "COMPLIANT" } else { "NON-COMPLIANT" }
        ));
        output.push_str("\nSLO Summary:\n");

        for slo in &report.slo_summaries {
            let status = if slo.met { "✓" } else { "✗" };
            output.push_str(&format!(
                "  {} {:?}: {:.2}{} (target: {:.2}{})\n",
                status, slo.metric, slo.actual, slo.unit, slo.target, slo.unit
            ));
        }

        if report.breach_count > 0 {
            output.push_str(&format!("\nBreaches: {}\n", report.breach_count));
        }

        if report.credit_owed > 0.0 {
            output.push_str(&format!("Credit Owed: {:.1}%\n", report.credit_owed));
        }

        output
    }
}

impl Default for SLAReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reporter_creation() {
        let reporter = SLAReporter::new();
        assert!(reporter.reports().is_empty());
    }

    #[test]
    fn test_generate_report() {
        let mut reporter = SLAReporter::new();
        let mut tracker = SLATracker::new("sla-1");

        // Record good metrics
        tracker.record(SLOMetric::Availability, 99.95);

        let contract = SLAContract::enterprise_standard("sla-1", "Test SLA", "tenant-1");
        let report = reporter.generate_report(&contract, &tracker, now(), now());

        assert_eq!(report.contract_id, "sla-1");
        assert!(!report.slo_summaries.is_empty());
    }

    #[test]
    fn test_report_compliance() {
        let mut reporter = SLAReporter::new();
        let mut tracker = SLATracker::new("sla-1");

        // Record metrics that meet all targets
        tracker.record(SLOMetric::Availability, 99.95);
        use crate::sla::agreement::Percentile;
        tracker.record(SLOMetric::Latency(Percentile::P99), 50.0);
        tracker.record(SLOMetric::ErrorRate, 0.05);

        let contract = SLAContract::enterprise_standard("sla-1", "Test", "tenant-1");
        let report = reporter.generate_report(&contract, &tracker, now(), now());

        assert!(report.compliant);
        assert_eq!(report.credit_owed, 0.0);
    }

    #[test]
    fn test_report_non_compliance() {
        let mut reporter = SLAReporter::new();
        let mut tracker = SLATracker::new("sla-1");

        // Record metrics that don't meet availability target
        tracker.record(SLOMetric::Availability, 99.0); // Below 99.9% target

        let contract = SLAContract::enterprise_standard("sla-1", "Test", "tenant-1");
        let report = reporter.generate_report(&contract, &tracker, now(), now());

        assert!(!report.compliant);
        assert!(report.credit_owed > 0.0);
    }

    #[test]
    fn test_report_formats() {
        let mut reporter = SLAReporter::new();
        let tracker = SLATracker::new("sla-1");
        let contract = SLAContract::new("sla-1", "Test", "tenant-1");
        let report = reporter.generate_report(&contract, &tracker, now(), now());

        let json = SLAReporter::to_json(&report);
        assert!(json.contains("sla-1"));

        let text = SLAReporter::to_text(&report);
        assert!(text.contains("SLA Report"));
    }
}

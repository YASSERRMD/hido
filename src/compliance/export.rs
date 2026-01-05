//! Regulatory exports for compliance.
//!
//! Exports data for regulatory requirements.

use crate::compliance::audit::{AuditEntry, AuditFilter, AuditLogger};
use crate::core::{now, Result, Timestamp};
use serde::{Deserialize, Serialize};

/// Export format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// XML format
    Xml,
}

/// Regulatory exporter.
pub struct RegulatoryExporter {
    /// Audit logger reference
    logger: AuditLogger,
}

impl RegulatoryExporter {
    /// Create a new exporter.
    pub fn new(logger: AuditLogger) -> Self {
        Self { logger }
    }

    /// Export for GDPR data subject request.
    pub fn export_gdpr(&self, subject: &str) -> Result<Vec<u8>> {
        let filter = AuditFilter::new().by_actor(subject);
        let entries = self.logger.query(&filter);

        let export = GDPRExport {
            subject: subject.to_string(),
            entries: entries.clone(),
            export_date: crate::core::now(),
            purpose: "Data Subject Access Request (DSAR)".to_string(),
        };

        Ok(serde_json::to_vec_pretty(&export)?)
    }

    /// Export audit trail.
    pub fn export_audit_trail(
        &self,
        from: Option<Timestamp>,
        to: Option<Timestamp>,
        format: ExportFormat,
    ) -> Result<Vec<u8>> {
        let filter = AuditFilter {
            date_from: from,
            date_to: to,
            ..Default::default()
        };
        let entries = self.logger.query(&filter);

        match format {
            ExportFormat::Json => Ok(serde_json::to_vec_pretty(&entries)?),
            ExportFormat::Csv => self.to_csv(&entries),
            ExportFormat::Xml => self.to_xml(&entries),
        }
    }

    /// Export decision log.
    pub fn export_decision_log(&self) -> Result<Vec<u8>> {
        let filter = AuditFilter::new().by_action(crate::compliance::audit::AuditAction::ConsensusReached);
        let entries = self.logger.query(&filter);

        let log = DecisionLog {
            entries,
            export_date: crate::core::now(),
        };

        Ok(serde_json::to_vec_pretty(&log)?)
    }

    /// Export incident report.
    pub fn export_incident_report(&self, incident_id: &str) -> Result<Vec<u8>> {
        let entries: Vec<_> = self.logger.all().into_iter()
            .filter(|e| {
                e.details.get("incident_id")
                    .and_then(|v| v.as_str())
                    .map(|id| id == incident_id)
                    .unwrap_or(false)
            })
            .collect();

        let report = IncidentReport {
            incident_id: incident_id.to_string(),
            entries,
            export_date: crate::core::now(),
        };

        Ok(serde_json::to_vec_pretty(&report)?)
    }

    fn to_csv(&self, entries: &[AuditEntry]) -> Result<Vec<u8>> {
        let mut output = String::new();
        output.push_str("id,timestamp,actor,action,resource,success\n");

        for entry in entries {
            output.push_str(&format!(
                "{},{},{},{:?},{},{}\n",
                entry.id,
                entry.timestamp,
                entry.actor,
                entry.action,
                entry.resource,
                entry.success
            ));
        }

        Ok(output.into_bytes())
    }

    fn to_xml(&self, entries: &[AuditEntry]) -> Result<Vec<u8>> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<AuditEntries>\n");

        for entry in entries {
            output.push_str("  <Entry>\n");
            output.push_str(&format!("    <Id>{}</Id>\n", entry.id));
            output.push_str(&format!("    <Timestamp>{}</Timestamp>\n", entry.timestamp));
            output.push_str(&format!("    <Actor>{}</Actor>\n", entry.actor));
            output.push_str(&format!("    <Action>{:?}</Action>\n", entry.action));
            output.push_str(&format!("    <Resource>{}</Resource>\n", entry.resource));
            output.push_str(&format!("    <Success>{}</Success>\n", entry.success));
            output.push_str("  </Entry>\n");
        }

        output.push_str("</AuditEntries>\n");
        Ok(output.into_bytes())
    }
}

/// GDPR export structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GDPRExport {
    subject: String,
    entries: Vec<AuditEntry>,
    export_date: Timestamp,
    purpose: String,
}

/// Decision log export.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DecisionLog {
    entries: Vec<AuditEntry>,
    export_date: Timestamp,
}

/// Incident report export.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct IncidentReport {
    incident_id: String,
    entries: Vec<AuditEntry>,
    export_date: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compliance::audit::AuditAction;

    fn setup_logger() -> AuditLogger {
        let logger = AuditLogger::default();
        logger.log(AuditEntry::new("user-1", AuditAction::DataAccessed, "res-1"));
        logger.log(AuditEntry::new("user-2", AuditAction::DataModified, "res-2"));
        logger.log(AuditEntry::new("user-1", AuditAction::DataAccessed, "res-3"));
        logger
    }

    #[test]
    fn test_export_gdpr() {
        let logger = setup_logger();
        let exporter = RegulatoryExporter::new(logger);

        let data = exporter.export_gdpr("user-1").unwrap();
        let json = String::from_utf8(data).unwrap();

        assert!(json.contains("user-1"));
        assert!(json.contains("DSAR"));
    }

    #[test]
    fn test_export_audit_trail_json() {
        let logger = setup_logger();
        let exporter = RegulatoryExporter::new(logger);

        let data = exporter.export_audit_trail(None, None, ExportFormat::Json).unwrap();
        let json = String::from_utf8(data).unwrap();

        assert!(json.contains("DataAccessed"));
    }

    #[test]
    fn test_export_audit_trail_csv() {
        let logger = setup_logger();
        let exporter = RegulatoryExporter::new(logger);

        let data = exporter.export_audit_trail(None, None, ExportFormat::Csv).unwrap();
        let csv = String::from_utf8(data).unwrap();

        assert!(csv.contains("id,timestamp,actor"));
        assert!(csv.contains("user-1"));
    }

    #[test]
    fn test_export_audit_trail_xml() {
        let logger = setup_logger();
        let exporter = RegulatoryExporter::new(logger);

        let data = exporter.export_audit_trail(None, None, ExportFormat::Xml).unwrap();
        let xml = String::from_utf8(data).unwrap();

        assert!(xml.contains("<?xml"));
        assert!(xml.contains("<AuditEntries>"));
    }
}

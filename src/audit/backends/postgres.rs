//! PostgreSQL backend implementation.
//!
//! Fast reads, SQL queries, JSONB storage.

use crate::audit::backend::{AuditBackend, BackendType, VerificationResult};
use crate::audit::config::PostgresConfig;
use crate::audit::entry::{AuditEntry, EntryId};
use crate::audit::filter::AuditFilter;
use crate::core::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// PostgreSQL backend for fast reads and SQL queries.
///
/// Uses JSONB for flexible schema.
/// In production, would use sqlx or tokio-postgres.
pub struct PostgresBackend {
    /// Configuration
    config: PostgresConfig,
    /// In-memory storage (mock for now)
    entries: RwLock<HashMap<String, AuditEntry>>,
    /// Connected flag
    connected: bool,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend.
    pub async fn new(config: PostgresConfig) -> Result<Self> {
        // In production: establish database connection pool
        // let pool = PgPoolOptions::new()
        //     .max_connections(config.max_connections)
        //     .connect(&config.url)
        //     .await?;
        
        Ok(Self {
            config,
            entries: RwLock::new(HashMap::new()),
            connected: true,
        })
    }

    /// Get connection URL.
    pub fn url(&self) -> &str {
        &self.config.url
    }

    /// Get table name.
    pub fn table(&self) -> &str {
        &self.config.table
    }
}

#[async_trait]
impl AuditBackend for PostgresBackend {
    async fn record(&self, entry: AuditEntry) -> Result<EntryId> {
        let id = entry.id.clone();
        
        // In production:
        // sqlx::query!(
        //     "INSERT INTO audit_entries (id, actor, action, target, data) VALUES ($1, $2, $3, $4, $5)",
        //     id.as_str(), entry.actor, entry.action, entry.target, serde_json::to_value(&entry)?
        // ).execute(&self.pool).await?;

        let mut entries = self.entries.write().unwrap();
        entries.insert(id.as_str().to_string(), entry);

        Ok(id)
    }

    async fn read(&self, id: &EntryId) -> Result<Option<AuditEntry>> {
        // In production:
        // sqlx::query_as!(
        //     AuditEntry,
        //     "SELECT * FROM audit_entries WHERE id = $1",
        //     id.as_str()
        // ).fetch_optional(&self.pool).await?

        let entries = self.entries.read().unwrap();
        Ok(entries.get(id.as_str()).cloned())
    }

    async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        // In production: build SQL WHERE clause from filter
        // let mut query = "SELECT * FROM audit_entries WHERE 1=1".to_string();
        // if let Some(actor) = &filter.actor {
        //     query.push_str(&format!(" AND actor = '{}'", actor));
        // }
        // ...

        let entries = self.entries.read().unwrap();
        let mut results: Vec<AuditEntry> = entries
            .values()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect();

        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn verify(&self, id: &EntryId) -> Result<VerificationResult> {
        if let Some(entry) = self.read(id).await? {
            let hash_valid = entry.verify_hash();
            Ok(VerificationResult {
                entry_id: id.clone(),
                is_valid: hash_valid,
                hash_valid,
                signature_valid: None,
                message: if hash_valid {
                    "Entry hash verified".to_string()
                } else {
                    "Entry hash mismatch or not computed".to_string()
                },
            })
        } else {
            Ok(VerificationResult::invalid(id.clone(), "Entry not found"))
        }
    }

    fn backend_type(&self) -> BackendType {
        BackendType::PostgreSQL
    }

    async fn health_check(&self) -> Result<bool> {
        // In production: SELECT 1 FROM audit_entries LIMIT 1
        Ok(self.connected)
    }

    async fn count(&self) -> Result<u64> {
        // In production: SELECT COUNT(*) FROM audit_entries
        let entries = self.entries.read().unwrap();
        Ok(entries.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_postgres_backend_creation() {
        let backend = PostgresBackend::new(PostgresConfig::default()).await.unwrap();
        assert_eq!(backend.backend_type(), BackendType::PostgreSQL);
    }

    #[tokio::test]
    async fn test_record_and_read() {
        let backend = PostgresBackend::new(PostgresConfig::default()).await.unwrap();
        
        let entry = AuditEntry::new("agent-1", "execute", "task-1");
        let id = entry.id.clone();
        backend.record(entry).await.unwrap();

        let read = backend.read(&id).await.unwrap();
        assert!(read.is_some());
        assert_eq!(read.unwrap().actor, "agent-1");
    }

    #[tokio::test]
    async fn test_query() {
        let backend = PostgresBackend::new(PostgresConfig::default()).await.unwrap();
        
        backend.record(AuditEntry::new("agent-1", "execute", "task-1")).await.unwrap();
        backend.record(AuditEntry::new("agent-2", "execute", "task-2")).await.unwrap();

        let filter = AuditFilter::new().by_actor("agent-1");
        let results = backend.query(&filter).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor, "agent-1");
    }

    #[tokio::test]
    async fn test_count() {
        let backend = PostgresBackend::new(PostgresConfig::default()).await.unwrap();
        
        assert_eq!(backend.count().await.unwrap(), 0);
        
        backend.record(AuditEntry::new("agent-1", "test", "target")).await.unwrap();
        assert_eq!(backend.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_health_check() {
        let backend = PostgresBackend::new(PostgresConfig::default()).await.unwrap();
        assert!(backend.health_check().await.unwrap());
    }
}

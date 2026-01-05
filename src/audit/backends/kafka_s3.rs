//! Kafka + S3 backend implementation.
//!
//! Streaming to Kafka, archival to S3.

use crate::audit::backend::{AuditBackend, BackendType, VerificationResult};
use crate::audit::config::KafkaS3Config;
use crate::audit::entry::{AuditEntry, EntryId};
use crate::audit::filter::AuditFilter;
use crate::core::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// Backend using Kafka for streaming and S3 for storage.
///
/// In production:
/// - Writes go to Kafka topic
/// - Reads come from S3 (indexed) or recent Kafka stream
pub struct KafkaS3Backend {
    /// Configuration
    config: KafkaS3Config,
    /// Mock storage for implementation
    kafka_topic: RwLock<Vec<AuditEntry>>,
    s3_bucket: RwLock<HashMap<String, AuditEntry>>,
}

impl KafkaS3Backend {
    /// Create a new Kafka+S3 backend.
    pub async fn new(config: KafkaS3Config) -> Result<Self> {
        // In production: Initialize Kafka producer and S3 client
        Ok(Self {
            config,
            kafka_topic: RwLock::new(Vec::new()),
            s3_bucket: RwLock::new(HashMap::new()),
        })
    }

    /// Simulate archival process (moves from Kafka to S3).
    pub async fn archive(&self) -> Result<usize> {
        let mut topic = self.kafka_topic.write().unwrap();
        let mut bucket = self.s3_bucket.write().unwrap();
        
        let count = topic.len();
        for entry in topic.drain(..) {
            bucket.insert(entry.id.to_string(), entry);
        }
        
        Ok(count)
    }
}

#[async_trait]
impl AuditBackend for KafkaS3Backend {
    async fn record(&self, entry: AuditEntry) -> Result<EntryId> {
        let id = entry.id.clone();
        
        // Write to Kafka
        let mut topic = self.kafka_topic.write().unwrap();
        topic.push(entry);

        Ok(id)
    }

    async fn read(&self, id: &EntryId) -> Result<Option<AuditEntry>> {
        // Check S3 first
        {
            let bucket = self.s3_bucket.read().unwrap();
            if let Some(entry) = bucket.get(id.as_str()) {
                return Ok(Some(entry.clone()));
            }
        }

        // Check recent Kafka messages
        {
            let topic = self.kafka_topic.read().unwrap();
            if let Some(entry) = topic.iter().find(|e| &e.id == id) {
                return Ok(Some(entry.clone()));
            }
        }

        Ok(None)
    }

    async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        let mut results = Vec::new();

        // Search S3
        {
            let bucket = self.s3_bucket.read().unwrap();
            results.extend(bucket.values().filter(|e| filter.matches(e)).cloned());
        }

        // Search Kafka
        {
            let topic = self.kafka_topic.read().unwrap();
            results.extend(topic.iter().filter(|e| filter.matches(e)).cloned());
        }

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
                    "Entry hash validation failed".to_string()
                },
            })
        } else {
            Ok(VerificationResult::invalid(id.clone(), "Entry not found"))
        }
    }

    fn backend_type(&self) -> BackendType {
        BackendType::KafkaS3
    }

    async fn count(&self) -> Result<u64> {
        let topic_count = self.kafka_topic.read().unwrap().len();
        let s3_count = self.s3_bucket.read().unwrap().len();
        Ok((topic_count + s3_count) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kafka_s3_creation() {
        let backend = KafkaS3Backend::new(KafkaS3Config::default()).await.unwrap();
        assert_eq!(backend.backend_type(), BackendType::KafkaS3);
    }

    #[tokio::test]
    async fn test_record_and_archive() {
        let backend = KafkaS3Backend::new(KafkaS3Config::default()).await.unwrap();
        
        let entry = AuditEntry::new("agent-1", "test", "target");
        let id = entry.id.clone();
        
        // Write to Kafka
        backend.record(entry).await.unwrap();
        
        // Should be found (in Kafka)
        assert!(backend.read(&id).await.unwrap().is_some());

        // Archive to S3
        let archived = backend.archive().await.unwrap();
        assert_eq!(archived, 1);

        // Should still be found (in S3)
        assert!(backend.read(&id).await.unwrap().is_some());
    }
}

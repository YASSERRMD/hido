//! Parameter server for distributed learning.
//!
//! Manages global parameters with async push/pull.

use crate::core::{Result, Timestamp};
use crate::paramserver::region::RegionalServer;
use crate::uail::DIDKey;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// A parameter update from an agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterUpdate {
    /// Agent that submitted the update
    pub agent_id: String,
    /// The update (gradients or parameters)
    pub update: Vec<f32>,
    /// Submission timestamp
    pub timestamp: Timestamp,
    /// Version this update was computed against
    pub version_applied_to: u32,
}

/// Parameter server for coordinating distributed learning.
pub struct ParameterServer {
    /// Global parameters
    pub global_parameters: Vec<f32>,
    /// Current version
    pub version: u32,
    /// Regional servers
    pub regions: HashMap<String, RegionalServer>,
    /// Pending updates
    pending_updates: VecDeque<ParameterUpdate>,
    /// Server configuration
    config: ServerConfig,
    /// Statistics
    stats: ServerStats,
}

/// Server configuration.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Maximum pending updates
    pub max_pending: usize,
    /// Batch size for update application
    pub batch_size: usize,
    /// Learning rate for applying updates
    pub learning_rate: f32,
    /// Whether to use momentum
    pub use_momentum: bool,
    /// Momentum factor
    pub momentum: f32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_pending: 1000,
            batch_size: 10,
            learning_rate: 0.01,
            use_momentum: false,
            momentum: 0.9,
        }
    }
}

/// Server statistics.
#[derive(Clone, Debug, Default)]
pub struct ServerStats {
    pub total_updates_received: u64,
    pub total_updates_applied: u64,
    pub total_push_requests: u64,
    pub total_pull_requests: u64,
    pub current_version: u32,
}

impl ParameterServer {
    /// Create a new parameter server.
    pub fn new(initial_params: Vec<f32>) -> Self {
        Self {
            global_parameters: initial_params,
            version: 1,
            regions: HashMap::new(),
            pending_updates: VecDeque::new(),
            config: ServerConfig::default(),
            stats: ServerStats::default(),
        }
    }

    /// Create with configuration.
    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Push current parameters to an agent.
    /// REQUIREMENT: Non-blocking operation.
    pub async fn push_parameters(&self, _agent: &DIDKey) -> Result<Vec<f32>> {
        // In a real implementation, this would be truly async
        // For now, we return a clone of current parameters
        Ok(self.global_parameters.clone())
    }

    /// Pull update from an agent.
    /// REQUIREMENT: Non-blocking operation.
    pub async fn pull_update(&mut self, update: ParameterUpdate) -> Result<()> {
        self.stats.total_updates_received += 1;

        if self.pending_updates.len() >= self.config.max_pending {
            // Drop oldest update if at capacity
            self.pending_updates.pop_front();
        }

        self.pending_updates.push_back(update);
        Ok(())
    }

    /// Apply pending updates in batch.
    /// REQUIREMENT: Non-blocking, async application.
    pub async fn apply_updates(&mut self) -> Result<u32> {
        let batch_size = self.config.batch_size.min(self.pending_updates.len());

        if batch_size == 0 {
            return Ok(0);
        }

        // Collect batch of updates
        let mut batch: Vec<ParameterUpdate> = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            if let Some(update) = self.pending_updates.pop_front() {
                batch.push(update);
            }
        }

        // Average the updates
        let param_count = self.global_parameters.len();
        let mut avg_update = vec![0.0; param_count];

        for update in &batch {
            if update.update.len() == param_count {
                for (i, u) in update.update.iter().enumerate() {
                    avg_update[i] += u;
                }
            }
        }

        let batch_count = batch.len() as f32;
        for u in &mut avg_update {
            *u /= batch_count;
        }

        // Apply update with learning rate
        for (param, upd) in self.global_parameters.iter_mut().zip(avg_update.iter()) {
            *param -= self.config.learning_rate * upd;
        }

        self.version += 1;
        self.stats.total_updates_applied += batch.len() as u64;
        self.stats.current_version = self.version;

        Ok(batch.len() as u32)
    }

    /// Add a regional server.
    pub fn add_region(&mut self, region_id: &str) {
        let regional = RegionalServer::new(region_id, self.global_parameters.clone());
        self.regions.insert(region_id.to_string(), regional);
    }

    /// Synchronize all regional servers.
    pub async fn sync_regions(&mut self) -> Result<()> {
        let global = self.global_parameters.clone();
        let version = self.version;

        for region in self.regions.values_mut() {
            region.sync_from_global(global.clone(), version);
        }

        Ok(())
    }

    /// Get parameters from a specific region.
    pub fn get_regional_params(&self, region_id: &str) -> Option<&Vec<f32>> {
        self.regions.get(region_id).map(|r| &r.parameters)
    }

    /// Get pending update count.
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Get current version.
    pub fn current_version(&self) -> u32 {
        self.version
    }

    /// Get statistics.
    pub fn stats(&self) -> &ServerStats {
        &self.stats
    }

    /// Get parameter count.
    pub fn param_count(&self) -> usize {
        self.global_parameters.len()
    }
}

impl Default for ParameterServer {
    fn default() -> Self {
        Self::new(vec![0.0; 100])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uail::crypto::CryptoSuite;

    fn create_test_did() -> DIDKey {
        let crypto = CryptoSuite::new();
        DIDKey::new(&crypto)
    }

    #[tokio::test]
    async fn test_server_creation() {
        let server = ParameterServer::new(vec![1.0; 10]);
        assert_eq!(server.param_count(), 10);
        assert_eq!(server.current_version(), 1);
    }

    #[tokio::test]
    async fn test_push_parameters() {
        let server = ParameterServer::new(vec![0.5; 5]);
        let agent = create_test_did();

        let params = server.push_parameters(&agent).await.unwrap();
        assert_eq!(params.len(), 5);
        assert!((params[0] - 0.5).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_pull_update() {
        let mut server = ParameterServer::new(vec![0.0; 5]);
        let agent = create_test_did();

        let update = ParameterUpdate {
            agent_id: agent.id.clone(),
            update: vec![0.1; 5],
            timestamp: now(),
            version_applied_to: 1,
        };

        server.pull_update(update).await.unwrap();
        assert_eq!(server.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_apply_updates() {
        let mut server = ParameterServer::new(vec![0.0; 3]).with_config(ServerConfig {
            learning_rate: 1.0, // Direct application for easier testing
            batch_size: 10,
            ..Default::default()
        });

        let agent = create_test_did();
        let update = ParameterUpdate {
            agent_id: agent.id.clone(),
            update: vec![0.1, 0.2, 0.3],
            timestamp: now(),
            version_applied_to: 1,
        };

        server.pull_update(update).await.unwrap();
        let applied = server.apply_updates().await.unwrap();

        assert_eq!(applied, 1);
        assert_eq!(server.current_version(), 2);
        // Parameters should be updated: 0.0 - 1.0 * 0.1 = -0.1
        assert!((server.global_parameters[0] + 0.1).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_regional_servers() {
        let mut server = ParameterServer::new(vec![1.0; 5]);

        server.add_region("us-west");
        server.add_region("eu-central");

        assert_eq!(server.regions.len(), 2);

        let regional_params = server.get_regional_params("us-west").unwrap();
        assert_eq!(regional_params.len(), 5);
    }

    #[tokio::test]
    async fn test_sync_regions() {
        let mut server = ParameterServer::new(vec![0.0; 5]);
        server.add_region("test-region");

        // Modify global parameters
        server.global_parameters = vec![1.0; 5];
        server.version = 2;

        server.sync_regions().await.unwrap();

        let regional = server.get_regional_params("test-region").unwrap();
        assert!((regional[0] - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_batch_updates() {
        let mut server = ParameterServer::new(vec![0.0; 3]).with_config(ServerConfig {
            batch_size: 2,
            learning_rate: 1.0,
            ..Default::default()
        });

        let agent1 = create_test_did();
        let agent2 = create_test_did();
        let agent3 = create_test_did();

        // Submit 3 updates
        for (agent, val) in [(&agent1, 0.1), (&agent2, 0.2), (&agent3, 0.3)] {
            let update = ParameterUpdate {
                agent_id: agent.id.clone(),
                update: vec![val; 3],
                timestamp: now(),
                version_applied_to: 1,
            };
            server.pull_update(update).await.unwrap();
        }

        assert_eq!(server.pending_count(), 3);

        // Apply batch of 2
        let applied = server.apply_updates().await.unwrap();
        assert_eq!(applied, 2);
        assert_eq!(server.pending_count(), 1);
    }

    #[test]
    fn test_stats_tracking() {
        let server = ParameterServer::default();
        let stats = server.stats();
        assert_eq!(stats.total_updates_received, 0);
        assert_eq!(stats.current_version, 0);
    }
}

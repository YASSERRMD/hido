//! Regional server for multi-region parameter distribution.
//!
//! Maintains local copies of parameters for latency optimization.

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A regional parameter server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionalServer {
    /// Region identifier
    pub region_id: String,
    /// Local copy of parameters
    pub parameters: Vec<f32>,
    /// Last sync with global server
    pub last_sync: Timestamp,
    /// Pending updates for this region
    pub pending_updates: VecDeque<RegionalUpdate>,
    /// Local version
    pub local_version: u32,
    /// Global version at last sync
    pub global_version: u32,
}

/// A regional update.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionalUpdate {
    /// Agent ID
    pub agent_id: String,
    /// Update values
    pub update: Vec<f32>,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl RegionalServer {
    /// Create a new regional server.
    pub fn new(region_id: &str, initial_params: Vec<f32>) -> Self {
        Self {
            region_id: region_id.to_string(),
            parameters: initial_params,
            last_sync: now(),
            pending_updates: VecDeque::new(),
            local_version: 1,
            global_version: 1,
        }
    }

    /// Receive an update from a local agent.
    pub fn receive_update(&mut self, agent_id: &str, update: Vec<f32>) {
        self.pending_updates.push_back(RegionalUpdate {
            agent_id: agent_id.to_string(),
            update,
            timestamp: now(),
        });
    }

    /// Apply pending updates locally.
    pub fn apply_local_updates(&mut self, learning_rate: f32) {
        while let Some(update) = self.pending_updates.pop_front() {
            if update.update.len() == self.parameters.len() {
                for (param, upd) in self.parameters.iter_mut().zip(update.update.iter()) {
                    *param -= learning_rate * upd;
                }
                self.local_version += 1;
            }
        }
    }

    /// Sync from global server.
    pub fn sync_from_global(&mut self, global_params: Vec<f32>, global_version: u32) {
        self.parameters = global_params;
        self.global_version = global_version;
        self.last_sync = now();
    }

    /// Get updates to send to global server.
    pub fn get_updates_for_sync(&self) -> Vec<&RegionalUpdate> {
        self.pending_updates.iter().collect()
    }

    /// Check if sync is needed (stale local version).
    pub fn needs_sync(&self, current_global_version: u32) -> bool {
        self.global_version < current_global_version
    }

    /// Get pending update count.
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Get staleness (version difference).
    pub fn staleness(&self, current_global_version: u32) -> u32 {
        current_global_version.saturating_sub(self.global_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regional_server_creation() {
        let server = RegionalServer::new("us-west", vec![0.0; 10]);
        assert_eq!(server.region_id, "us-west");
        assert_eq!(server.parameters.len(), 10);
        assert_eq!(server.local_version, 1);
    }

    #[test]
    fn test_receive_update() {
        let mut server = RegionalServer::new("eu-central", vec![0.0; 5]);
        server.receive_update("agent-1", vec![0.1; 5]);

        assert_eq!(server.pending_count(), 1);
    }

    #[test]
    fn test_apply_local_updates() {
        let mut server = RegionalServer::new("ap-south", vec![0.0; 3]);
        server.receive_update("agent-1", vec![0.1, 0.2, 0.3]);

        server.apply_local_updates(1.0);

        assert_eq!(server.pending_count(), 0);
        assert!((server.parameters[0] + 0.1).abs() < 1e-5);
        assert_eq!(server.local_version, 2);
    }

    #[test]
    fn test_sync_from_global() {
        let mut server = RegionalServer::new("test", vec![0.0; 5]);
        server.sync_from_global(vec![1.0; 5], 10);

        assert!((server.parameters[0] - 1.0).abs() < 1e-5);
        assert_eq!(server.global_version, 10);
    }

    #[test]
    fn test_needs_sync() {
        let server = RegionalServer::new("test", vec![0.0; 5]);

        assert!(!server.needs_sync(1));
        assert!(server.needs_sync(2));
        assert!(server.needs_sync(10));
    }

    #[test]
    fn test_staleness() {
        let server = RegionalServer::new("test", vec![0.0; 5]);

        assert_eq!(server.staleness(1), 0);
        assert_eq!(server.staleness(5), 4);
    }
}

//! State synchronization for multi-region consistency.
//!
//! Handles cross-region state replication and conflict resolution.

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Synchronization state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    /// Fully synchronized
    Synced,
    /// Synchronization in progress
    Syncing,
    /// Behind by some versions
    Behind(u64),
    /// Conflict detected
    Conflict,
    /// Sync failed
    Failed,
}

/// A state version.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateVersion {
    /// Version number
    pub version: u64,
    /// Region that produced this version
    pub origin_region: String,
    /// Timestamp of the version
    pub timestamp: Timestamp,
    /// Hash of the state
    pub state_hash: String,
}

/// State synchronizer for cross-region consistency.
pub struct StateSynchronizer {
    /// Region ID
    region_id: String,
    /// Current local version
    local_version: u64,
    /// Remote versions
    remote_versions: HashMap<String, StateVersion>,
    /// Sync state per region
    sync_states: HashMap<String, SyncState>,
    /// Pending updates
    pending_updates: Vec<StateUpdate>,
}

/// A state update to be synchronized.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateUpdate {
    /// Update ID
    pub id: String,
    /// Version this update creates
    pub version: u64,
    /// Origin region
    pub origin: String,
    /// Update timestamp
    pub timestamp: Timestamp,
    /// Update data (serialized)
    pub data: Vec<u8>,
}

impl StateSynchronizer {
    /// Create a new synchronizer.
    pub fn new(region_id: &str) -> Self {
        Self {
            region_id: region_id.to_string(),
            local_version: 0,
            remote_versions: HashMap::new(),
            sync_states: HashMap::new(),
            pending_updates: Vec::new(),
        }
    }

    /// Register a remote region.
    pub fn register_remote(&mut self, region_id: &str) {
        self.sync_states.insert(region_id.to_string(), SyncState::Synced);
    }

    /// Get local version.
    pub fn local_version(&self) -> u64 {
        self.local_version
    }

    /// Apply a local update.
    pub fn apply_local_update(&mut self, data: Vec<u8>) -> StateUpdate {
        self.local_version += 1;

        let update = StateUpdate {
            id: uuid::Uuid::new_v4().to_string(),
            version: self.local_version,
            origin: self.region_id.clone(),
            timestamp: now(),
            data,
        };

        self.pending_updates.push(update.clone());
        update
    }

    /// Receive update from remote region.
    pub fn receive_update(&mut self, update: StateUpdate) -> SyncState {
        let remote_region = update.origin.clone();

        // Check for conflicts
        if let Some(remote_ver) = self.remote_versions.get(&remote_region) {
            if update.version <= remote_ver.version {
                // Already have this or newer version
                return SyncState::Synced;
            }

            if update.version > remote_ver.version + 1 {
                // Missing intermediate versions
                let behind = update.version - remote_ver.version - 1;
                self.sync_states.insert(remote_region.clone(), SyncState::Behind(behind));
                return SyncState::Behind(behind);
            }
        }

        // Apply update
        self.remote_versions.insert(
            remote_region.clone(),
            StateVersion {
                version: update.version,
                origin_region: update.origin,
                timestamp: update.timestamp,
                state_hash: format!("{:x}", md5_hash(&update.data)),
            },
        );

        self.sync_states.insert(remote_region, SyncState::Synced);
        SyncState::Synced
    }

    /// Get pending updates to send.
    pub fn get_pending_updates(&self) -> &[StateUpdate] {
        &self.pending_updates
    }

    /// Clear pending updates (after successful send).
    pub fn clear_pending(&mut self) {
        self.pending_updates.clear();
    }

    /// Get sync state for a region.
    pub fn get_sync_state(&self, region_id: &str) -> Option<&SyncState> {
        self.sync_states.get(region_id)
    }

    /// Check if all regions are synced.
    pub fn is_fully_synced(&self) -> bool {
        self.sync_states.values().all(|s| *s == SyncState::Synced)
    }

    /// Get regions that need catch-up.
    pub fn regions_behind(&self) -> Vec<&str> {
        self.sync_states
            .iter()
            .filter_map(|(id, state)| {
                if matches!(state, SyncState::Behind(_)) {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Simple hash function (placeholder for real implementation).
fn md5_hash(data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synchronizer_creation() {
        let sync = StateSynchronizer::new("us-west");
        assert_eq!(sync.local_version(), 0);
    }

    #[test]
    fn test_local_update() {
        let mut sync = StateSynchronizer::new("us-west");
        let update = sync.apply_local_update(b"test data".to_vec());

        assert_eq!(update.version, 1);
        assert_eq!(sync.local_version(), 1);
        assert_eq!(sync.get_pending_updates().len(), 1);
    }

    #[test]
    fn test_receive_update() {
        let mut sync = StateSynchronizer::new("us-west");
        sync.register_remote("eu-central");

        let update = StateUpdate {
            id: "update-1".to_string(),
            version: 1,
            origin: "eu-central".to_string(),
            timestamp: now(),
            data: b"remote data".to_vec(),
        };

        let state = sync.receive_update(update);
        assert_eq!(state, SyncState::Synced);
    }

    #[test]
    fn test_behind_detection() {
        let mut sync = StateSynchronizer::new("us-west");
        sync.register_remote("eu-central");

        // Receive version 1
        sync.receive_update(StateUpdate {
            id: "1".to_string(),
            version: 1,
            origin: "eu-central".to_string(),
            timestamp: now(),
            data: vec![],
        });

        // Receive version 3 (skip 2)
        let state = sync.receive_update(StateUpdate {
            id: "3".to_string(),
            version: 3,
            origin: "eu-central".to_string(),
            timestamp: now(),
            data: vec![],
        });

        assert!(matches!(state, SyncState::Behind(1)));
    }

    #[test]
    fn test_fully_synced() {
        let mut sync = StateSynchronizer::new("us-west");
        sync.register_remote("eu-central");
        sync.register_remote("ap-south");

        assert!(sync.is_fully_synced());
    }
}

//! Multi-region Module
//!
//! Provides regional deployment management:
//! - Region manager
//! - State synchronization
//! - Failover and rebalancing

pub mod failover;
pub mod manager;
pub mod sync;

pub use failover::{FailoverManager, FailoverStrategy};
pub use manager::{Region, RegionManager, RegionStatus};
pub use sync::{SyncState, StateSynchronizer};

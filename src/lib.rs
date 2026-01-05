//! # HIDO - Hierarchical Intent-Driven Orchestration
//!
//! A decentralized agent framework providing:
//! - **UAIL**: Universal Agent Identity Layer (DID-based identity)
//! - **ICC**: Intent Communication Channel (semantic intents)
//! - **BAL**: Blockchain Audit Layer (immutable audit trail)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use hido::uail::{DIDManager, DIDConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a new agent identity
//!     let mut manager = DIDManager::new(DIDConfig::default());
//!     let did = manager.generate().await.unwrap();
//!     println!("Agent DID: {}", did.id);
//! }
//! ```

pub mod audit;
pub mod bal;
pub mod compliance;
pub mod consensus;
pub mod core;
pub mod federated;
pub mod gnn;
pub mod icc;
pub mod k8s;
pub mod monitoring;
pub mod paramserver;
pub mod plugin;
pub mod region;
pub mod sla;
pub mod python;
pub mod uail;

pub use core::error::{Error, Result};

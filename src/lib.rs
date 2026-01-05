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

pub mod bal;
pub mod core;
pub mod icc;
pub mod uail;

pub use core::error::{Error, Result};

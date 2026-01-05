//! Blockchain Audit Layer (BAL)
//!
//! Provides immutable audit trail for agent actions:
//! - Content-addressed action blocks
//! - Tamper-evident blockchain
//! - Chain integrity verification

pub mod block;
pub mod chain;

pub use block::{AgentAction, AgentActionBlock, BlockVerification};
pub use chain::{AgentBlockchain, ChainMetadata, ChainVerification};

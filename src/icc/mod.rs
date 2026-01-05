//! Intent Communication Channel (ICC)
//!
//! Provides semantic intent communication:
//! - Semantic Intent structure and serialization
//! - Protocol handlers for message exchange
//! - 10x compression for efficient transmission
//! - Intent routing to capable agents

pub mod compression;
pub mod intent;
pub mod protocol;
pub mod router;

pub use compression::CompressionEngine;
pub use intent::{IntentDomain, IntentPriority, SemanticIntent};
pub use protocol::{IntentMessage, IntentProtocol, MessageType};
pub use router::{IntentRouter, RouteResult};

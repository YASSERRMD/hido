//! Consensus Module
//!
//! Provides Byzantine-fault-tolerant decision making:
//! - Byzantine voting with 2/3 quorum
//! - Ethical guardrails for decision validation
//! - Decision engine orchestration

pub mod engine;
pub mod guardrails;
pub mod voting;

pub use engine::{DecisionEngine, DecisionExplanation};
pub use guardrails::{EthicalGuardrail, GuardrailAction, RuleType};
pub use voting::{ByzantineVoting, ConsensusResult, Vote, VoteType};

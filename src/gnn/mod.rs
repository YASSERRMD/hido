//! Graph Neural Network (GNN) Module
//!
//! Provides graph-based learning for agent relationships:
//! - Graph Attention (GATE) for agent interaction scoring
//! - Temporal encoding (HADTE) for time-aware processing
//! - GNN Learner for decision prediction

pub mod attention;
pub mod learner;
pub mod temporal;

pub use attention::{AttentionHead, AttentionWeights, GraphAttention};
pub use learner::{GNNLearner, PredictedDecision};
pub use temporal::TemporalEncoder;

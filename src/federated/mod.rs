//! Federated Learning Module
//!
//! Provides privacy-preserving distributed learning:
//! - Local training on individual agents
//! - Gradient aggregation with FedAvg
//! - Differential privacy support
//! - Non-IID data handling

pub mod aggregator;
pub mod learner;
pub mod noniid;

pub use aggregator::{AggregationMethod, GradientAggregator};
pub use learner::{LocalLearner, LocalModel, TrainingResult};
pub use noniid::{DataDistribution, NonIIDHandler};

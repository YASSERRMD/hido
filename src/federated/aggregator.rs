//! Gradient aggregation for federated learning.
//!
//! Implements FedAvg and differential privacy.

use crate::core::{now, Result, Timestamp};
use crate::uail::DIDKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregation methods.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationMethod {
    /// Simple averaging
    FedAvg,
    /// With proximal regularization
    FedProx,
    /// With differential privacy
    DifferentialPrivacy,
}

impl Default for AggregationMethod {
    fn default() -> Self {
        Self::FedAvg
    }
}

/// Gradients from a participant.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParticipantGradients {
    /// Agent ID
    pub agent_id: String,
    /// Gradients
    pub gradients: Vec<f32>,
    /// Weight (typically data size fraction)
    pub weight: f32,
    /// Submission timestamp
    pub timestamp: Timestamp,
}

/// Gradient aggregator for federated learning.
pub struct GradientAggregator {
    /// Aggregation method
    pub aggregation_method: AggregationMethod,
    /// Submitted gradients
    participants: HashMap<String, ParticipantGradients>,
    /// Current aggregation round
    pub aggregation_round: u32,
    /// Total gradient dimension
    gradient_dim: usize,
}

impl GradientAggregator {
    /// Create a new aggregator.
    pub fn new(method: AggregationMethod, gradient_dim: usize) -> Self {
        Self {
            aggregation_method: method,
            participants: HashMap::new(),
            aggregation_round: 1,
            gradient_dim,
        }
    }

    /// Submit gradients from an agent.
    pub fn submit_gradients(
        &mut self,
        agent: &DIDKey,
        gradients: Vec<f32>,
        weight: f32,
    ) -> Result<()> {
        if gradients.len() != self.gradient_dim {
            return Err(crate::core::Error::Internal(format!(
                "Gradient dimension mismatch: expected {}, got {}",
                self.gradient_dim,
                gradients.len()
            )));
        }

        self.participants.insert(
            agent.id.clone(),
            ParticipantGradients {
                agent_id: agent.id.clone(),
                gradients,
                weight: weight.clamp(0.0, 1.0),
                timestamp: now(),
            },
        );

        Ok(())
    }

    /// Compute aggregated gradient using FedAvg.
    /// REQUIREMENT: Weighted average with O(n) complexity.
    pub fn aggregate(&self) -> Result<Vec<f32>> {
        if self.participants.is_empty() {
            return Ok(vec![0.0; self.gradient_dim]);
        }

        let total_weight: f32 = self.participants.values().map(|p| p.weight).sum();

        if total_weight <= 0.0 {
            return Err(crate::core::Error::Internal(
                "Total weight must be positive".to_string(),
            ));
        }

        let mut aggregated = vec![0.0; self.gradient_dim];

        for participant in self.participants.values() {
            let normalized_weight = participant.weight / total_weight;
            for (i, grad) in participant.gradients.iter().enumerate() {
                aggregated[i] += normalized_weight * grad;
            }
        }

        Ok(aggregated)
    }

    /// Compute aggregated gradient with differential privacy.
    /// REQUIREMENT: Add Laplace noise to protect individual contributions.
    pub fn aggregate_with_privacy(&self, epsilon: f32) -> Result<Vec<f32>> {
        let mut aggregated = self.aggregate()?;

        // Add Laplace noise for differential privacy
        // Scale = sensitivity / epsilon
        // For gradient averaging, sensitivity is bounded by gradient clipping
        let sensitivity = 1.0; // Assuming gradients are clipped to [-1, 1]
        let scale = sensitivity / epsilon;

        use rand::Rng;
        let mut rng = rand::thread_rng();

        for val in &mut aggregated {
            // Laplace noise: sample from Laplace(0, scale)
            let u: f32 = rng.gen::<f32>() - 0.5;
            let noise = -scale * u.signum() * (1.0 - 2.0 * u.abs()).ln();
            *val += noise;
        }

        Ok(aggregated)
    }

    /// Compute aggregated gradient with FedProx regularization.
    pub fn aggregate_with_prox(&self, global_params: &[f32], mu: f32) -> Result<Vec<f32>> {
        let mut aggregated = self.aggregate()?;

        // FedProx adds proximal term: mu/2 * ||w - w_global||^2
        // This affects the gradient update, not aggregation directly
        // We return the adjusted gradient
        for i in 0..aggregated.len() {
            if i < global_params.len() {
                // Proximal term gradient contribution
                // In practice, this would be applied during local training
                let diff = aggregated[i] - global_params[i];
                aggregated[i] += mu * diff;
            }
        }

        Ok(aggregated)
    }

    /// Get participant count.
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Clear all submissions and advance round.
    pub fn next_round(&mut self) {
        self.participants.clear();
        self.aggregation_round += 1;
    }

    /// Get current round.
    pub fn current_round(&self) -> u32 {
        self.aggregation_round
    }

    /// Check if we have enough participants.
    pub fn has_quorum(&self, min_participants: usize) -> bool {
        self.participants.len() >= min_participants
    }

    /// Get all participant IDs.
    pub fn participant_ids(&self) -> Vec<&String> {
        self.participants.keys().collect()
    }
}

impl Default for GradientAggregator {
    fn default() -> Self {
        Self::new(AggregationMethod::FedAvg, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uail::crypto::CryptoSuite;

    fn create_test_did() -> DIDKey {
        let crypto = CryptoSuite::new();
        DIDKey::new(&crypto)
    }

    #[test]
    fn test_aggregator_creation() {
        let agg = GradientAggregator::new(AggregationMethod::FedAvg, 10);
        assert_eq!(agg.participant_count(), 0);
        assert_eq!(agg.current_round(), 1);
    }

    #[test]
    fn test_submit_gradients() {
        let mut agg = GradientAggregator::new(AggregationMethod::FedAvg, 3);
        let agent = create_test_did();

        agg.submit_gradients(&agent, vec![1.0, 2.0, 3.0], 1.0).unwrap();
        assert_eq!(agg.participant_count(), 1);
    }

    #[test]
    fn test_fedavg_aggregation() {
        let mut agg = GradientAggregator::new(AggregationMethod::FedAvg, 3);

        let agent1 = create_test_did();
        let agent2 = create_test_did();

        // Equal weights
        agg.submit_gradients(&agent1, vec![1.0, 2.0, 3.0], 0.5).unwrap();
        agg.submit_gradients(&agent2, vec![3.0, 4.0, 5.0], 0.5).unwrap();

        let aggregated = agg.aggregate().unwrap();
        // Average: [2.0, 3.0, 4.0]
        assert!((aggregated[0] - 2.0).abs() < 1e-5);
        assert!((aggregated[1] - 3.0).abs() < 1e-5);
        assert!((aggregated[2] - 4.0).abs() < 1e-5);
    }

    #[test]
    fn test_weighted_aggregation() {
        let mut agg = GradientAggregator::new(AggregationMethod::FedAvg, 2);

        let agent1 = create_test_did();
        let agent2 = create_test_did();

        // Different weights (agent1 has more data)
        agg.submit_gradients(&agent1, vec![1.0, 1.0], 0.8).unwrap();
        agg.submit_gradients(&agent2, vec![2.0, 2.0], 0.2).unwrap();

        let aggregated = agg.aggregate().unwrap();
        // Weighted average: (0.8*1.0 + 0.2*2.0) = 1.2
        assert!((aggregated[0] - 1.2).abs() < 1e-5);
    }

    #[test]
    fn test_differential_privacy() {
        let mut agg = GradientAggregator::new(AggregationMethod::DifferentialPrivacy, 10);
        let agent = create_test_did();

        agg.submit_gradients(&agent, vec![1.0; 10], 1.0).unwrap();

        let result1 = agg.aggregate_with_privacy(1.0).unwrap();
        let result2 = agg.aggregate_with_privacy(1.0).unwrap();

        // Results should differ due to noise
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_next_round() {
        let mut agg = GradientAggregator::new(AggregationMethod::FedAvg, 5);
        let agent = create_test_did();

        agg.submit_gradients(&agent, vec![1.0; 5], 1.0).unwrap();
        assert_eq!(agg.participant_count(), 1);

        agg.next_round();
        assert_eq!(agg.participant_count(), 0);
        assert_eq!(agg.current_round(), 2);
    }

    #[test]
    fn test_has_quorum() {
        let mut agg = GradientAggregator::new(AggregationMethod::FedAvg, 3);

        assert!(!agg.has_quorum(2));

        let agent1 = create_test_did();
        let agent2 = create_test_did();

        agg.submit_gradients(&agent1, vec![1.0; 3], 0.5).unwrap();
        assert!(!agg.has_quorum(2));

        agg.submit_gradients(&agent2, vec![2.0; 3], 0.5).unwrap();
        assert!(agg.has_quorum(2));
    }

    #[test]
    fn test_gradient_dimension_mismatch() {
        let mut agg = GradientAggregator::new(AggregationMethod::FedAvg, 5);
        let agent = create_test_did();

        let result = agg.submit_gradients(&agent, vec![1.0, 2.0], 1.0);
        assert!(result.is_err());
    }
}

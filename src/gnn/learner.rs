//! GNN Learner for agent decision prediction.
//!
//! Combines graph attention with temporal encoding for learning.

use crate::core::Result;
use crate::gnn::attention::{AttentionConfig, GraphAttention};
use crate::gnn::temporal::{TemporalConfig, TemporalEncoder, TemporalEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the GNN learner.
#[derive(Clone, Debug)]
pub struct LearnerConfig {
    /// Input embedding dimension
    pub input_dim: usize,
    /// Hidden dimension
    pub hidden_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Number of GNN layers
    pub num_layers: usize,
    /// Learning rate
    pub learning_rate: f32,
    /// Attention configuration
    pub attention_config: AttentionConfig,
    /// Temporal configuration
    pub temporal_config: TemporalConfig,
}

impl Default for LearnerConfig {
    fn default() -> Self {
        Self {
            input_dim: 64,
            hidden_dim: 128,
            output_dim: 32,
            num_layers: 2,
            learning_rate: 0.001,
            attention_config: AttentionConfig::default(),
            temporal_config: TemporalConfig::default(),
        }
    }
}

/// Predicted decision from the GNN.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PredictedDecision {
    /// Predicted agent ID
    pub agent_id: String,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Alternative agents with scores
    pub alternatives: Vec<(String, f32)>,
    /// Explanation features
    pub explanation: HashMap<String, f32>,
}

/// Training result from one epoch.
#[derive(Clone, Debug)]
pub struct TrainingResult {
    /// Average loss
    pub loss: f32,
    /// Number of samples processed
    pub samples: usize,
    /// Accuracy on training data
    pub accuracy: f32,
}

/// Training sample for supervised learning.
#[derive(Clone, Debug)]
pub struct TrainingSample {
    /// Node embeddings
    pub embeddings: HashMap<String, Vec<f32>>,
    /// Graph edges
    pub edges: Vec<(String, String)>,
    /// Query node
    pub query_node: String,
    /// Target (correct answer)
    pub target: String,
    /// Historical events
    pub events: Vec<TemporalEvent>,
}

/// GNN Learner for decision prediction.
pub struct GNNLearner {
    /// Configuration
    pub config: LearnerConfig,
    /// Graph attention layers
    attention_layers: Vec<GraphAttention>,
    /// Temporal encoder
    temporal_encoder: TemporalEncoder,
    /// Output projection weights
    output_weights: Vec<f32>,
    /// Training metrics
    metrics: LearnerMetrics,
}

/// Metrics tracked during training.
#[derive(Clone, Debug, Default)]
pub struct LearnerMetrics {
    pub epochs_trained: u32,
    pub total_loss: f32,
    pub best_accuracy: f32,
}

impl GNNLearner {
    /// Create a new GNN learner.
    pub fn new(config: LearnerConfig) -> Self {
        // Create attention layers
        let mut attention_layers = Vec::with_capacity(config.num_layers);
        let mut current_dim = config.input_dim;

        for i in 0..config.num_layers {
            let out_dim = if i == config.num_layers - 1 {
                config.hidden_dim
            } else {
                config.hidden_dim
            };

            attention_layers.push(GraphAttention::new(
                current_dim,
                out_dim,
                config.attention_config.clone(),
            ));
            current_dim = out_dim;
        }

        // Create temporal encoder
        let temporal_encoder = TemporalEncoder::new(config.temporal_config.clone());

        // Initialize output weights
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let output_weights: Vec<f32> = (0..config.hidden_dim * config.output_dim)
            .map(|_| rng.gen::<f32>() * 0.1 - 0.05)
            .collect();

        Self {
            config,
            attention_layers,
            temporal_encoder,
            output_weights,
            metrics: LearnerMetrics::default(),
        }
    }

    /// Forward pass through the GNN.
    pub fn forward(
        &self,
        embeddings: &HashMap<String, Vec<f32>>,
        edges: &[(String, String)],
        events: &[TemporalEvent],
    ) -> Result<HashMap<String, Vec<f32>>> {
        let mut current_embeddings = embeddings.clone();

        // Apply temporal encoding to embeddings if events provided
        if !events.is_empty() {
            let encoded = self.temporal_encoder.encode_sequence(events);
            for (event, enc) in events.iter().zip(encoded.iter()) {
                if let Some(emb) = current_embeddings.get_mut(&event.id) {
                    for (i, v) in enc.iter().enumerate() {
                        if i < emb.len() {
                            emb[i] += v * 0.1; // Blend temporal info
                        }
                    }
                }
            }
        }

        // Forward through attention layers
        for layer in &self.attention_layers {
            current_embeddings = layer.forward(&current_embeddings, edges)?;
        }

        Ok(current_embeddings)
    }

    /// Predict the best agent for a query.
    pub fn predict(
        &self,
        embeddings: &HashMap<String, Vec<f32>>,
        edges: &[(String, String)],
        query_node: &str,
        candidates: &[String],
        events: &[TemporalEvent],
    ) -> Result<PredictedDecision> {
        // Forward pass
        let updated = self.forward(embeddings, edges, events)?;

        // Get query embedding
        let query_emb = updated
            .get(query_node)
            .cloned()
            .unwrap_or_else(|| vec![0.0; self.config.hidden_dim]);

        // Score candidates
        let mut scores: Vec<(String, f32)> = candidates
            .iter()
            .filter_map(|c| {
                updated.get(c).map(|emb| {
                    let score = Self::cosine_similarity(&query_emb, emb);
                    (c.clone(), score)
                })
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply softmax to get probabilities
        let mut probs: Vec<f32> = scores.iter().map(|(_, s)| *s).collect();
        Self::softmax_inplace(&mut probs);

        let best = scores.first().cloned().unwrap_or(("unknown".to_string(), 0.0));
        let alternatives: Vec<(String, f32)> = scores
            .iter()
            .skip(1)
            .take(3)
            .zip(probs.iter().skip(1))
            .map(|((id, _), &prob)| (id.clone(), prob))
            .collect();

        Ok(PredictedDecision {
            agent_id: best.0,
            confidence: probs.first().cloned().unwrap_or(0.0),
            alternatives,
            explanation: HashMap::new(),
        })
    }

    /// Train on a batch of samples.
    pub async fn train(&mut self, samples: &[TrainingSample]) -> Result<TrainingResult> {
        let mut total_loss = 0.0;
        let mut correct = 0;

        for sample in samples {
            // Forward pass
            let prediction = self.predict(
                &sample.embeddings,
                &sample.edges,
                &sample.query_node,
                &sample
                    .embeddings
                    .keys()
                    .filter(|k| *k != &sample.query_node)
                    .cloned()
                    .collect::<Vec<_>>(),
                &sample.events,
            )?;

            // Compute loss (cross-entropy)
            let loss = if prediction.agent_id == sample.target {
                -prediction.confidence.ln()
            } else {
                -(1.0 - prediction.confidence).ln()
            };

            total_loss += loss;

            if prediction.agent_id == sample.target {
                correct += 1;
            }

            // Gradient computation and update would go here
            // For now, we simulate basic weight updates
            self.update_weights(loss);
        }

        let accuracy = correct as f32 / samples.len() as f32;
        self.metrics.epochs_trained += 1;
        self.metrics.total_loss += total_loss;
        if accuracy > self.metrics.best_accuracy {
            self.metrics.best_accuracy = accuracy;
        }

        Ok(TrainingResult {
            loss: total_loss / samples.len() as f32,
            samples: samples.len(),
            accuracy,
        })
    }

    /// Update weights based on loss (simplified gradient descent).
    fn update_weights(&mut self, _loss: f32) {
        // In a real implementation, this would compute gradients
        // and apply optimizer updates. For now, we apply small random updates.
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let lr = self.config.learning_rate;

        for w in &mut self.output_weights {
            *w -= lr * (rng.gen::<f32>() - 0.5) * 0.01;
        }
    }

    /// Compute cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    /// Apply softmax in-place.
    fn softmax_inplace(scores: &mut [f32]) {
        if scores.is_empty() {
            return;
        }

        let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0.0;

        for s in scores.iter_mut() {
            *s = (*s - max).exp();
            sum += *s;
        }

        if sum > 0.0 {
            for s in scores.iter_mut() {
                *s /= sum;
            }
        }
    }

    /// Get training metrics.
    pub fn metrics(&self) -> &LearnerMetrics {
        &self.metrics
    }
}

impl Default for GNNLearner {
    fn default() -> Self {
        Self::new(LearnerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sample() -> TrainingSample {
        let mut embeddings = HashMap::new();
        embeddings.insert("query".to_string(), vec![0.1; 64]);
        embeddings.insert("agent1".to_string(), vec![0.2; 64]);
        embeddings.insert("agent2".to_string(), vec![0.3; 64]);

        TrainingSample {
            embeddings,
            edges: vec![
                ("query".to_string(), "agent1".to_string()),
                ("query".to_string(), "agent2".to_string()),
            ],
            query_node: "query".to_string(),
            target: "agent1".to_string(),
            events: Vec::new(),
        }
    }

    #[test]
    fn test_learner_creation() {
        let learner = GNNLearner::default();
        assert_eq!(learner.config.num_layers, 2);
        assert_eq!(learner.attention_layers.len(), 2);
    }

    #[test]
    fn test_forward_pass() {
        let learner = GNNLearner::new(LearnerConfig {
            input_dim: 16,
            hidden_dim: 32,
            output_dim: 8,
            num_layers: 1,
            attention_config: AttentionConfig {
                num_heads: 2,
                head_dim: 8,
                ..Default::default()
            },
            ..Default::default()
        });

        let mut embeddings = HashMap::new();
        embeddings.insert("a".to_string(), vec![0.1; 16]);
        embeddings.insert("b".to_string(), vec![0.2; 16]);

        let edges = vec![("a".to_string(), "b".to_string())];

        let result = learner.forward(&embeddings, &edges, &[]).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_predict() {
        let learner = GNNLearner::new(LearnerConfig {
            input_dim: 16,
            hidden_dim: 16,
            output_dim: 8,
            num_layers: 1,
            attention_config: AttentionConfig {
                num_heads: 2,
                head_dim: 4,
                ..Default::default()
            },
            ..Default::default()
        });

        let mut embeddings = HashMap::new();
        embeddings.insert("query".to_string(), vec![0.1; 16]);
        embeddings.insert("candidate1".to_string(), vec![0.15; 16]);
        embeddings.insert("candidate2".to_string(), vec![0.5; 16]);

        let edges = vec![
            ("query".to_string(), "candidate1".to_string()),
            ("query".to_string(), "candidate2".to_string()),
        ];

        let candidates = vec!["candidate1".to_string(), "candidate2".to_string()];

        let prediction =
            learner.predict(&embeddings, &edges, "query", &candidates, &[]).unwrap();

        assert!(prediction.confidence > 0.0);
        assert!(prediction.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_training() {
        let mut learner = GNNLearner::new(LearnerConfig {
            input_dim: 64,
            hidden_dim: 32,
            output_dim: 16,
            num_layers: 1,
            attention_config: AttentionConfig {
                num_heads: 2,
                head_dim: 8,
                ..Default::default()
            },
            ..Default::default()
        });

        let samples = vec![create_test_sample()];
        let result = learner.train(&samples).await.unwrap();

        assert_eq!(result.samples, 1);
        assert!(result.loss >= 0.0);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = GNNLearner::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5);

        let c = vec![0.0, 1.0, 0.0];
        let sim2 = GNNLearner::cosine_similarity(&a, &c);
        assert!(sim2.abs() < 1e-5);
    }

    #[test]
    fn test_metrics() {
        let learner = GNNLearner::default();
        let metrics = learner.metrics();
        assert_eq!(metrics.epochs_trained, 0);
    }
}

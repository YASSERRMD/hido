//! Graph Attention mechanism for agent relationships.
//!
//! Implements multi-head attention for scoring agent interactions.

use crate::core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for attention mechanism.
#[derive(Clone, Debug)]
pub struct AttentionConfig {
    /// Number of attention heads
    pub num_heads: usize,
    /// Dimension of each head
    pub head_dim: usize,
    /// Dropout rate
    pub dropout: f32,
    /// Whether to use scaled attention
    pub scaled: bool,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            num_heads: 4,
            head_dim: 64,
            dropout: 0.1,
            scaled: true,
        }
    }
}

/// A single attention head.
#[derive(Clone, Debug)]
pub struct AttentionHead {
    /// Head index
    pub index: usize,
    /// Query weights
    pub w_query: Vec<f32>,
    /// Key weights
    pub w_key: Vec<f32>,
    /// Value weights
    pub w_value: Vec<f32>,
    /// Head dimension
    pub dim: usize,
}

impl AttentionHead {
    /// Create a new attention head with random initialization.
    pub fn new(index: usize, input_dim: usize, head_dim: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let scale = (2.0 / (input_dim + head_dim) as f32).sqrt();

        let w_query: Vec<f32> = (0..input_dim * head_dim)
            .map(|_| rng.gen::<f32>() * scale - scale / 2.0)
            .collect();
        let w_key: Vec<f32> = (0..input_dim * head_dim)
            .map(|_| rng.gen::<f32>() * scale - scale / 2.0)
            .collect();
        let w_value: Vec<f32> = (0..input_dim * head_dim)
            .map(|_| rng.gen::<f32>() * scale - scale / 2.0)
            .collect();

        Self {
            index,
            w_query,
            w_key,
            w_value,
            dim: head_dim,
        }
    }

    /// Compute attention scores for a single head.
    pub fn compute(&self, query: &[f32], keys: &[Vec<f32>], values: &[Vec<f32>]) -> Vec<f32> {
        // Project query
        let q = self.project(query, &self.w_query);

        // Compute attention scores
        let mut scores: Vec<f32> = keys
            .iter()
            .map(|k| {
                let k_proj = self.project(k, &self.w_key);
                dot_product(&q, &k_proj) / (self.dim as f32).sqrt()
            })
            .collect();

        // Softmax
        softmax(&mut scores);

        // Weighted sum of values
        let mut output = vec![0.0; self.dim];
        for (i, v) in values.iter().enumerate() {
            let v_proj = self.project(v, &self.w_value);
            for (j, val) in v_proj.iter().enumerate() {
                output[j] += scores[i] * val;
            }
        }

        output
    }

    fn project(&self, input: &[f32], weights: &[f32]) -> Vec<f32> {
        let _input_dim = input.len();
        let mut output = vec![0.0; self.dim];

        for i in 0..self.dim {
            for (j, &inp) in input.iter().enumerate() {
                if j * self.dim + i < weights.len() {
                    output[i] += inp * weights[j * self.dim + i];
                }
            }
        }
        output
    }
}

/// Attention weights for visualization and interpretability.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttentionWeights {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Attention weight
    pub weight: f32,
    /// Head index
    pub head: usize,
}

/// Multi-head Graph Attention mechanism.
pub struct GraphAttention {
    /// Attention heads
    pub heads: Vec<AttentionHead>,
    /// Configuration
    pub config: AttentionConfig,
    /// Output projection weights
    pub w_output: Vec<f32>,
    /// Input dimension
    pub input_dim: usize,
    /// Output dimension
    pub output_dim: usize,
}

impl GraphAttention {
    /// Create a new multi-head graph attention layer.
    pub fn new(input_dim: usize, output_dim: usize, config: AttentionConfig) -> Self {
        let heads: Vec<AttentionHead> = (0..config.num_heads)
            .map(|i| AttentionHead::new(i, input_dim, config.head_dim))
            .collect();

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let concat_dim = config.num_heads * config.head_dim;
        let scale = (2.0 / (concat_dim + output_dim) as f32).sqrt();
        let w_output: Vec<f32> = (0..concat_dim * output_dim)
            .map(|_| rng.gen::<f32>() * scale - scale / 2.0)
            .collect();

        Self {
            heads,
            config,
            w_output,
            input_dim,
            output_dim,
        }
    }

    /// Compute attention over a graph.
    /// Returns updated node embeddings.
    pub fn forward(
        &self,
        node_embeddings: &HashMap<String, Vec<f32>>,
        edges: &[(String, String)],
    ) -> Result<HashMap<String, Vec<f32>>> {
        let mut outputs: HashMap<String, Vec<f32>> = HashMap::new();

        for (node_id, embedding) in node_embeddings {
            // Find neighbors
            let neighbors: Vec<&String> = edges
                .iter()
                .filter(|(src, _)| src == node_id)
                .map(|(_, dst)| dst)
                .collect();

            if neighbors.is_empty() {
                // No neighbors, just project self
                outputs.insert(node_id.clone(), embedding.clone());
                continue;
            }

            // Get neighbor embeddings
            let neighbor_embeddings: Vec<Vec<f32>> = neighbors
                .iter()
                .filter_map(|n| node_embeddings.get(*n).cloned())
                .collect();

            // Compute multi-head attention
            let mut head_outputs: Vec<Vec<f32>> = Vec::new();
            for head in &self.heads {
                let output = head.compute(embedding, &neighbor_embeddings, &neighbor_embeddings);
                head_outputs.push(output);
            }

            // Concatenate heads
            let concat: Vec<f32> = head_outputs.into_iter().flatten().collect();

            // Project to output dimension
            let mut final_output = vec![0.0; self.output_dim];
            for i in 0..self.output_dim {
                for (j, &c) in concat.iter().enumerate() {
                    if j * self.output_dim + i < self.w_output.len() {
                        final_output[i] += c * self.w_output[j * self.output_dim + i];
                    }
                }
            }

            // Apply ReLU
            for v in &mut final_output {
                *v = v.max(0.0);
            }

            outputs.insert(node_id.clone(), final_output);
        }

        Ok(outputs)
    }

    /// Get attention weights for interpretability.
    pub fn get_attention_weights(
        &self,
        node_embeddings: &HashMap<String, Vec<f32>>,
        edges: &[(String, String)],
    ) -> Vec<AttentionWeights> {
        let mut weights = Vec::new();

        for (node_id, embedding) in node_embeddings {
            let neighbors: Vec<&String> = edges
                .iter()
                .filter(|(src, _)| src == node_id)
                .map(|(_, dst)| dst)
                .collect();

            if neighbors.is_empty() {
                continue;
            }

            let neighbor_embeddings: Vec<Vec<f32>> = neighbors
                .iter()
                .filter_map(|n| node_embeddings.get(*n).cloned())
                .collect();

            for (head_idx, head) in self.heads.iter().enumerate() {
                // Project query
                let q = head.project(embedding, &head.w_query);

                // Compute and softmax scores
                let mut scores: Vec<f32> = neighbor_embeddings
                    .iter()
                    .map(|k| {
                        let k_proj = head.project(k, &head.w_key);
                        dot_product(&q, &k_proj) / (head.dim as f32).sqrt()
                    })
                    .collect();
                softmax(&mut scores);

                // Store weights
                for (i, neighbor) in neighbors.iter().enumerate() {
                    weights.push(AttentionWeights {
                        source: node_id.clone(),
                        target: (*neighbor).clone(),
                        weight: scores[i],
                        head: head_idx,
                    });
                }
            }
        }

        weights
    }
}

/// Compute dot product of two vectors.
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Apply softmax to a vector in-place.
fn softmax(scores: &mut [f32]) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attention_head_creation() {
        let head = AttentionHead::new(0, 64, 32);
        assert_eq!(head.index, 0);
        assert_eq!(head.dim, 32);
        assert_eq!(head.w_query.len(), 64 * 32);
    }

    #[test]
    fn test_attention_head_compute() {
        let head = AttentionHead::new(0, 8, 4);
        let query = vec![0.1; 8];
        let keys = vec![vec![0.2; 8], vec![0.3; 8]];
        let values = vec![vec![0.4; 8], vec![0.5; 8]];

        let output = head.compute(&query, &keys, &values);
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_graph_attention_creation() {
        let config = AttentionConfig::default();
        let ga = GraphAttention::new(64, 32, config.clone());
        assert_eq!(ga.heads.len(), config.num_heads);
        assert_eq!(ga.input_dim, 64);
        assert_eq!(ga.output_dim, 32);
    }

    #[test]
    fn test_graph_attention_forward() {
        let config = AttentionConfig {
            num_heads: 2,
            head_dim: 8,
            ..Default::default()
        };
        let ga = GraphAttention::new(16, 16, config);

        let mut embeddings = HashMap::new();
        embeddings.insert("agent1".to_string(), vec![0.1; 16]);
        embeddings.insert("agent2".to_string(), vec![0.2; 16]);
        embeddings.insert("agent3".to_string(), vec![0.3; 16]);

        let edges = vec![
            ("agent1".to_string(), "agent2".to_string()),
            ("agent1".to_string(), "agent3".to_string()),
            ("agent2".to_string(), "agent1".to_string()),
        ];

        let outputs = ga.forward(&embeddings, &edges).unwrap();
        assert_eq!(outputs.len(), 3);
        assert!(outputs.contains_key("agent1"));
    }

    #[test]
    fn test_get_attention_weights() {
        let config = AttentionConfig {
            num_heads: 2,
            head_dim: 4,
            ..Default::default()
        };
        let ga = GraphAttention::new(8, 8, config);

        let mut embeddings = HashMap::new();
        embeddings.insert("a".to_string(), vec![0.1; 8]);
        embeddings.insert("b".to_string(), vec![0.2; 8]);

        let edges = vec![("a".to_string(), "b".to_string())];

        let weights = ga.get_attention_weights(&embeddings, &edges);
        assert!(!weights.is_empty());
        assert!(weights.iter().all(|w| w.weight >= 0.0 && w.weight <= 1.0));
    }

    #[test]
    fn test_softmax() {
        let mut scores = vec![1.0, 2.0, 3.0];
        softmax(&mut scores);
        let sum: f32 = scores.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = dot_product(&a, &b);
        assert!((result - 32.0).abs() < 1e-5);
    }
}

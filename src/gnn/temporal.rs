//! Temporal encoding for time-aware attention.
//!
//! Implements HADTE (Hierarchical Attention with Decay Temporal Encoding).

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};

/// Configuration for temporal encoding.
#[derive(Clone, Debug)]
pub struct TemporalConfig {
    /// Maximum sequence length
    pub max_seq_len: usize,
    /// Embedding dimension
    pub embed_dim: usize,
    /// Time decay factor (higher = faster decay)
    pub decay_factor: f32,
    /// Whether to use positional encoding
    pub use_positional: bool,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            max_seq_len: 1000,
            embed_dim: 64,
            decay_factor: 0.1,
            use_positional: true,
        }
    }
}

/// Temporal encoder for time-aware processing.
pub struct TemporalEncoder {
    /// Configuration
    pub config: TemporalConfig,
    /// Positional encoding matrix
    positional_encoding: Vec<Vec<f32>>,
}

impl TemporalEncoder {
    /// Create a new temporal encoder.
    pub fn new(config: TemporalConfig) -> Self {
        let positional_encoding = Self::create_positional_encoding(
            config.max_seq_len,
            config.embed_dim,
        );

        Self {
            config,
            positional_encoding,
        }
    }

    /// Create sinusoidal positional encoding.
    fn create_positional_encoding(max_len: usize, dim: usize) -> Vec<Vec<f32>> {
        let mut encoding = Vec::with_capacity(max_len);

        for pos in 0..max_len {
            let mut pos_encoding = Vec::with_capacity(dim);
            for i in 0..dim {
                let angle = (pos as f32) / 10000_f32.powf((2 * (i / 2)) as f32 / dim as f32);
                if i % 2 == 0 {
                    pos_encoding.push(angle.sin());
                } else {
                    pos_encoding.push(angle.cos());
                }
            }
            encoding.push(pos_encoding);
        }

        encoding
    }

    /// Get positional encoding for a position.
    pub fn get_positional_encoding(&self, position: usize) -> Option<&Vec<f32>> {
        self.positional_encoding.get(position)
    }

    /// Compute time decay weight.
    /// More recent events have higher weights.
    pub fn time_decay(&self, event_time: Timestamp, reference_time: Timestamp) -> f32 {
        let diff_seconds = (reference_time - event_time).num_seconds().abs() as f32;
        (-self.config.decay_factor * diff_seconds / 3600.0).exp() // decay per hour
    }

    /// Encode a sequence of events with temporal information.
    pub fn encode_sequence(&self, events: &[TemporalEvent]) -> Vec<Vec<f32>> {
        let reference_time = now();
        let mut encoded = Vec::with_capacity(events.len());

        for (i, event) in events.iter().enumerate() {
            let mut embedding = event.embedding.clone();

            // Apply time decay
            let decay = self.time_decay(event.timestamp, reference_time);
            for v in &mut embedding {
                *v *= decay;
            }

            // Add positional encoding if enabled
            if self.config.use_positional {
                if let Some(pos_enc) = self.get_positional_encoding(i) {
                    for (j, v) in embedding.iter_mut().enumerate() {
                        if j < pos_enc.len() {
                            *v += pos_enc[j];
                        }
                    }
                }
            }

            encoded.push(embedding);
        }

        encoded
    }

    /// Compute temporal attention weights.
    /// Combines content attention with time decay.
    pub fn temporal_attention(
        &self,
        _query: &[f32],
        events: &[TemporalEvent],
        content_scores: &[f32],
    ) -> Vec<f32> {
        let reference_time = now();

        let mut weights: Vec<f32> = events
            .iter()
            .zip(content_scores.iter())
            .map(|(event, &score)| {
                let decay = self.time_decay(event.timestamp, reference_time);
                score * decay
            })
            .collect();

        // Normalize
        let sum: f32 = weights.iter().sum();
        if sum > 0.0 {
            for w in &mut weights {
                *w /= sum;
            }
        }

        weights
    }
}

impl Default for TemporalEncoder {
    fn default() -> Self {
        Self::new(TemporalConfig::default())
    }
}

/// A temporal event with timestamp and embedding.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalEvent {
    /// Event ID
    pub id: String,
    /// Event timestamp
    pub timestamp: Timestamp,
    /// Event embedding
    pub embedding: Vec<f32>,
    /// Event type
    pub event_type: String,
}

impl TemporalEvent {
    /// Create a new temporal event.
    pub fn new(id: &str, event_type: &str, embedding: Vec<f32>) -> Self {
        Self {
            id: id.to_string(),
            timestamp: now(),
            embedding,
            event_type: event_type.to_string(),
        }
    }

    /// Create with specific timestamp.
    pub fn with_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = timestamp;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_temporal_encoder_creation() {
        let encoder = TemporalEncoder::default();
        assert_eq!(encoder.config.max_seq_len, 1000);
        assert_eq!(encoder.positional_encoding.len(), 1000);
    }

    #[test]
    fn test_positional_encoding() {
        let encoder = TemporalEncoder::new(TemporalConfig {
            max_seq_len: 10,
            embed_dim: 8,
            ..Default::default()
        });

        let pos0 = encoder.get_positional_encoding(0).unwrap();
        let pos1 = encoder.get_positional_encoding(1).unwrap();

        assert_eq!(pos0.len(), 8);
        assert_ne!(pos0, pos1);
    }

    #[test]
    fn test_time_decay() {
        let encoder = TemporalEncoder::default();
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);

        let decay_now = encoder.time_decay(now, now);
        let decay_1h = encoder.time_decay(one_hour_ago, now);

        assert!((decay_now - 1.0).abs() < 0.01);
        assert!(decay_1h < decay_now);
        assert!(decay_1h > 0.0);
    }

    #[test]
    fn test_encode_sequence() {
        let encoder = TemporalEncoder::new(TemporalConfig {
            max_seq_len: 100,
            embed_dim: 4,
            ..Default::default()
        });

        let events = vec![
            TemporalEvent::new("e1", "action", vec![1.0, 2.0, 3.0, 4.0]),
            TemporalEvent::new("e2", "action", vec![5.0, 6.0, 7.0, 8.0]),
        ];

        let encoded = encoder.encode_sequence(&events);
        assert_eq!(encoded.len(), 2);
        assert_eq!(encoded[0].len(), 4);
    }

    #[test]
    fn test_temporal_attention() {
        let encoder = TemporalEncoder::default();
        let query = vec![0.1; 64];

        let now = Utc::now();
        let events = vec![
            TemporalEvent::new("e1", "action", vec![0.1; 64]).with_timestamp(now),
            TemporalEvent::new("e2", "action", vec![0.2; 64])
                .with_timestamp(now - Duration::hours(2)),
        ];

        let content_scores = vec![1.0, 1.0];
        let weights = encoder.temporal_attention(&query, &events, &content_scores);

        assert_eq!(weights.len(), 2);
        // Recent event should have higher weight
        assert!(weights[0] > weights[1]);
        // Weights should sum to ~1
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_temporal_event_creation() {
        let event = TemporalEvent::new("test", "action", vec![1.0, 2.0]);
        assert_eq!(event.id, "test");
        assert_eq!(event.event_type, "action");
        assert_eq!(event.embedding.len(), 2);
    }
}

//! Non-IID data handling for federated learning.
//!
//! Handles heterogeneous data distributions across agents.

use crate::federated::learner::LocalTrainingData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data distribution statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataDistribution {
    /// Number of samples
    pub sample_count: usize,
    /// Mean of labels
    pub label_mean: f32,
    /// Variance of labels
    pub label_variance: f32,
    /// Feature means
    pub feature_means: Vec<f32>,
    /// Feature variances
    pub feature_variances: Vec<f32>,
    /// Label histogram (binned)
    pub label_histogram: HashMap<i32, usize>,
    /// Skewness score (0 = perfectly balanced)
    pub skewness: f32,
}

impl DataDistribution {
    /// Create an empty distribution.
    pub fn empty() -> Self {
        Self {
            sample_count: 0,
            label_mean: 0.0,
            label_variance: 0.0,
            feature_means: Vec::new(),
            feature_variances: Vec::new(),
            label_histogram: HashMap::new(),
            skewness: 0.0,
        }
    }

    /// Check if distribution is IID-like.
    pub fn is_iid_like(&self) -> bool {
        // Consider IID if skewness is low
        self.skewness < 0.3
    }
}

/// Handler for non-IID data scenarios.
pub struct NonIIDHandler {
    /// Whether to use adaptive learning rate
    pub adaptive_learning_rate: bool,
    /// Base learning rate
    base_lr: f32,
    /// Historical distributions for drift detection
    history: Vec<DataDistribution>,
}

impl NonIIDHandler {
    /// Create a new non-IID handler.
    pub fn new() -> Self {
        Self {
            adaptive_learning_rate: true,
            base_lr: 0.01,
            history: Vec::new(),
        }
    }

    /// Set base learning rate.
    pub fn with_base_lr(mut self, lr: f32) -> Self {
        self.base_lr = lr;
        self
    }

    /// Analyze local data distribution.
    pub fn analyze_distribution(&self, data: &[LocalTrainingData]) -> DataDistribution {
        if data.is_empty() {
            return DataDistribution::empty();
        }

        let n = data.len();

        // Compute label statistics
        let label_mean: f32 = data.iter().map(|d| d.label).sum::<f32>() / n as f32;
        let label_variance: f32 = data
            .iter()
            .map(|d| (d.label - label_mean).powi(2))
            .sum::<f32>()
            / n as f32;

        // Compute feature statistics
        let feature_dim = data.first().map(|d| d.features.len()).unwrap_or(0);
        let mut feature_means = vec![0.0; feature_dim];
        let mut feature_variances = vec![0.0; feature_dim];

        for d in data {
            for (i, &f) in d.features.iter().enumerate() {
                feature_means[i] += f;
            }
        }
        for mean in &mut feature_means {
            *mean /= n as f32;
        }

        for d in data {
            for (i, &f) in d.features.iter().enumerate() {
                feature_variances[i] += (f - feature_means[i]).powi(2);
            }
        }
        for var in &mut feature_variances {
            *var /= n as f32;
        }

        // Compute label histogram
        let mut label_histogram = HashMap::new();
        for d in data {
            let bin = (d.label * 10.0) as i32; // Bin to 0.1 precision
            *label_histogram.entry(bin).or_insert(0) += 1;
        }

        // Compute skewness (using simplified metric based on histogram entropy)
        let max_count = label_histogram.values().max().cloned().unwrap_or(1) as f32;
        let min_count = label_histogram.values().min().cloned().unwrap_or(0) as f32;
        let skewness = if max_count > 0.0 {
            (max_count - min_count) / max_count
        } else {
            0.0
        };

        DataDistribution {
            sample_count: n,
            label_mean,
            label_variance,
            feature_means,
            feature_variances,
            label_histogram,
            skewness,
        }
    }

    /// Compute adaptive learning rate based on distribution.
    pub fn adaptive_lr(&self, distribution: &DataDistribution) -> f32 {
        if !self.adaptive_learning_rate {
            return self.base_lr;
        }

        // Reduce learning rate for highly skewed data
        let skew_factor = 1.0 - (distribution.skewness * 0.5).min(0.8);

        // Reduce learning rate for small sample sizes
        let size_factor = (distribution.sample_count as f32 / 100.0).min(1.0).max(0.2);

        self.base_lr * skew_factor * size_factor
    }

    /// Detect if data drift has occurred.
    pub fn detect_drift(&mut self, current: &DataDistribution) -> f32 {
        if self.history.is_empty() {
            self.history.push(current.clone());
            return 0.0;
        }

        let last = self.history.last().unwrap();

        // Compute drift score based on distribution differences
        let mean_drift = (current.label_mean - last.label_mean).abs();
        let var_drift = (current.label_variance - last.label_variance).abs();
        let skew_drift = (current.skewness - last.skewness).abs();

        // Feature drift
        let feature_drift: f32 = current
            .feature_means
            .iter()
            .zip(last.feature_means.iter())
            .map(|(c, l)| (c - l).abs())
            .sum::<f32>()
            / current.feature_means.len().max(1) as f32;

        let drift_score = (mean_drift + var_drift + skew_drift + feature_drift) / 4.0;

        // Store current distribution
        self.history.push(current.clone());
        if self.history.len() > 10 {
            self.history.remove(0);
        }

        drift_score.min(1.0)
    }

    /// Get drift threshold recommendation for retraining.
    pub fn should_retrain(&self, drift_score: f32) -> bool {
        drift_score > 0.3
    }

    /// Suggest data mixing strategy for non-IID data.
    pub fn suggest_strategy(&self, distribution: &DataDistribution) -> NonIIDStrategy {
        if distribution.is_iid_like() {
            NonIIDStrategy::Standard
        } else if distribution.skewness > 0.7 {
            NonIIDStrategy::Oversampling
        } else if distribution.sample_count < 50 {
            NonIIDStrategy::DataAugmentation
        } else {
            NonIIDStrategy::WeightedAggregation
        }
    }
}

impl Default for NonIIDHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Strategies for handling non-IID data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NonIIDStrategy {
    /// Standard training (data is IID-like)
    Standard,
    /// Oversample minority classes
    Oversampling,
    /// Apply data augmentation
    DataAugmentation,
    /// Use weighted aggregation based on data quality
    WeightedAggregation,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> Vec<LocalTrainingData> {
        vec![
            LocalTrainingData::new(vec![1.0, 2.0], 0.0),
            LocalTrainingData::new(vec![2.0, 3.0], 0.1),
            LocalTrainingData::new(vec![3.0, 4.0], 0.2),
            LocalTrainingData::new(vec![4.0, 5.0], 0.3),
            LocalTrainingData::new(vec![5.0, 6.0], 0.4),
        ]
    }

    fn create_skewed_data() -> Vec<LocalTrainingData> {
        vec![
            LocalTrainingData::new(vec![1.0, 2.0], 0.0),
            LocalTrainingData::new(vec![2.0, 3.0], 0.0),
            LocalTrainingData::new(vec![3.0, 4.0], 0.0),
            LocalTrainingData::new(vec![4.0, 5.0], 0.0),
            LocalTrainingData::new(vec![5.0, 6.0], 1.0),
        ]
    }

    #[test]
    fn test_analyze_distribution() {
        let handler = NonIIDHandler::new();
        let data = create_test_data();
        let dist = handler.analyze_distribution(&data);

        assert_eq!(dist.sample_count, 5);
        assert!((dist.label_mean - 0.2).abs() < 1e-5);
        assert_eq!(dist.feature_means.len(), 2);
    }

    #[test]
    fn test_empty_data() {
        let handler = NonIIDHandler::new();
        let dist = handler.analyze_distribution(&[]);

        assert_eq!(dist.sample_count, 0);
        assert!(dist.is_iid_like()); // Empty is considered IID-like
    }

    #[test]
    fn test_skewed_distribution() {
        let handler = NonIIDHandler::new();
        let data = create_skewed_data();
        let dist = handler.analyze_distribution(&data);

        // Skewed data should have high skewness
        assert!(dist.skewness > 0.5);
    }

    #[test]
    fn test_adaptive_learning_rate() {
        let handler = NonIIDHandler::new().with_base_lr(0.1);
        let balanced_dist = DataDistribution {
            sample_count: 100,
            skewness: 0.1,
            ..DataDistribution::empty()
        };

        let skewed_dist = DataDistribution {
            sample_count: 100,
            skewness: 0.8,
            ..DataDistribution::empty()
        };

        let lr_balanced = handler.adaptive_lr(&balanced_dist);
        let lr_skewed = handler.adaptive_lr(&skewed_dist);

        // Skewed data should have lower learning rate
        assert!(lr_balanced > lr_skewed);
    }

    #[test]
    fn test_drift_detection() {
        let mut handler = NonIIDHandler::new();

        let dist1 = DataDistribution {
            sample_count: 100,
            label_mean: 0.5,
            label_variance: 0.1,
            skewness: 0.2,
            feature_means: vec![1.0, 2.0],
            ..DataDistribution::empty()
        };

        let dist2 = DataDistribution {
            sample_count: 100,
            label_mean: 0.8, // Significant drift in mean
            label_variance: 0.3,
            skewness: 0.5,
            feature_means: vec![1.5, 2.5],
            ..DataDistribution::empty()
        };

        let drift1 = handler.detect_drift(&dist1);
        assert_eq!(drift1, 0.0); // First distribution, no drift

        let drift2 = handler.detect_drift(&dist2);
        assert!(drift2 > 0.1); // Should detect drift
    }

    #[test]
    fn test_suggest_strategy() {
        let handler = NonIIDHandler::new();

        let iid_dist = DataDistribution {
            skewness: 0.1,
            sample_count: 100,
            ..DataDistribution::empty()
        };
        assert_eq!(handler.suggest_strategy(&iid_dist), NonIIDStrategy::Standard);

        let skewed_dist = DataDistribution {
            skewness: 0.8,
            sample_count: 100,
            ..DataDistribution::empty()
        };
        assert_eq!(
            handler.suggest_strategy(&skewed_dist),
            NonIIDStrategy::Oversampling
        );

        let small_dist = DataDistribution {
            skewness: 0.4,
            sample_count: 20,
            ..DataDistribution::empty()
        };
        assert_eq!(
            handler.suggest_strategy(&small_dist),
            NonIIDStrategy::DataAugmentation
        );
    }

    #[test]
    fn test_should_retrain() {
        let handler = NonIIDHandler::new();

        assert!(!handler.should_retrain(0.1));
        assert!(!handler.should_retrain(0.29));
        assert!(handler.should_retrain(0.31));
        assert!(handler.should_retrain(0.8));
    }
}

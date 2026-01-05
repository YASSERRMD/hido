//! Local learner for on-device training.
//!
//! Trains models locally without sharing raw data.

use crate::core::{now, Result, Timestamp};
use crate::uail::DIDKey;
use serde::{Deserialize, Serialize};

/// Compute device for training.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeDevice {
    CPU,
    GPU,
}

impl Default for ComputeDevice {
    fn default() -> Self {
        Self::CPU
    }
}

/// Local model with parameters and gradients.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalModel {
    /// Model parameters
    pub parameters: Vec<f32>,
    /// Computed gradients
    pub gradients: Vec<f32>,
    /// Model version
    pub model_version: u32,
    /// Number of parameters
    pub param_count: usize,
}

impl LocalModel {
    /// Create a new local model with random initialization.
    pub fn new(param_count: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let scale = (2.0 / param_count as f32).sqrt();

        let parameters: Vec<f32> = (0..param_count)
            .map(|_| rng.gen::<f32>() * scale - scale / 2.0)
            .collect();

        Self {
            parameters,
            gradients: vec![0.0; param_count],
            model_version: 1,
            param_count,
        }
    }

    /// Create from existing parameters.
    pub fn from_params(parameters: Vec<f32>) -> Self {
        let param_count = parameters.len();
        Self {
            parameters,
            gradients: vec![0.0; param_count],
            model_version: 1,
            param_count,
        }
    }

    /// Reset gradients to zero.
    pub fn zero_gradients(&mut self) {
        self.gradients.fill(0.0);
    }

    /// Apply gradients with learning rate.
    pub fn apply_gradients(&mut self, learning_rate: f32) {
        for (param, grad) in self.parameters.iter_mut().zip(self.gradients.iter()) {
            *param -= learning_rate * grad;
        }
        self.model_version += 1;
    }

    /// Forward pass (simple linear model for demonstration).
    pub fn forward(&self, input: &[f32]) -> f32 {
        self.parameters
            .iter()
            .zip(input.iter())
            .map(|(w, x)| w * x)
            .sum::<f32>()
            + self.parameters.last().cloned().unwrap_or(0.0) // bias
    }

    /// Compute loss (MSE) and gradients.
    pub fn backward(&mut self, input: &[f32], target: f32) -> f32 {
        let prediction = self.forward(input);
        let error = prediction - target;
        let loss = error * error;

        // Compute gradients (for simple linear model)
        for (i, grad) in self.gradients.iter_mut().enumerate() {
            if i < input.len() {
                *grad += 2.0 * error * input[i];
            } else {
                *grad += 2.0 * error; // bias gradient
            }
        }

        loss
    }
}

/// Training data sample.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalTrainingData {
    /// Input features
    pub features: Vec<f32>,
    /// Target label
    pub label: f32,
    /// Data timestamp
    pub timestamp: Timestamp,
}

impl LocalTrainingData {
    /// Create a new training sample.
    pub fn new(features: Vec<f32>, label: f32) -> Self {
        Self {
            features,
            label,
            timestamp: now(),
        }
    }
}

/// Result of local training.
#[derive(Clone, Debug)]
pub struct TrainingResult {
    /// Average loss
    pub loss: f32,
    /// Number of samples trained on
    pub samples_trained: usize,
    /// Epochs completed
    pub epochs: usize,
    /// Final model version
    pub model_version: u32,
}

/// Local learner for federated training.
pub struct LocalLearner {
    /// Agent ID
    pub agent_id: String,
    /// Local model
    pub model: LocalModel,
    /// Training data
    pub training_data: Vec<LocalTrainingData>,
    /// Compute device
    pub device: ComputeDevice,
    /// Learning rate
    learning_rate: f32,
    /// Batch size
    batch_size: usize,
}

impl LocalLearner {
    /// Create a new local learner.
    pub fn new(agent: &DIDKey, model: LocalModel) -> Self {
        Self {
            agent_id: agent.id.clone(),
            model,
            training_data: Vec::new(),
            device: ComputeDevice::CPU,
            learning_rate: 0.01,
            batch_size: 32,
        }
    }

    /// Set learning rate.
    pub fn with_learning_rate(mut self, lr: f32) -> Self {
        self.learning_rate = lr;
        self
    }

    /// Set batch size.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Add training data.
    pub fn add_data(&mut self, data: LocalTrainingData) {
        self.training_data.push(data);
    }

    /// Add batch of training data.
    pub fn add_data_batch(&mut self, data: Vec<LocalTrainingData>) {
        self.training_data.extend(data);
    }

    /// Train locally on data.
    /// REQUIREMENT: No raw data leaves this agent.
    pub async fn train_locally(&mut self, epochs: usize) -> Result<TrainingResult> {
        if self.training_data.is_empty() {
            return Ok(TrainingResult {
                loss: 0.0,
                samples_trained: 0,
                epochs: 0,
                model_version: self.model.model_version,
            });
        }

        let mut total_loss = 0.0;
        let samples = self.training_data.len();

        for _epoch in 0..epochs {
            self.model.zero_gradients();
            let mut epoch_loss = 0.0;

            // Process all data (in batches conceptually)
            for data in &self.training_data {
                let loss = self.model.backward(&data.features, data.label);
                epoch_loss += loss;
            }

            // Average gradients
            let sample_count = samples as f32;
            for grad in &mut self.model.gradients {
                *grad /= sample_count;
            }

            // Apply gradients
            self.model.apply_gradients(self.learning_rate);

            total_loss = epoch_loss / sample_count;
        }

        Ok(TrainingResult {
            loss: total_loss,
            samples_trained: samples,
            epochs,
            model_version: self.model.model_version,
        })
    }

    /// Compute gradients without applying them.
    pub fn compute_gradients(&mut self) -> Result<Vec<f32>> {
        self.model.zero_gradients();

        for data in &self.training_data {
            self.model.backward(&data.features, data.label);
        }

        // Average gradients
        let sample_count = self.training_data.len() as f32;
        if sample_count > 0.0 {
            for grad in &mut self.model.gradients {
                *grad /= sample_count;
            }
        }

        Ok(self.model.gradients.clone())
    }

    /// Apply global update from aggregator.
    pub fn apply_global_update(&mut self, global_params: Vec<f32>) -> Result<()> {
        if global_params.len() != self.model.param_count {
            return Err(crate::core::Error::Internal(
                "Parameter count mismatch".to_string(),
            ));
        }

        self.model.parameters = global_params;
        self.model.model_version += 1;
        Ok(())
    }

    /// Get current model parameters.
    pub fn get_parameters(&self) -> &[f32] {
        &self.model.parameters
    }

    /// Get data sample count.
    pub fn data_count(&self) -> usize {
        self.training_data.len()
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
    fn test_local_model_creation() {
        let model = LocalModel::new(100);
        assert_eq!(model.param_count, 100);
        assert_eq!(model.parameters.len(), 100);
        assert_eq!(model.gradients.len(), 100);
    }

    #[test]
    fn test_model_forward() {
        let model = LocalModel::from_params(vec![1.0, 2.0, 0.5]);
        let output = model.forward(&[1.0, 1.0]);
        // 1.0*1.0 + 2.0*1.0 + 0.5 (bias) = 3.5
        assert!((output - 3.5).abs() < 1e-5);
    }

    #[test]
    fn test_model_backward() {
        let mut model = LocalModel::from_params(vec![1.0, 1.0, 0.0]);
        let loss = model.backward(&[1.0, 1.0], 2.0);
        // Prediction: 1.0 + 1.0 + 0.0 = 2.0, target = 2.0, error = 0
        assert!(loss < 1e-5);
    }

    #[test]
    fn test_local_learner_creation() {
        let agent = create_test_did();
        let model = LocalModel::new(10);
        let learner = LocalLearner::new(&agent, model);
        assert_eq!(learner.data_count(), 0);
    }

    #[test]
    fn test_add_training_data() {
        let agent = create_test_did();
        let model = LocalModel::new(3);
        let mut learner = LocalLearner::new(&agent, model);

        learner.add_data(LocalTrainingData::new(vec![1.0, 2.0], 3.0));
        learner.add_data(LocalTrainingData::new(vec![2.0, 3.0], 5.0));

        assert_eq!(learner.data_count(), 2);
    }

    #[tokio::test]
    async fn test_train_locally() {
        let agent = create_test_did();
        let model = LocalModel::new(3);
        let mut learner = LocalLearner::new(&agent, model).with_learning_rate(0.001);

        learner.add_data(LocalTrainingData::new(vec![1.0, 2.0], 5.0));
        learner.add_data(LocalTrainingData::new(vec![2.0, 3.0], 8.0));

        let result = learner.train_locally(10).await.unwrap();
        assert_eq!(result.epochs, 10);
        assert_eq!(result.samples_trained, 2);
    }

    #[test]
    fn test_compute_gradients() {
        let agent = create_test_did();
        let model = LocalModel::new(3);
        let mut learner = LocalLearner::new(&agent, model);

        learner.add_data(LocalTrainingData::new(vec![1.0, 1.0], 2.0));

        let gradients = learner.compute_gradients().unwrap();
        assert_eq!(gradients.len(), 3);
    }

    #[test]
    fn test_apply_global_update() {
        let agent = create_test_did();
        let model = LocalModel::new(3);
        let mut learner = LocalLearner::new(&agent, model);

        let new_params = vec![0.5, 0.5, 0.1];
        learner.apply_global_update(new_params.clone()).unwrap();

        assert_eq!(learner.get_parameters(), &new_params[..]);
    }
}

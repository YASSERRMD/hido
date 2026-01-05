//! Decision engine orchestrating consensus and guardrails.
//!
//! Combines Byzantine voting, ethical guardrails, and GNN predictions.

use crate::consensus::guardrails::{EthicalEvaluation, EthicalGuardrail};
use crate::consensus::voting::{ByzantineVoting, ConsensusResult, Vote, VoteType, VotingConfig};
use crate::core::Result;
use crate::gnn::learner::{GNNLearner, PredictedDecision};
use crate::icc::intent::SemanticIntent;
use crate::uail::DIDKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A decision made by the engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    /// Decision ID
    pub id: String,
    /// Selected agent
    pub selected_agent: String,
    /// Decision type (Approve/Reject)
    pub decision_type: VoteType,
    /// Confidence score
    pub confidence: f32,
    /// Timestamp
    pub timestamp: crate::core::Timestamp,
    /// Whether human approval is required
    pub requires_human_approval: bool,
}

/// Explanation of how a decision was made.
#[derive(Clone, Debug)]
pub struct DecisionExplanation {
    /// The decision
    pub decision: Decision,
    /// GNN recommendation
    pub gnn_recommendation: Option<PredictedDecision>,
    /// Voting result
    pub voting_result: ConsensusResult,
    /// Guardrail evaluation
    pub guardrail_check: EthicalEvaluation,
    /// Human-readable reasoning
    pub reasoning: String,
    /// Factors that influenced the decision
    pub factors: HashMap<String, f32>,
}

/// Decision engine metrics.
#[derive(Clone, Debug, Default)]
pub struct DecisionMetrics {
    pub total_decisions: u64,
    pub approved_decisions: u64,
    pub rejected_decisions: u64,
    pub escalated_decisions: u64,
    pub average_confidence: f64,
}

/// Configuration for the decision engine.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Weight for GNN recommendations (0-1)
    pub gnn_weight: f32,
    /// Weight for voting (0-1)
    pub voting_weight: f32,
    /// Minimum confidence threshold
    pub min_confidence: f32,
    /// Voting configuration
    pub voting_config: VotingConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            gnn_weight: 0.3,
            voting_weight: 0.7,
            min_confidence: 0.5,
            voting_config: VotingConfig::default(),
        }
    }
}

/// Decision engine orchestrating all components.
pub struct DecisionEngine {
    /// Byzantine voting system
    pub voting: ByzantineVoting,
    /// Ethical guardrails
    pub guardrails: EthicalGuardrail,
    /// GNN learner (optional)
    gnn: Option<GNNLearner>,
    /// Metrics
    pub metrics: DecisionMetrics,
    /// Configuration
    config: EngineConfig,
}

impl DecisionEngine {
    /// Create a new decision engine.
    pub fn new(config: EngineConfig) -> Self {
        Self {
            voting: ByzantineVoting::new(config.voting_config.clone()),
            guardrails: EthicalGuardrail::with_defaults(),
            gnn: None,
            metrics: DecisionMetrics::default(),
            config,
        }
    }

    /// Create with GNN learner.
    pub fn with_gnn(mut self, gnn: GNNLearner) -> Self {
        self.gnn = Some(gnn);
        self
    }

    /// Register an agent as a voter.
    pub fn register_agent(&mut self, agent: &DIDKey, weight: f32) {
        self.voting.register_voter(agent, weight);
    }

    /// Make an autonomous decision.
    pub async fn make_decision(
        &mut self,
        intent: &SemanticIntent,
        candidates: Vec<DIDKey>,
        votes: Vec<Vote>,
    ) -> Result<(Decision, DecisionExplanation)> {
        let decision_id = uuid::Uuid::new_v4().to_string();

        // 1. Start voting
        self.voting.start_vote(&intent.id);
        for vote in &votes {
            let _ = self.voting.cast_vote(vote.clone());
        }
        let voting_result = self.voting.tally();

        // 2. Get GNN recommendation (if available)
        let gnn_recommendation = if self.gnn.is_some() {
            // In a real implementation, we would pass proper embeddings
            let mut embeddings = HashMap::new();
            embeddings.insert(intent.sender.clone(), vec![0.1; 64]);
            for c in &candidates {
                embeddings.insert(c.id.clone(), vec![0.2; 64]);
            }

            let edges: Vec<(String, String)> = candidates
                .iter()
                .map(|c| (intent.sender.clone(), c.id.clone()))
                .collect();

            if let Some(gnn) = &self.gnn {
                let candidate_ids: Vec<String> = candidates.iter().map(|c| c.id.clone()).collect();
                gnn.predict(&embeddings, &edges, &intent.sender, &candidate_ids, &[])
                    .ok()
            } else {
                None
            }
        } else {
            None
        };

        // 3. Build context for guardrail check
        let mut context = HashMap::new();
        context.insert(
            "risk_score".to_string(),
            serde_json::json!(1.0 - voting_result.confidence),
        );
        context.insert(
            "has_explanation".to_string(),
            serde_json::json!(!intent.action.is_empty()),
        );
        context.insert("contains_pii".to_string(), serde_json::json!(false));
        context.insert(
            "impact".to_string(),
            serde_json::json!(match intent.priority {
                crate::icc::intent::IntentPriority::Critical => 1.0,
                crate::icc::intent::IntentPriority::High => 0.7,
                crate::icc::intent::IntentPriority::Normal => 0.5,
                crate::icc::intent::IntentPriority::Low => 0.3,
            }),
        );

        let guardrail_check = self.guardrails.evaluate(&context);

        // 4. Combine signals to make decision
        let (selected_agent, decision_type, confidence, requires_human_approval) =
            self.combine_signals(&voting_result, &gnn_recommendation, &guardrail_check, &candidates);

        let decision = Decision {
            id: decision_id,
            selected_agent: selected_agent.clone(),
            decision_type: decision_type.clone(),
            confidence,
            timestamp: crate::core::now(),
            requires_human_approval,
        };

        // 5. Build explanation
        let mut factors = HashMap::new();
        factors.insert("voting_confidence".to_string(), voting_result.confidence);
        if let Some(ref gnn_rec) = gnn_recommendation {
            factors.insert("gnn_confidence".to_string(), gnn_rec.confidence);
        }
        factors.insert("guardrail_risk".to_string(), guardrail_check.risk_score);

        let reasoning = self.generate_reasoning(&voting_result, &gnn_recommendation, &guardrail_check);

        let explanation = DecisionExplanation {
            decision: decision.clone(),
            gnn_recommendation,
            voting_result,
            guardrail_check,
            reasoning,
            factors,
        };

        // Update metrics
        self.update_metrics(&decision);

        Ok((decision, explanation))
    }

    fn combine_signals(
        &self,
        voting: &ConsensusResult,
        gnn: &Option<PredictedDecision>,
        guardrails: &EthicalEvaluation,
        candidates: &[DIDKey],
    ) -> (String, VoteType, f32, bool) {
        // If guardrails require rejection or escalation, honor that
        if let Some(ref action) = guardrails.required_action {
            match action {
                crate::consensus::guardrails::GuardrailAction::Reject => {
                    let agent = candidates.first().map(|c| c.id.clone()).unwrap_or_default();
                    return (agent, VoteType::Reject, guardrails.risk_score, false);
                }
                crate::consensus::guardrails::GuardrailAction::RequireApproval
                | crate::consensus::guardrails::GuardrailAction::Escalate => {
                    let agent = gnn
                        .as_ref()
                        .map(|g| g.agent_id.clone())
                        .or_else(|| candidates.first().map(|c| c.id.clone()))
                        .unwrap_or_default();
                    let decision_type = voting.decision.clone().unwrap_or(VoteType::Approve);
                    return (agent, decision_type, voting.confidence, true);
                }
                _ => {}
            }
        }

        // Combine voting and GNN signals
        let selected_agent = if let Some(ref gnn_rec) = gnn {
            if gnn_rec.confidence > 0.8 {
                gnn_rec.agent_id.clone()
            } else {
                candidates.first().map(|c| c.id.clone()).unwrap_or_default()
            }
        } else {
            candidates.first().map(|c| c.id.clone()).unwrap_or_default()
        };

        let decision_type = voting.decision.clone().unwrap_or(VoteType::Approve);

        let gnn_confidence = gnn.as_ref().map(|g| g.confidence).unwrap_or(0.5);
        let combined_confidence = self.config.voting_weight * voting.confidence
            + self.config.gnn_weight * gnn_confidence;

        let requires_human = combined_confidence < self.config.min_confidence;

        (selected_agent, decision_type, combined_confidence, requires_human)
    }

    fn generate_reasoning(
        &self,
        voting: &ConsensusResult,
        gnn: &Option<PredictedDecision>,
        guardrails: &EthicalEvaluation,
    ) -> String {
        let mut parts = Vec::new();

        if voting.consensus_reached {
            parts.push(format!(
                "Voting: {} with {:.0}% approval ({}/{} votes)",
                if voting.decision == Some(VoteType::Approve) {
                    "APPROVED"
                } else {
                    "REJECTED"
                },
                voting.confidence * 100.0,
                voting.approve_votes,
                voting.total_voters
            ));
        } else {
            parts.push(format!(
                "Voting: No consensus (quorum not met, {}/{} voted)",
                voting.approve_votes + voting.reject_votes,
                voting.total_voters
            ));
        }

        if let Some(ref gnn_rec) = gnn {
            parts.push(format!(
                "GNN: Recommends {} with {:.0}% confidence",
                gnn_rec.agent_id,
                gnn_rec.confidence * 100.0
            ));
        }

        if !guardrails.passes {
            parts.push(format!(
                "Guardrails: {} violations detected (risk: {:.0}%)",
                guardrails.violations.len(),
                guardrails.risk_score * 100.0
            ));
        } else {
            parts.push("Guardrails: All checks passed".to_string());
        }

        parts.join(". ")
    }

    fn update_metrics(&mut self, decision: &Decision) {
        self.metrics.total_decisions += 1;

        match decision.decision_type {
            VoteType::Approve => self.metrics.approved_decisions += 1,
            VoteType::Reject => self.metrics.rejected_decisions += 1,
            VoteType::Abstain => {}
        }

        if decision.requires_human_approval {
            self.metrics.escalated_decisions += 1;
        }

        self.metrics.average_confidence = (self.metrics.average_confidence
            * (self.metrics.total_decisions - 1) as f64
            + decision.confidence as f64)
            / self.metrics.total_decisions as f64;
    }

    /// Get engine metrics.
    pub fn metrics(&self) -> &DecisionMetrics {
        &self.metrics
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uail::crypto::CryptoSuite;
    use crate::icc::intent::IntentDomain;

    fn create_test_did() -> DIDKey {
        let crypto = CryptoSuite::new();
        DIDKey::new(&crypto)
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = DecisionEngine::default();
        assert_eq!(engine.metrics.total_decisions, 0);
    }

    #[tokio::test]
    async fn test_make_decision() {
        let mut engine = DecisionEngine::default();

        let voter1 = create_test_did();
        let voter2 = create_test_did();
        let candidate = create_test_did();

        engine.register_agent(&voter1, 1.0);
        engine.register_agent(&voter2, 1.0);

        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Coordination, "delegate");

        let votes = vec![
            Vote::new(&voter1, VoteType::Approve),
            Vote::new(&voter2, VoteType::Approve),
        ];

        let (decision, explanation) = engine
            .make_decision(&intent, vec![candidate], votes)
            .await
            .unwrap();

        assert!(!decision.id.is_empty());
        assert!(!explanation.reasoning.is_empty());
    }

    #[tokio::test]
    async fn test_decision_with_guardrail_rejection() {
        let mut engine = DecisionEngine::default();

        let voter = create_test_did();
        let candidate = create_test_did();
        engine.register_agent(&voter, 1.0);

        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Coordination, "high-risk")
            .with_priority(crate::icc::intent::IntentPriority::Critical);

        // Add a guardrail that will trigger
        engine.guardrails.add_rule(
            crate::consensus::guardrails::GuardrailRule::new(
                "test-reject",
                crate::consensus::guardrails::RuleType::Safety,
                crate::consensus::guardrails::GuardrailAction::Reject,
                "Test rejection",
            )
            .with_condition(crate::consensus::guardrails::Condition::new(
                "impact",
                "eq",
                serde_json::json!(1.0),
            ))
            .with_severity(10),
        );

        let votes = vec![Vote::new(&voter, VoteType::Approve)];

        let (decision, _) = engine
            .make_decision(&intent, vec![candidate], votes)
            .await
            .unwrap();

        assert_eq!(decision.decision_type, VoteType::Reject);
    }

    #[tokio::test]
    async fn test_metrics_update() {
        let mut engine = DecisionEngine::default();

        let voter = create_test_did();
        let candidate = create_test_did();
        engine.register_agent(&voter, 1.0);

        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");
        let votes = vec![Vote::new(&voter, VoteType::Approve)];

        engine.make_decision(&intent, vec![candidate.clone()], votes.clone()).await.unwrap();
        engine.make_decision(&intent, vec![candidate], votes).await.unwrap();

        assert_eq!(engine.metrics.total_decisions, 2);
    }
}

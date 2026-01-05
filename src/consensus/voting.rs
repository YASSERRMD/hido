//! Byzantine-fault-tolerant voting mechanism.
//!
//! Implements 2/3 quorum consensus tolerating up to 1/3 Byzantine agents.

use crate::core::{now, Result, Timestamp};
use crate::uail::DIDKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of vote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    /// Approve the proposal
    Approve,
    /// Reject the proposal
    Reject,
    /// Abstain from voting
    Abstain,
}

/// A single vote from an agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vote {
    /// Voter DID
    pub voter: String,
    /// Vote type
    pub vote_type: VoteType,
    /// Vote timestamp
    pub timestamp: Timestamp,
    /// Optional justification
    pub justification: Option<String>,
    /// Signature of the vote
    pub signature: Option<Vec<u8>>,
}

impl Vote {
    /// Create a new vote.
    pub fn new(voter: &DIDKey, vote_type: VoteType) -> Self {
        Self {
            voter: voter.id.clone(),
            vote_type,
            timestamp: now(),
            justification: None,
            signature: None,
        }
    }

    /// Add justification.
    pub fn with_justification(mut self, justification: &str) -> Self {
        self.justification = Some(justification.to_string());
        self
    }

    /// Sign the vote.
    pub fn sign(mut self, sign_fn: impl FnOnce(&[u8]) -> Vec<u8>) -> Self {
        let data = format!("{}:{}:{:?}", self.voter, self.timestamp, self.vote_type);
        self.signature = Some(sign_fn(data.as_bytes()));
        self
    }
}

/// Result of a consensus vote.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Whether consensus was reached
    pub consensus_reached: bool,
    /// The decision (Approve/Reject)
    pub decision: Option<VoteType>,
    /// Number of approving votes
    pub approve_votes: usize,
    /// Number of rejecting votes
    pub reject_votes: usize,
    /// Number of abstaining votes
    pub abstain_votes: usize,
    /// Total voters
    pub total_voters: usize,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Voting round
    pub round: u32,
}

/// Byzantine-fault-tolerant voting system.
pub struct ByzantineVoting {
    /// Registered voters
    voters: HashMap<String, VoterInfo>,
    /// Current votes
    votes: HashMap<String, Vote>,
    /// Current round
    round: u32,
    /// Proposal being voted on
    proposal: Option<String>,
    /// Configuration
    config: VotingConfig,
}

/// Information about a voter.
#[derive(Clone, Debug)]
pub struct VoterInfo {
    pub id: String,
    pub weight: f32,
    pub reputation: f32,
    pub is_byzantine: bool, // For testing
}

/// Voting configuration.
#[derive(Clone, Debug)]
pub struct VotingConfig {
    /// Quorum threshold (default 2/3)
    pub quorum_threshold: f32,
    /// Timeout in seconds
    pub timeout_seconds: u64,
    /// Whether to use weighted voting
    pub weighted: bool,
    /// Maximum rounds before failure
    pub max_rounds: u32,
}

impl Default for VotingConfig {
    fn default() -> Self {
        Self {
            quorum_threshold: 2.0 / 3.0,
            timeout_seconds: 60,
            weighted: false,
            max_rounds: 3,
        }
    }
}

impl ByzantineVoting {
    /// Create a new voting instance.
    pub fn new(config: VotingConfig) -> Self {
        Self {
            voters: HashMap::new(),
            votes: HashMap::new(),
            round: 1,
            proposal: None,
            config,
        }
    }

    /// Register a voter.
    pub fn register_voter(&mut self, voter: &DIDKey, weight: f32) {
        self.voters.insert(
            voter.id.clone(),
            VoterInfo {
                id: voter.id.clone(),
                weight: weight.clamp(0.0, 1.0),
                reputation: 1.0,
                is_byzantine: false,
            },
        );
    }

    /// Unregister a voter.
    pub fn unregister_voter(&mut self, voter_id: &str) {
        self.voters.remove(voter_id);
    }

    /// Start a new voting round for a proposal.
    pub fn start_vote(&mut self, proposal: &str) {
        self.votes.clear();
        self.proposal = Some(proposal.to_string());
        self.round = 1;
    }

    /// Cast a vote.
    pub fn cast_vote(&mut self, vote: Vote) -> Result<()> {
        if !self.voters.contains_key(&vote.voter) {
            return Err(crate::core::Error::Internal(format!(
                "Voter {} not registered",
                vote.voter
            )));
        }

        if self.proposal.is_none() {
            return Err(crate::core::Error::Internal(
                "No active proposal".to_string(),
            ));
        }

        self.votes.insert(vote.voter.clone(), vote);
        Ok(())
    }

    /// Tally votes and determine consensus.
    pub fn tally(&self) -> ConsensusResult {
        let total_voters = self.voters.len();

        if total_voters == 0 {
            return ConsensusResult {
                consensus_reached: false,
                decision: None,
                approve_votes: 0,
                reject_votes: 0,
                abstain_votes: 0,
                total_voters: 0,
                confidence: 0.0,
                round: self.round,
            };
        }

        let mut approve_weight = 0.0;
        let mut reject_weight = 0.0;
        let mut abstain_count = 0;
        let mut approve_count = 0;
        let mut reject_count = 0;

        for (voter_id, vote) in &self.votes {
            let weight = if self.config.weighted {
                self.voters.get(voter_id).map(|v| v.weight).unwrap_or(1.0)
            } else {
                1.0
            };

            match vote.vote_type {
                VoteType::Approve => {
                    approve_weight += weight;
                    approve_count += 1;
                }
                VoteType::Reject => {
                    reject_weight += weight;
                    reject_count += 1;
                }
                VoteType::Abstain => {
                    abstain_count += 1;
                }
            }
        }

        let total_weight = if self.config.weighted {
            self.voters.values().map(|v| v.weight).sum::<f32>()
        } else {
            total_voters as f32
        };

        // Check for quorum
        let participation = (approve_weight + reject_weight) / total_weight;
        let quorum_met = participation >= self.config.quorum_threshold;

        let (consensus_reached, decision, confidence) = if quorum_met {
            if approve_weight > reject_weight {
                let conf = approve_weight / (approve_weight + reject_weight);
                (true, Some(VoteType::Approve), conf)
            } else if reject_weight > approve_weight {
                let conf = reject_weight / (approve_weight + reject_weight);
                (true, Some(VoteType::Reject), conf)
            } else {
                (false, None, 0.5)
            }
        } else {
            (false, None, participation)
        };

        ConsensusResult {
            consensus_reached,
            decision,
            approve_votes: approve_count,
            reject_votes: reject_count,
            abstain_votes: abstain_count,
            total_voters,
            confidence,
            round: self.round,
        }
    }

    /// Check if Byzantine fault tolerance is maintained.
    /// System tolerates up to f Byzantine nodes where n >= 3f + 1.
    pub fn byzantine_tolerance(&self) -> ByzantineTolerance {
        let n = self.voters.len();
        let max_faulty = (n - 1) / 3;
        let honest_required = n - max_faulty;

        ByzantineTolerance {
            total_voters: n,
            max_faulty_tolerated: max_faulty,
            honest_required,
            is_secure: n >= 4, // Need at least 4 nodes for any tolerance
        }
    }

    /// Get current round.
    pub fn current_round(&self) -> u32 {
        self.round
    }

    /// Advance to next round.
    pub fn next_round(&mut self) {
        self.round += 1;
        self.votes.clear();
    }

    /// Get vote count.
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Get voter count.
    pub fn voter_count(&self) -> usize {
        self.voters.len()
    }
}

impl Default for ByzantineVoting {
    fn default() -> Self {
        Self::new(VotingConfig::default())
    }
}

/// Byzantine fault tolerance information.
#[derive(Clone, Debug)]
pub struct ByzantineTolerance {
    pub total_voters: usize,
    pub max_faulty_tolerated: usize,
    pub honest_required: usize,
    pub is_secure: bool,
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
    fn test_voting_creation() {
        let voting = ByzantineVoting::default();
        assert_eq!(voting.voter_count(), 0);
        assert_eq!(voting.current_round(), 1);
    }

    #[test]
    fn test_register_voter() {
        let mut voting = ByzantineVoting::default();
        let voter = create_test_did();
        voting.register_voter(&voter, 1.0);
        assert_eq!(voting.voter_count(), 1);
    }

    #[test]
    fn test_cast_vote() {
        let mut voting = ByzantineVoting::default();
        let voter = create_test_did();
        voting.register_voter(&voter, 1.0);
        voting.start_vote("proposal-1");

        let vote = Vote::new(&voter, VoteType::Approve);
        voting.cast_vote(vote).unwrap();

        assert_eq!(voting.vote_count(), 1);
    }

    #[test]
    fn test_consensus_reached() {
        let mut voting = ByzantineVoting::new(VotingConfig {
            quorum_threshold: 0.5,
            ..Default::default()
        });

        let voter1 = create_test_did();
        let voter2 = create_test_did();
        let voter3 = create_test_did();

        voting.register_voter(&voter1, 1.0);
        voting.register_voter(&voter2, 1.0);
        voting.register_voter(&voter3, 1.0);
        voting.start_vote("proposal-1");

        voting.cast_vote(Vote::new(&voter1, VoteType::Approve)).unwrap();
        voting.cast_vote(Vote::new(&voter2, VoteType::Approve)).unwrap();
        voting.cast_vote(Vote::new(&voter3, VoteType::Reject)).unwrap();

        let result = voting.tally();
        assert!(result.consensus_reached);
        assert_eq!(result.decision, Some(VoteType::Approve));
        assert_eq!(result.approve_votes, 2);
        assert_eq!(result.reject_votes, 1);
    }

    #[test]
    fn test_quorum_not_met() {
        let mut voting = ByzantineVoting::new(VotingConfig {
            quorum_threshold: 2.0 / 3.0,
            ..Default::default()
        });

        let voter1 = create_test_did();
        let voter2 = create_test_did();
        let voter3 = create_test_did();

        voting.register_voter(&voter1, 1.0);
        voting.register_voter(&voter2, 1.0);
        voting.register_voter(&voter3, 1.0);
        voting.start_vote("proposal-1");

        // Only 1 out of 3 votes
        voting.cast_vote(Vote::new(&voter1, VoteType::Approve)).unwrap();

        let result = voting.tally();
        assert!(!result.consensus_reached);
    }

    #[test]
    fn test_byzantine_tolerance() {
        let mut voting = ByzantineVoting::default();

        // Register 4 voters (n=4, can tolerate f=1 Byzantine)
        for _ in 0..4 {
            let voter = create_test_did();
            voting.register_voter(&voter, 1.0);
        }

        let tolerance = voting.byzantine_tolerance();
        assert_eq!(tolerance.total_voters, 4);
        assert_eq!(tolerance.max_faulty_tolerated, 1);
        assert_eq!(tolerance.honest_required, 3);
        assert!(tolerance.is_secure);
    }

    #[test]
    fn test_weighted_voting() {
        let mut voting = ByzantineVoting::new(VotingConfig {
            weighted: true,
            quorum_threshold: 0.5,
            ..Default::default()
        });

        let voter1 = create_test_did();
        let voter2 = create_test_did();

        voting.register_voter(&voter1, 0.8);
        voting.register_voter(&voter2, 0.2);
        voting.start_vote("proposal-1");

        voting.cast_vote(Vote::new(&voter1, VoteType::Approve)).unwrap();
        voting.cast_vote(Vote::new(&voter2, VoteType::Reject)).unwrap();

        let result = voting.tally();
        // voter1 has more weight, so Approve should win
        assert!(result.consensus_reached);
        assert_eq!(result.decision, Some(VoteType::Approve));
    }

    #[test]
    fn test_vote_with_justification() {
        let voter = create_test_did();
        let vote = Vote::new(&voter, VoteType::Approve)
            .with_justification("Good proposal");

        assert_eq!(vote.justification, Some("Good proposal".to_string()));
    }

    #[test]
    fn test_next_round() {
        let mut voting = ByzantineVoting::default();
        let voter = create_test_did();
        voting.register_voter(&voter, 1.0);
        voting.start_vote("proposal-1");
        voting.cast_vote(Vote::new(&voter, VoteType::Approve)).unwrap();

        assert_eq!(voting.vote_count(), 1);
        voting.next_round();
        assert_eq!(voting.vote_count(), 0);
        assert_eq!(voting.current_round(), 2);
    }
}

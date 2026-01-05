//! Intent router for capability-based routing.
//!
//! Routes intents to agents based on their capabilities.

use crate::core::Result;
use crate::icc::intent::{IntentDomain, SemanticIntent};
use crate::uail::DIDKey;
use std::collections::HashMap;

/// Agent capability descriptor.
#[derive(Clone, Debug)]
pub struct AgentCapability {
    /// Agent DID
    pub agent_id: String,
    /// Supported domains
    pub domains: Vec<IntentDomain>,
    /// Supported actions
    pub actions: Vec<String>,
    /// Capability score (0.0 - 1.0)
    pub score: f32,
    /// Whether agent is currently available
    pub available: bool,
    /// Load factor (0.0 = idle, 1.0 = fully loaded)
    pub load: f32,
}

impl AgentCapability {
    /// Create a new agent capability.
    pub fn new(agent: &DIDKey) -> Self {
        Self {
            agent_id: agent.id.clone(),
            domains: Vec::new(),
            actions: Vec::new(),
            score: 1.0,
            available: true,
            load: 0.0,
        }
    }

    /// Add a supported domain.
    pub fn with_domain(mut self, domain: IntentDomain) -> Self {
        self.domains.push(domain);
        self
    }

    /// Add a supported action.
    pub fn with_action(mut self, action: &str) -> Self {
        self.actions.push(action.to_string());
        self
    }

    /// Set capability score.
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score.clamp(0.0, 1.0);
        self
    }

    /// Check if agent supports a domain.
    pub fn supports_domain(&self, domain: &IntentDomain) -> bool {
        self.domains.contains(domain)
    }

    /// Check if agent supports an action.
    pub fn supports_action(&self, action: &str) -> bool {
        self.actions.contains(&action.to_string())
    }

    /// Compute match score for an intent.
    pub fn match_score(&self, intent: &SemanticIntent) -> f32 {
        if !self.available {
            return 0.0;
        }

        let domain_match = if self.supports_domain(&intent.domain) {
            1.0
        } else {
            0.0
        };

        let action_match = if self.supports_action(&intent.action) {
            1.0
        } else {
            0.5 // Partial match if domain matches but action doesn't
        };

        let load_factor = 1.0 - self.load;

        self.score * domain_match * action_match * load_factor
    }
}

/// Routing result.
#[derive(Clone, Debug)]
pub struct RouteResult {
    /// Selected agent ID
    pub agent_id: String,
    /// Match score
    pub score: f32,
    /// Alternative agents
    pub alternatives: Vec<String>,
}

/// Intent router with capability index.
pub struct IntentRouter {
    /// Registered agent capabilities
    capabilities: HashMap<String, AgentCapability>,
    /// Routing metrics
    metrics: RouterMetrics,
}

/// Router metrics for monitoring.
#[derive(Clone, Debug, Default)]
pub struct RouterMetrics {
    pub total_routes: u64,
    pub successful_routes: u64,
    pub failed_routes: u64,
    pub average_score: f64,
}

impl IntentRouter {
    /// Create a new intent router.
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            metrics: RouterMetrics::default(),
        }
    }

    /// Register an agent's capabilities.
    pub fn register(&mut self, capability: AgentCapability) {
        self.capabilities
            .insert(capability.agent_id.clone(), capability);
    }

    /// Unregister an agent.
    pub fn unregister(&mut self, agent_id: &str) {
        self.capabilities.remove(agent_id);
    }

    /// Update agent availability.
    pub fn set_availability(&mut self, agent_id: &str, available: bool) {
        if let Some(cap) = self.capabilities.get_mut(agent_id) {
            cap.available = available;
        }
    }

    /// Update agent load.
    pub fn set_load(&mut self, agent_id: &str, load: f32) {
        if let Some(cap) = self.capabilities.get_mut(agent_id) {
            cap.load = load.clamp(0.0, 1.0);
        }
    }

    /// Route an intent to the best matching agent.
    pub fn route(&mut self, intent: &SemanticIntent) -> Result<RouteResult> {
        self.metrics.total_routes += 1;

        // Score all agents
        let mut scored: Vec<_> = self
            .capabilities
            .values()
            .map(|cap| (cap.agent_id.clone(), cap.match_score(intent)))
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by score (descending)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if scored.is_empty() {
            self.metrics.failed_routes += 1;
            return Err(crate::core::Error::NoCapableAgent);
        }

        let (best_agent, best_score) = scored.remove(0);
        let alternatives: Vec<String> = scored.into_iter().take(3).map(|(id, _)| id).collect();

        self.metrics.successful_routes += 1;
        self.metrics.average_score = (self.metrics.average_score
            * (self.metrics.successful_routes - 1) as f64
            + best_score as f64)
            / self.metrics.successful_routes as f64;

        Ok(RouteResult {
            agent_id: best_agent,
            score: best_score,
            alternatives,
        })
    }

    /// Route with specific recipient preference.
    pub fn route_to(&self, intent: &SemanticIntent, recipient: &str) -> Result<RouteResult> {
        if let Some(cap) = self.capabilities.get(recipient) {
            if cap.available {
                let score = cap.match_score(intent);
                return Ok(RouteResult {
                    agent_id: recipient.to_string(),
                    score,
                    alternatives: Vec::new(),
                });
            }
        }
        Err(crate::core::Error::NoCapableAgent)
    }

    /// Get all agents capable of handling a domain.
    pub fn find_by_domain(&self, domain: &IntentDomain) -> Vec<&AgentCapability> {
        self.capabilities
            .values()
            .filter(|cap| cap.available && cap.supports_domain(domain))
            .collect()
    }

    /// Get all agents capable of handling an action.
    pub fn find_by_action(&self, action: &str) -> Vec<&AgentCapability> {
        self.capabilities
            .values()
            .filter(|cap| cap.available && cap.supports_action(action))
            .collect()
    }

    /// Get router metrics.
    pub fn metrics(&self) -> &RouterMetrics {
        &self.metrics
    }

    /// Get number of registered agents.
    pub fn agent_count(&self) -> usize {
        self.capabilities.len()
    }

    /// Get all registered agent IDs.
    pub fn agent_ids(&self) -> Vec<&String> {
        self.capabilities.keys().collect()
    }
}

impl Default for IntentRouter {
    fn default() -> Self {
        Self::new()
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
    fn test_agent_capability() {
        let agent = create_test_did();
        let cap = AgentCapability::new(&agent)
            .with_domain(IntentDomain::Data)
            .with_action("read")
            .with_action("write");

        assert!(cap.supports_domain(&IntentDomain::Data));
        assert!(!cap.supports_domain(&IntentDomain::Compute));
        assert!(cap.supports_action("read"));
        assert!(!cap.supports_action("process"));
    }

    #[test]
    fn test_router_registration() {
        let mut router = IntentRouter::new();
        let agent = create_test_did();
        let cap = AgentCapability::new(&agent).with_domain(IntentDomain::Data);

        router.register(cap);
        assert_eq!(router.agent_count(), 1);

        router.unregister(&agent.id);
        assert_eq!(router.agent_count(), 0);
    }

    #[test]
    fn test_router_routing() {
        let mut router = IntentRouter::new();

        // Register two agents
        let agent1 = create_test_did();
        let cap1 = AgentCapability::new(&agent1)
            .with_domain(IntentDomain::Data)
            .with_action("read")
            .with_score(0.8);

        let agent2 = create_test_did();
        let cap2 = AgentCapability::new(&agent2)
            .with_domain(IntentDomain::Data)
            .with_action("read")
            .with_score(0.9);

        router.register(cap1);
        router.register(cap2);

        // Create intent
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");

        let result = router.route(&intent).unwrap();
        // Agent2 should be selected (higher score)
        assert_eq!(result.agent_id, agent2.id);
        assert!(!result.alternatives.is_empty());
    }

    #[test]
    fn test_router_no_capable_agent() {
        let mut router = IntentRouter::new();

        let agent = create_test_did();
        let cap = AgentCapability::new(&agent).with_domain(IntentDomain::Data);
        router.register(cap);

        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Compute, "process");

        let result = router.route(&intent);
        assert!(result.is_err());
    }

    #[test]
    fn test_router_availability() {
        let mut router = IntentRouter::new();

        let agent = create_test_did();
        let cap = AgentCapability::new(&agent)
            .with_domain(IntentDomain::Data)
            .with_action("read");
        router.register(cap);

        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");

        // Should route successfully
        assert!(router.route(&intent).is_ok());

        // Set unavailable
        router.set_availability(&agent.id, false);
        assert!(router.route(&intent).is_err());

        // Set available again
        router.set_availability(&agent.id, true);
        assert!(router.route(&intent).is_ok());
    }

    #[test]
    fn test_router_load_balancing() {
        let mut router = IntentRouter::new();

        let agent1 = create_test_did();
        let cap1 = AgentCapability::new(&agent1)
            .with_domain(IntentDomain::Data)
            .with_action("read")
            .with_score(1.0);

        let agent2 = create_test_did();
        let mut cap2 = AgentCapability::new(&agent2)
            .with_domain(IntentDomain::Data)
            .with_action("read")
            .with_score(1.0);
        cap2.load = 0.9; // High load

        router.register(cap1);
        router.register(cap2);

        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");

        // Agent1 should be preferred due to lower load
        let result = router.route(&intent).unwrap();
        assert_eq!(result.agent_id, agent1.id);
    }

    #[test]
    fn test_find_by_domain() {
        let mut router = IntentRouter::new();

        let agent1 = create_test_did();
        let agent2 = create_test_did();

        router.register(
            AgentCapability::new(&agent1)
                .with_domain(IntentDomain::Data)
                .with_domain(IntentDomain::Compute),
        );
        router.register(AgentCapability::new(&agent2).with_domain(IntentDomain::Communication));

        let data_agents = router.find_by_domain(&IntentDomain::Data);
        assert_eq!(data_agents.len(), 1);

        let comm_agents = router.find_by_domain(&IntentDomain::Communication);
        assert_eq!(comm_agents.len(), 1);
    }

    #[test]
    fn test_router_metrics() {
        let mut router = IntentRouter::new();

        let agent = create_test_did();
        router.register(
            AgentCapability::new(&agent)
                .with_domain(IntentDomain::Data)
                .with_action("read"),
        );

        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");

        router.route(&intent).unwrap();
        router.route(&intent).unwrap();

        let metrics = router.metrics();
        assert_eq!(metrics.total_routes, 2);
        assert_eq!(metrics.successful_routes, 2);
        assert_eq!(metrics.failed_routes, 0);
    }
}

//! Kubernetes service management.
//!
//! Manages HIDO service endpoints and load balancing.

use crate::core::{now, Result, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Service configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Service type
    pub service_type: ServiceType,
    /// Selector labels
    pub selector: HashMap<String, String>,
    /// Service ports
    pub ports: Vec<ServicePort>,
    /// Annotations
    pub annotations: HashMap<String, String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        let mut selector = HashMap::new();
        selector.insert("app".to_string(), "hido".to_string());

        Self {
            name: "hido-service".to_string(),
            namespace: "default".to_string(),
            service_type: ServiceType::ClusterIP,
            selector,
            ports: vec![ServicePort {
                name: "http".to_string(),
                port: 80,
                target_port: 8080,
                protocol: "TCP".to_string(),
                node_port: None,
            }],
            annotations: HashMap::new(),
        }
    }
}

/// Service type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServiceType {
    /// Cluster-internal IP
    ClusterIP,
    /// Node port
    NodePort,
    /// Cloud load balancer
    LoadBalancer,
    /// External name
    ExternalName(String),
}

/// Service port configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServicePort {
    /// Port name
    pub name: String,
    /// Service port
    pub port: u16,
    /// Target port on pods
    pub target_port: u16,
    /// Protocol
    pub protocol: String,
    /// Node port (for NodePort/LoadBalancer)
    pub node_port: Option<u16>,
}

/// Endpoint for a service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Endpoint {
    /// Endpoint address (IP)
    pub address: String,
    /// Port
    pub port: u16,
    /// Is ready
    pub ready: bool,
    /// Pod name
    pub pod_name: Option<String>,
    /// Node name
    pub node_name: Option<String>,
}

/// HIDO Kubernetes service.
pub struct HIDOService {
    /// Service configuration
    pub config: ServiceConfig,
    /// Current endpoints
    endpoints: Vec<Endpoint>,
    /// Load balancer IP (if applicable)
    load_balancer_ip: Option<String>,
    /// Cluster IP
    cluster_ip: Option<String>,
    /// Created timestamp
    created: Timestamp,
}

impl HIDOService {
    /// Create a new service.
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            endpoints: Vec::new(),
            load_balancer_ip: None,
            cluster_ip: Some("10.0.0.100".to_string()), // Simulated
            created: now(),
        }
    }

    /// Create with defaults.
    pub fn default_service() -> Self {
        Self::new(ServiceConfig::default())
    }

    /// Create a LoadBalancer service.
    pub fn load_balancer(name: &str, port: u16, target_port: u16) -> Self {
        let mut config = ServiceConfig::default();
        config.name = name.to_string();
        config.service_type = ServiceType::LoadBalancer;
        config.ports = vec![ServicePort {
            name: "http".to_string(),
            port,
            target_port,
            protocol: "TCP".to_string(),
            node_port: None,
        }];

        Self::new(config)
    }

    /// Add an endpoint.
    pub fn add_endpoint(&mut self, endpoint: Endpoint) {
        self.endpoints.push(endpoint);
    }

    /// Remove endpoints for a pod.
    pub fn remove_pod_endpoints(&mut self, pod_name: &str) {
        self.endpoints.retain(|e| e.pod_name.as_deref() != Some(pod_name));
    }

    /// Get ready endpoints.
    pub fn ready_endpoints(&self) -> Vec<&Endpoint> {
        self.endpoints.iter().filter(|e| e.ready).collect()
    }

    /// Get all endpoints.
    pub fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }

    /// Get cluster IP.
    pub fn cluster_ip(&self) -> Option<&str> {
        self.cluster_ip.as_deref()
    }

    /// Get load balancer IP.
    pub fn load_balancer_ip(&self) -> Option<&str> {
        self.load_balancer_ip.as_deref()
    }

    /// Set load balancer IP (simulates cloud provisioning).
    pub fn set_load_balancer_ip(&mut self, ip: &str) {
        self.load_balancer_ip = Some(ip.to_string());
    }

    /// Check if service has ready endpoints.
    pub fn is_ready(&self) -> bool {
        !self.ready_endpoints().is_empty()
    }

    /// Get endpoint count.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Perform simple round-robin load balancing.
    pub fn select_endpoint(&self) -> Option<&Endpoint> {
        let ready = self.ready_endpoints();
        if ready.is_empty() {
            None
        } else {
            // Simple selection (in real impl would track index)
            ready.first().copied()
        }
    }

    /// Generate Kubernetes YAML manifest.
    pub fn to_yaml(&self) -> Result<String> {
        let service_type = match &self.config.service_type {
            ServiceType::ClusterIP => "ClusterIP",
            ServiceType::NodePort => "NodePort",
            ServiceType::LoadBalancer => "LoadBalancer",
            ServiceType::ExternalName(_) => "ExternalName",
        };

        let manifest = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Service",
            "metadata": {
                "name": self.config.name,
                "namespace": self.config.namespace,
                "annotations": self.config.annotations,
            },
            "spec": {
                "type": service_type,
                "selector": self.config.selector,
                "ports": self.config.ports.iter().map(|p| {
                    let mut port_spec = serde_json::json!({
                        "name": p.name,
                        "port": p.port,
                        "targetPort": p.target_port,
                        "protocol": p.protocol,
                    });
                    if let Some(np) = p.node_port {
                        port_spec["nodePort"] = serde_json::json!(np);
                    }
                    port_spec
                }).collect::<Vec<_>>(),
            },
        });

        Ok(serde_json::to_string_pretty(&manifest)?)
    }
}

/// Health check result.
#[derive(Clone, Debug)]
pub struct HealthCheck {
    /// Endpoint checked
    pub endpoint: Endpoint,
    /// Is healthy
    pub healthy: bool,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Error message if unhealthy
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = HIDOService::default_service();
        assert_eq!(service.config.name, "hido-service");
        assert!(service.cluster_ip().is_some());
    }

    #[test]
    fn test_load_balancer_service() {
        let service = HIDOService::load_balancer("my-lb", 443, 8443);
        assert!(matches!(service.config.service_type, ServiceType::LoadBalancer));
        assert_eq!(service.config.ports[0].port, 443);
    }

    #[test]
    fn test_add_endpoint() {
        let mut service = HIDOService::default_service();
        service.add_endpoint(Endpoint {
            address: "10.0.0.1".to_string(),
            port: 8080,
            ready: true,
            pod_name: Some("hido-pod-1".to_string()),
            node_name: Some("node-1".to_string()),
        });

        assert_eq!(service.endpoint_count(), 1);
        assert!(service.is_ready());
    }

    #[test]
    fn test_remove_pod_endpoints() {
        let mut service = HIDOService::default_service();
        service.add_endpoint(Endpoint {
            address: "10.0.0.1".to_string(),
            port: 8080,
            ready: true,
            pod_name: Some("pod-1".to_string()),
            node_name: None,
        });
        service.add_endpoint(Endpoint {
            address: "10.0.0.2".to_string(),
            port: 8080,
            ready: true,
            pod_name: Some("pod-2".to_string()),
            node_name: None,
        });

        service.remove_pod_endpoints("pod-1");
        assert_eq!(service.endpoint_count(), 1);
    }

    #[test]
    fn test_ready_endpoints() {
        let mut service = HIDOService::default_service();
        service.add_endpoint(Endpoint {
            address: "10.0.0.1".to_string(),
            port: 8080,
            ready: true,
            pod_name: None,
            node_name: None,
        });
        service.add_endpoint(Endpoint {
            address: "10.0.0.2".to_string(),
            port: 8080,
            ready: false,
            pod_name: None,
            node_name: None,
        });

        assert_eq!(service.ready_endpoints().len(), 1);
    }

    #[test]
    fn test_select_endpoint() {
        let mut service = HIDOService::default_service();
        assert!(service.select_endpoint().is_none());

        service.add_endpoint(Endpoint {
            address: "10.0.0.1".to_string(),
            port: 8080,
            ready: true,
            pod_name: None,
            node_name: None,
        });

        assert!(service.select_endpoint().is_some());
    }

    #[test]
    fn test_to_yaml() {
        let service = HIDOService::default_service();
        let yaml = service.to_yaml().unwrap();
        assert!(yaml.contains("Service"));
        assert!(yaml.contains("hido-service"));
    }

    #[test]
    fn test_load_balancer_ip() {
        let mut service = HIDOService::load_balancer("lb", 80, 8080);
        assert!(service.load_balancer_ip().is_none());

        service.set_load_balancer_ip("203.0.113.10");
        assert_eq!(service.load_balancer_ip(), Some("203.0.113.10"));
    }
}

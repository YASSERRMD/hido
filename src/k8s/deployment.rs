//! Kubernetes deployment configuration.
//!
//! Manages HIDO pod deployments.

use crate::core::{now, Result, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pod specification for HIDO agents.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PodSpec {
    /// Container image
    pub image: String,
    /// Container name
    pub name: String,
    /// Resource requests
    pub resources: ResourceRequirements,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Ports to expose
    pub ports: Vec<ContainerPort>,
    /// Liveness probe
    pub liveness_probe: Option<Probe>,
    /// Readiness probe
    pub readiness_probe: Option<Probe>,
}

impl Default for PodSpec {
    fn default() -> Self {
        Self {
            image: "hido:latest".to_string(),
            name: "hido-agent".to_string(),
            resources: ResourceRequirements::default(),
            env: HashMap::new(),
            ports: vec![ContainerPort {
                name: "http".to_string(),
                container_port: 8080,
                protocol: "TCP".to_string(),
            }],
            liveness_probe: Some(Probe::http("/health", 8080, 30)),
            readiness_probe: Some(Probe::http("/ready", 8080, 5)),
        }
    }
}

/// Resource requirements for a container.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU request (e.g., "100m")
    pub cpu_request: String,
    /// CPU limit
    pub cpu_limit: String,
    /// Memory request (e.g., "128Mi")
    pub memory_request: String,
    /// Memory limit
    pub memory_limit: String,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_request: "100m".to_string(),
            cpu_limit: "500m".to_string(),
            memory_request: "128Mi".to_string(),
            memory_limit: "512Mi".to_string(),
        }
    }
}

/// Container port configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerPort {
    /// Port name
    pub name: String,
    /// Port number
    pub container_port: u16,
    /// Protocol (TCP/UDP)
    pub protocol: String,
}

/// Health probe configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Probe {
    /// Probe type
    pub probe_type: ProbeType,
    /// Path for HTTP probe
    pub path: Option<String>,
    /// Port for probe
    pub port: u16,
    /// Initial delay seconds
    pub initial_delay_seconds: u32,
    /// Period seconds
    pub period_seconds: u32,
    /// Timeout seconds
    pub timeout_seconds: u32,
    /// Failure threshold
    pub failure_threshold: u32,
}

impl Probe {
    /// Create an HTTP probe.
    pub fn http(path: &str, port: u16, initial_delay: u32) -> Self {
        Self {
            probe_type: ProbeType::Http,
            path: Some(path.to_string()),
            port,
            initial_delay_seconds: initial_delay,
            period_seconds: 10,
            timeout_seconds: 5,
            failure_threshold: 3,
        }
    }

    /// Create a TCP probe.
    pub fn tcp(port: u16, initial_delay: u32) -> Self {
        Self {
            probe_type: ProbeType::Tcp,
            path: None,
            port,
            initial_delay_seconds: initial_delay,
            period_seconds: 10,
            timeout_seconds: 5,
            failure_threshold: 3,
        }
    }
}

/// Type of health probe.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProbeType {
    Http,
    Tcp,
    Exec,
}

/// Deployment configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Number of replicas
    pub replicas: u32,
    /// Update strategy
    pub strategy: UpdateStrategy,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Annotations
    pub annotations: HashMap<String, String>,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "hido".to_string());

        Self {
            name: "hido-deployment".to_string(),
            namespace: "default".to_string(),
            replicas: 3,
            strategy: UpdateStrategy::RollingUpdate {
                max_surge: 1,
                max_unavailable: 0,
            },
            labels,
            annotations: HashMap::new(),
        }
    }
}

/// Update strategy for deployments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UpdateStrategy {
    /// Rolling update
    RollingUpdate {
        max_surge: u32,
        max_unavailable: u32,
    },
    /// Recreate all pods
    Recreate,
}

/// HIDO Kubernetes deployment.
pub struct HIDODeployment {
    /// Deployment configuration
    pub config: DeploymentConfig,
    /// Pod specification
    pub pod_spec: PodSpec,
    /// Current status
    status: DeploymentStatus,
    /// Created timestamp
    created: Timestamp,
}

/// Deployment status.
#[derive(Clone, Debug, Default)]
pub struct DeploymentStatus {
    /// Desired replicas
    pub replicas: u32,
    /// Ready replicas
    pub ready_replicas: u32,
    /// Available replicas
    pub available_replicas: u32,
    /// Updated replicas
    pub updated_replicas: u32,
    /// Conditions
    pub conditions: Vec<DeploymentCondition>,
}

/// Deployment condition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentCondition {
    /// Condition type
    pub condition_type: String,
    /// Status (True/False/Unknown)
    pub status: String,
    /// Reason
    pub reason: Option<String>,
    /// Message
    pub message: Option<String>,
    /// Last update time
    pub last_update: Timestamp,
}

impl HIDODeployment {
    /// Create a new deployment.
    pub fn new(config: DeploymentConfig, pod_spec: PodSpec) -> Self {
        let status = DeploymentStatus {
            replicas: config.replicas,
            ..Default::default()
        };

        Self {
            config,
            pod_spec,
            status,
            created: now(),
        }
    }

    /// Create with defaults.
    pub fn default_deployment() -> Self {
        Self::new(DeploymentConfig::default(), PodSpec::default())
    }

    /// Scale the deployment.
    pub fn scale(&mut self, replicas: u32) {
        self.config.replicas = replicas;
        self.status.replicas = replicas;
    }

    /// Update the image.
    pub fn update_image(&mut self, image: &str) {
        self.pod_spec.image = image.to_string();
    }

    /// Add environment variable.
    pub fn add_env(&mut self, key: &str, value: &str) {
        self.pod_spec.env.insert(key.to_string(), value.to_string());
    }

    /// Get deployment status.
    pub fn status(&self) -> &DeploymentStatus {
        &self.status
    }

    /// Check if deployment is ready.
    pub fn is_ready(&self) -> bool {
        self.status.ready_replicas >= self.config.replicas
    }

    /// Check if deployment is available.
    pub fn is_available(&self) -> bool {
        self.status.available_replicas > 0
    }

    /// Simulate pod becoming ready.
    pub fn simulate_ready(&mut self, count: u32) {
        self.status.ready_replicas = count.min(self.config.replicas);
        self.status.available_replicas = self.status.ready_replicas;
        self.status.updated_replicas = self.status.ready_replicas;
    }

    /// Generate Kubernetes YAML manifest.
    pub fn to_yaml(&self) -> Result<String> {
        let manifest = serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": self.config.name,
                "namespace": self.config.namespace,
                "labels": self.config.labels,
                "annotations": self.config.annotations,
            },
            "spec": {
                "replicas": self.config.replicas,
                "selector": {
                    "matchLabels": self.config.labels,
                },
                "template": {
                    "metadata": {
                        "labels": self.config.labels,
                    },
                    "spec": {
                        "containers": [{
                            "name": self.pod_spec.name,
                            "image": self.pod_spec.image,
                            "ports": self.pod_spec.ports,
                            "resources": {
                                "requests": {
                                    "cpu": self.pod_spec.resources.cpu_request,
                                    "memory": self.pod_spec.resources.memory_request,
                                },
                                "limits": {
                                    "cpu": self.pod_spec.resources.cpu_limit,
                                    "memory": self.pod_spec.resources.memory_limit,
                                },
                            },
                            "env": self.pod_spec.env.iter()
                                .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
                                .collect::<Vec<_>>(),
                        }],
                    },
                },
            },
        });

        Ok(serde_json::to_string_pretty(&manifest)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_creation() {
        let deployment = HIDODeployment::default_deployment();
        assert_eq!(deployment.config.replicas, 3);
        assert_eq!(deployment.pod_spec.image, "hido:latest");
    }

    #[test]
    fn test_scale() {
        let mut deployment = HIDODeployment::default_deployment();
        deployment.scale(5);
        assert_eq!(deployment.config.replicas, 5);
    }

    #[test]
    fn test_update_image() {
        let mut deployment = HIDODeployment::default_deployment();
        deployment.update_image("hido:v2.0.0");
        assert_eq!(deployment.pod_spec.image, "hido:v2.0.0");
    }

    #[test]
    fn test_add_env() {
        let mut deployment = HIDODeployment::default_deployment();
        deployment.add_env("LOG_LEVEL", "debug");
        assert_eq!(deployment.pod_spec.env.get("LOG_LEVEL"), Some(&"debug".to_string()));
    }

    #[test]
    fn test_is_ready() {
        let mut deployment = HIDODeployment::default_deployment();
        assert!(!deployment.is_ready());

        deployment.simulate_ready(3);
        assert!(deployment.is_ready());
    }

    #[test]
    fn test_to_yaml() {
        let deployment = HIDODeployment::default_deployment();
        let yaml = deployment.to_yaml().unwrap();
        assert!(yaml.contains("Deployment"));
        assert!(yaml.contains("hido-deployment"));
    }

    #[test]
    fn test_probe_creation() {
        let http_probe = Probe::http("/health", 8080, 30);
        assert!(http_probe.path.is_some());
        assert_eq!(http_probe.port, 8080);

        let tcp_probe = Probe::tcp(3306, 10);
        assert!(tcp_probe.path.is_none());
    }

    #[test]
    fn test_resource_requirements() {
        let resources = ResourceRequirements::default();
        assert_eq!(resources.cpu_request, "100m");
        assert_eq!(resources.memory_limit, "512Mi");
    }
}

//! Kubernetes Module
//!
//! Provides Kubernetes deployment and service management:
//! - Deployment configuration
//! - Service management
//! - Resource management

pub mod deployment;
pub mod service;

pub use deployment::{HIDODeployment, DeploymentConfig, PodSpec};
pub use service::{HIDOService, ServiceConfig, Endpoint};

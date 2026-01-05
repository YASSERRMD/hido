//! Plugin interface definition.
//!
//! Defines the interface plugins must implement.

use crate::core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin ID
    pub id: String,
    /// Plugin name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Author
    pub author: String,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Required HIDO version
    pub hido_version: String,
    /// Plugin capabilities
    pub capabilities: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
}

impl PluginInfo {
    /// Create new plugin info.
    pub fn new(id: &str, name: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: String::new(),
            author: String::new(),
            homepage: None,
            hido_version: "0.1.0".to_string(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// Set description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Set author.
    pub fn with_author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }

    /// Add capability.
    pub fn with_capability(mut self, cap: &str) -> Self {
        self.capabilities.push(cap.to_string());
        self
    }

    /// Add dependency.
    pub fn with_dependency(mut self, dep: &str) -> Self {
        self.dependencies.push(dep.to_string());
        self
    }
}

/// Context passed to plugin methods.
#[derive(Clone, Debug)]
pub struct PluginContext {
    /// Configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Plugin data directory
    pub data_dir: String,
    /// Is development mode
    pub dev_mode: bool,
}

impl PluginContext {
    /// Create a new context.
    pub fn new(data_dir: &str) -> Self {
        Self {
            config: HashMap::new(),
            data_dir: data_dir.to_string(),
            dev_mode: false,
        }
    }

    /// Get config value.
    pub fn get_config<T: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.config.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set config value.
    pub fn set_config(&mut self, key: &str, value: serde_json::Value) {
        self.config.insert(key.to_string(), value);
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new("/tmp/hido/plugins")
    }
}

/// Result type for plugin operations.
pub type PluginResult<T> = std::result::Result<T, PluginError>;

/// Plugin-specific error.
#[derive(Clone, Debug)]
pub struct PluginError {
    /// Error message
    pub message: String,
    /// Error code
    pub code: i32,
    /// Is recoverable
    pub recoverable: bool,
}

impl PluginError {
    /// Create a new error.
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            code: -1,
            recoverable: true,
        }
    }

    /// Create a fatal error.
    pub fn fatal(message: &str) -> Self {
        Self {
            message: message.to_string(),
            code: -1,
            recoverable: false,
        }
    }
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PluginError: {}", self.message)
    }
}

impl std::error::Error for PluginError {}

/// Plugin hook types.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PluginHook {
    /// Before intent processing
    BeforeIntent,
    /// After intent processing
    AfterIntent,
    /// Before consensus
    BeforeConsensus,
    /// After consensus
    AfterConsensus,
    /// On agent registration
    OnAgentRegister,
    /// On agent deregistration
    OnAgentDeregister,
    /// Custom hook
    Custom(String),
}

/// Plugin trait that all plugins must implement.
pub trait Plugin: Send + Sync {
    /// Get plugin info.
    fn info(&self) -> PluginInfo;

    /// Initialize the plugin.
    fn init(&mut self, ctx: &PluginContext) -> PluginResult<()>;

    /// Shutdown the plugin.
    fn shutdown(&mut self) -> PluginResult<()>;

    /// Get supported hooks.
    fn hooks(&self) -> Vec<PluginHook> {
        Vec::new()
    }

    /// Execute a hook.
    fn execute_hook(
        &self,
        hook: &PluginHook,
        data: &HashMap<String, serde_json::Value>,
    ) -> PluginResult<HashMap<String, serde_json::Value>> {
        // Default: pass through unchanged
        Ok(data.clone())
    }

    /// Health check.
    fn health_check(&self) -> PluginResult<bool> {
        Ok(true)
    }
}

/// A simple example plugin for testing.
pub struct EchoPlugin {
    info: PluginInfo,
    initialized: bool,
}

impl EchoPlugin {
    /// Create a new echo plugin.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("echo", "Echo Plugin", "1.0.0")
                .with_description("A simple echo plugin for testing"),
            initialized: false,
        }
    }
}

impl Default for EchoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for EchoPlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        self.initialized = false;
        Ok(())
    }

    fn hooks(&self) -> Vec<PluginHook> {
        vec![PluginHook::BeforeIntent, PluginHook::AfterIntent]
    }

    fn execute_hook(
        &self,
        _hook: &PluginHook,
        data: &HashMap<String, serde_json::Value>,
    ) -> PluginResult<HashMap<String, serde_json::Value>> {
        // Echo: return data unchanged
        Ok(data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info() {
        let info = PluginInfo::new("test", "Test Plugin", "1.0.0")
            .with_description("A test plugin")
            .with_author("HIDO Team")
            .with_capability("intents");

        assert_eq!(info.id, "test");
        assert_eq!(info.capabilities.len(), 1);
    }

    #[test]
    fn test_plugin_context() {
        let mut ctx = PluginContext::default();
        ctx.set_config("key", serde_json::json!("value"));

        let value: Option<String> = ctx.get_config("key");
        assert_eq!(value, Some("value".to_string()));
    }

    #[test]
    fn test_echo_plugin() {
        let mut plugin = EchoPlugin::new();
        let ctx = PluginContext::default();

        assert!(!plugin.initialized);
        plugin.init(&ctx).unwrap();
        assert!(plugin.initialized);

        let hooks = plugin.hooks();
        assert_eq!(hooks.len(), 2);

        plugin.shutdown().unwrap();
        assert!(!plugin.initialized);
    }

    #[test]
    fn test_plugin_hook_execution() {
        let plugin = EchoPlugin::new();
        let mut data = HashMap::new();
        data.insert("key".to_string(), serde_json::json!("value"));

        let result = plugin.execute_hook(&PluginHook::BeforeIntent, &data).unwrap();
        assert_eq!(result.get("key"), data.get("key"));
    }
}

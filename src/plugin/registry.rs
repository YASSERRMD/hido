//! Plugin registry for managing plugins.
//!
//! Handles plugin registration, discovery, and lifecycle.

use crate::core::now;
use crate::plugin::interface::{Plugin, PluginContext, PluginHook, PluginInfo, PluginResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginStatus {
    /// Not loaded
    Unloaded,
    /// Loaded but not initialized
    Loaded,
    /// Active
    Active,
    /// Disabled
    Disabled,
    /// Error state
    Error(String),
}

/// Registered plugin entry.
pub struct RegisteredPlugin {
    /// Plugin instance
    pub plugin: Box<dyn Plugin>,
    /// Plugin info
    pub info: PluginInfo,
    /// Current status
    pub status: PluginStatus,
    /// Registration time
    pub registered_at: crate::core::Timestamp,
}

/// Plugin registry.
pub struct PluginRegistry {
    /// Registered plugins
    plugins: HashMap<String, RegisteredPlugin>,
    /// Hook mappings
    hooks: HashMap<PluginHook, Vec<String>>,
    /// Plugin context
    context: PluginContext,
}

impl PluginRegistry {
    /// Create a new registry.
    pub fn new(context: PluginContext) -> Self {
        Self {
            plugins: HashMap::new(),
            hooks: HashMap::new(),
            context,
        }
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> PluginResult<()> {
        let info = plugin.info();
        let id = info.id.clone();

        // Check for duplicates
        if self.plugins.contains_key(&id) {
            return Err(crate::plugin::interface::PluginError::new(&format!(
                "Plugin {} is already registered",
                id
            )));
        }

        // Register hooks
        for hook in plugin.hooks() {
            self.hooks.entry(hook).or_insert_with(Vec::new).push(id.clone());
        }

        self.plugins.insert(
            id,
            RegisteredPlugin {
                plugin,
                info,
                status: PluginStatus::Loaded,
                registered_at: now(),
            },
        );

        Ok(())
    }

    /// Unregister a plugin.
    pub fn unregister(&mut self, plugin_id: &str) -> PluginResult<()> {
        if let Some(entry) = self.plugins.remove(plugin_id) {
            // Remove from hooks
            for (_, plugins) in self.hooks.iter_mut() {
                plugins.retain(|id| id != plugin_id);
            }
            Ok(())
        } else {
            Err(crate::plugin::interface::PluginError::new(&format!(
                "Plugin {} not found",
                plugin_id
            )))
        }
    }

    /// Initialize a plugin.
    pub fn init_plugin(&mut self, plugin_id: &str) -> PluginResult<()> {
        if let Some(entry) = self.plugins.get_mut(plugin_id) {
            match entry.plugin.init(&self.context) {
                Ok(()) => {
                    entry.status = PluginStatus::Active;
                    Ok(())
                }
                Err(e) => {
                    entry.status = PluginStatus::Error(e.message.clone());
                    Err(e)
                }
            }
        } else {
            Err(crate::plugin::interface::PluginError::new(&format!(
                "Plugin {} not found",
                plugin_id
            )))
        }
    }

    /// Initialize all plugins.
    pub fn init_all(&mut self) -> Vec<(String, PluginResult<()>)> {
        let ids: Vec<String> = self.plugins.keys().cloned().collect();
        ids.into_iter()
            .map(|id| {
                let result = self.init_plugin(&id);
                (id, result)
            })
            .collect()
    }

    /// Shutdown a plugin.
    pub fn shutdown_plugin(&mut self, plugin_id: &str) -> PluginResult<()> {
        if let Some(entry) = self.plugins.get_mut(plugin_id) {
            match entry.plugin.shutdown() {
                Ok(()) => {
                    entry.status = PluginStatus::Disabled;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            Err(crate::plugin::interface::PluginError::new(&format!(
                "Plugin {} not found",
                plugin_id
            )))
        }
    }

    /// Execute a hook across all registered plugins.
    pub fn execute_hook(
        &self,
        hook: &PluginHook,
        mut data: HashMap<String, serde_json::Value>,
    ) -> PluginResult<HashMap<String, serde_json::Value>> {
        if let Some(plugin_ids) = self.hooks.get(hook) {
            for id in plugin_ids {
                if let Some(entry) = self.plugins.get(id) {
                    if entry.status == PluginStatus::Active {
                        data = entry.plugin.execute_hook(hook, &data)?;
                    }
                }
            }
        }
        Ok(data)
    }

    /// Get plugin by ID.
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&RegisteredPlugin> {
        self.plugins.get(plugin_id)
    }

    /// Get plugin status.
    pub fn get_status(&self, plugin_id: &str) -> Option<&PluginStatus> {
        self.plugins.get(plugin_id).map(|p| &p.status)
    }

    /// List all plugins.
    pub fn list_plugins(&self) -> Vec<&PluginInfo> {
        self.plugins.values().map(|p| &p.info).collect()
    }

    /// Get active plugins.
    pub fn active_plugins(&self) -> Vec<&PluginInfo> {
        self.plugins
            .values()
            .filter(|p| p.status == PluginStatus::Active)
            .map(|p| &p.info)
            .collect()
    }

    /// Get plugin count.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Health check all plugins.
    pub fn health_check_all(&self) -> HashMap<String, bool> {
        self.plugins
            .iter()
            .map(|(id, entry)| {
                let healthy = entry.plugin.health_check().unwrap_or(false);
                (id.clone(), healthy)
            })
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new(PluginContext::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::interface::EchoPlugin;

    #[test]
    fn test_registry_creation() {
        let registry = PluginRegistry::default();
        assert_eq!(registry.plugin_count(), 0);
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::default();
        registry.register(Box::new(EchoPlugin::new())).unwrap();

        assert_eq!(registry.plugin_count(), 1);
        assert!(registry.get_plugin("echo").is_some());
    }

    #[test]
    fn test_init_plugin() {
        let mut registry = PluginRegistry::default();
        registry.register(Box::new(EchoPlugin::new())).unwrap();

        registry.init_plugin("echo").unwrap();

        assert_eq!(
            registry.get_status("echo"),
            Some(&PluginStatus::Active)
        );
    }

    #[test]
    fn test_shutdown_plugin() {
        let mut registry = PluginRegistry::default();
        registry.register(Box::new(EchoPlugin::new())).unwrap();
        registry.init_plugin("echo").unwrap();
        registry.shutdown_plugin("echo").unwrap();

        assert_eq!(
            registry.get_status("echo"),
            Some(&PluginStatus::Disabled)
        );
    }

    #[test]
    fn test_unregister_plugin() {
        let mut registry = PluginRegistry::default();
        registry.register(Box::new(EchoPlugin::new())).unwrap();
        registry.unregister("echo").unwrap();

        assert_eq!(registry.plugin_count(), 0);
    }

    #[test]
    fn test_execute_hook() {
        let mut registry = PluginRegistry::default();
        registry.register(Box::new(EchoPlugin::new())).unwrap();
        registry.init_plugin("echo").unwrap();

        let mut data = HashMap::new();
        data.insert("test".to_string(), serde_json::json!("value"));

        let result = registry.execute_hook(&PluginHook::BeforeIntent, data).unwrap();
        assert!(result.contains_key("test"));
    }

    #[test]
    fn test_list_plugins() {
        let mut registry = PluginRegistry::default();
        registry.register(Box::new(EchoPlugin::new())).unwrap();

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "echo");
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = PluginRegistry::default();
        registry.register(Box::new(EchoPlugin::new())).unwrap();

        let result = registry.register(Box::new(EchoPlugin::new()));
        assert!(result.is_err());
    }
}

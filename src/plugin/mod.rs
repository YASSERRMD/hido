//! Plugin Module
//!
//! Provides extensible plugin architecture:
//! - Plugin interface
//! - Plugin registry
//! - Lifecycle management

pub mod interface;
pub mod registry;

pub use interface::{Plugin, PluginContext, PluginInfo, PluginResult};
pub use registry::{PluginRegistry, PluginStatus};

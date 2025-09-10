pub mod descriptor;
pub mod registry;
pub mod events;
pub mod config;
pub mod sandbox;

pub use descriptor::*;
pub use registry::*;
pub use events::*;
pub use config::*;
pub use sandbox::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Current plugin API version
pub const CURRENT_API_VERSION: u32 = 1;

/// Result type for plugin operations
pub type PluginResult<T> = Result<T>;

/// Plugin context provided during initialization and runtime
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub config: Arc<RwLock<PluginConfig>>,
    pub event_bus: Arc<EventBus>,
    pub capabilities: PluginCapabilities,
}

/// Capabilities that a plugin can request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilities {
    pub file_system_access: bool,
    pub network_access: bool,
    pub system_info_access: bool,
    pub backup_events: bool,
    pub ui_panels: bool,
    pub config_ui: bool,
}

impl Default for PluginCapabilities {
    fn default() -> Self {
        Self {
            file_system_access: false,
            network_access: false,
            system_info_access: false,
            backup_events: false,
            ui_panels: false,
            config_ui: false,
        }
    }
}

/// Core trait that all plugins must implement
pub trait NovaPlugin: Send + Sync {
    /// Get plugin descriptor metadata
    fn descriptor(&self) -> &PluginDescriptor;
    
    /// Initialize the plugin with context
    fn init(&mut self, ctx: &PluginContext) -> PluginResult<()>;
    
    /// Shutdown the plugin gracefully
    fn shutdown(&mut self) -> PluginResult<()>;
    
    /// Check if plugin is healthy/operational
    fn health_check(&self) -> PluginResult<PluginHealth>;
    
    /// Get plugin as Any for downcasting to specific plugin types
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginHealth {
    Healthy,
    Warning { message: String },
    Error { message: String },
}

/// Different types of plugins supported
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginType {
    Analyzer,
    Exporter,
    CloudSync,
    UI,
    Crypto,
    Integration,
}
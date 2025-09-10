use crate::{NovaPlugin, PluginDescriptor, PluginResult, PluginContext, PluginHealth};
use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing plugins in the system
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Box<dyn NovaPlugin>>>>,
    context: PluginContext,
}

impl PluginRegistry {
    pub fn new(context: PluginContext) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            context,
        }
    }

    /// Register a plugin with the registry
    pub async fn register_plugin(&self, mut plugin: Box<dyn NovaPlugin>) -> PluginResult<()> {
        let descriptor = plugin.descriptor().clone();
        
        // Validate plugin compatibility
        descriptor.validate_compatibility()?;
        
        // Initialize the plugin
        plugin.init(&self.context)?;
        
        // Store in registry
        let mut plugins = self.plugins.write().await;
        if plugins.contains_key(&descriptor.id) {
            return Err(anyhow!("Plugin with ID '{}' is already registered", descriptor.id));
        }
        
        plugins.insert(descriptor.id.clone(), plugin);
        
        tracing::info!("Registered plugin: {} v{}", descriptor.name, descriptor.version);
        Ok(())
    }

    /// Unregister a plugin by ID
    pub async fn unregister_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(mut plugin) = plugins.remove(plugin_id) {
            plugin.shutdown()?;
            tracing::info!("Unregistered plugin: {}", plugin_id);
            Ok(())
        } else {
            Err(anyhow!("Plugin '{}' not found", plugin_id))
        }
    }

    /// Get list of all registered plugin descriptors
    pub async fn list_plugins(&self) -> Vec<PluginDescriptor> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.descriptor().clone()).collect()
    }

    /// Get a specific plugin by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginDescriptor> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|p| p.descriptor().clone())
    }

    /// Check health of all plugins
    pub async fn health_check_all(&self) -> HashMap<String, PluginHealth> {
        let plugins = self.plugins.read().await;
        let mut health_map = HashMap::new();
        
        for (id, plugin) in plugins.iter() {
            let health = plugin.health_check().unwrap_or(PluginHealth::Error {
                message: "Health check failed".to_string(),
            });
            health_map.insert(id.clone(), health);
        }
        
        health_map
    }

    /// Get plugin count
    pub async fn plugin_count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;
        let plugin_ids: Vec<String> = plugins.keys().cloned().collect();
        
        for plugin_id in plugin_ids {
            if let Some(mut plugin) = plugins.remove(&plugin_id) {
                if let Err(e) = plugin.shutdown() {
                    tracing::error!("Failed to shutdown plugin {}: {}", plugin_id, e);
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PluginCapabilities, EventBus, PluginConfig};
    use std::any::Any;

    struct TestPlugin {
        descriptor: PluginDescriptor,
    }

    impl NovaPlugin for TestPlugin {
        fn descriptor(&self) -> &PluginDescriptor {
            &self.descriptor
        }

        fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
            Ok(())
        }

        fn shutdown(&mut self) -> PluginResult<()> {
            Ok(())
        }

        fn health_check(&self) -> PluginResult<PluginHealth> {
            Ok(PluginHealth::Healthy)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    fn create_test_plugin(id: &str) -> TestPlugin {
        TestPlugin {
            descriptor: PluginDescriptor {
                id: id.to_string(),
                name: format!("Test Plugin {}", id),
                version: semver::Version::new(1, 0, 0),
                api_version: crate::CURRENT_API_VERSION,
                authors: vec!["Test".to_string()],
                description: "Test plugin".to_string(),
                categories: vec![crate::PluginCategory::Backup],
                capabilities: PluginCapabilities::default(),
                dependencies: HashMap::new(),
                entry_point: None,
            },
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let context = PluginContext {
            config: Arc::new(RwLock::new(PluginConfig::new())),
            event_bus: Arc::new(EventBus::new()),
            capabilities: PluginCapabilities::default(),
        };
        
        let registry = PluginRegistry::new(context);
        let plugin = Box::new(create_test_plugin("test1"));
        
        registry.register_plugin(plugin).await.unwrap();
        assert_eq!(registry.plugin_count().await, 1);
        
        let plugins = registry.list_plugins().await;
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "test1");
    }
}
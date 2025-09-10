use nova_plugin_api::{
    NovaPlugin, PluginDescriptor, PluginContext, PluginResult, PluginHealth,
    EventType, NovaEvent, parse_plugin_descriptor,
};
use std::any::Any;

/// Example plugin implementation
pub struct ExamplePlugin {
    descriptor: PluginDescriptor,
    is_initialized: bool,
}

impl ExamplePlugin {
    pub fn new() -> PluginResult<Self> {
        // Load plugin descriptor from embedded nova_plugin.toml
        let toml_content = include_str!("../nova_plugin.toml");
        let descriptor = parse_plugin_descriptor(toml_content)?;
        
        Ok(Self {
            descriptor,
            is_initialized: false,
        })
    }

    async fn handle_backup_event(&self, event: &NovaEvent) -> PluginResult<()> {
        match event.event_type {
            EventType::BackupStarted => {
                tracing::info!("Example plugin: Backup started - {}", event.id);
                // Perform backup analysis initialization
            }
            EventType::BackupCompleted => {
                tracing::info!("Example plugin: Backup completed - {}", event.id);
                // Perform backup analysis and reporting
                self.analyze_backup(&event.data).await?;
            }
            EventType::BackupFailed => {
                tracing::warn!("Example plugin: Backup failed - {}", event.id);
                // Handle backup failure
            }
            _ => {}
        }
        Ok(())
    }

    async fn analyze_backup(&self, backup_data: &serde_json::Value) -> PluginResult<()> {
        // Example backup analysis logic
        if let Some(files_count) = backup_data.get("files_count").and_then(|v| v.as_u64()) {
            tracing::info!("Example plugin analyzed backup with {} files", files_count);
            
            // Generate analysis report
            let analysis = serde_json::json!({
                "total_files": files_count,
                "analysis_timestamp": chrono::Utc::now(),
                "status": "completed",
                "recommendations": [
                    "Consider enabling compression for larger files",
                    "Schedule regular backup verification"
                ]
            });
            
            tracing::info!("Backup analysis: {}", analysis);
        }
        
        Ok(())
    }
}

impl NovaPlugin for ExamplePlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        tracing::info!("Initializing example plugin");
        
        // Subscribe to backup events
        let _event_filter = nova_plugin_api::EventFilter {
            event_types: vec![
                EventType::BackupStarted,
                EventType::BackupCompleted,
                EventType::BackupFailed,
            ],
            include_system: true,
            include_user: true,
        };
        
        // In a real implementation, we would spawn a task to handle events
        // For now, we'll just mark as initialized
        self.is_initialized = true;
        
        tracing::info!("Example plugin initialized successfully");
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        tracing::info!("Shutting down example plugin");
        self.is_initialized = false;
        Ok(())
    }

    fn health_check(&self) -> PluginResult<PluginHealth> {
        if self.is_initialized {
            Ok(PluginHealth::Healthy)
        } else {
            Ok(PluginHealth::Error {
                message: "Plugin not initialized".to_string(),
            })
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Factory function to create the plugin (would be used for dynamic loading)
pub fn create_plugin() -> PluginResult<Box<dyn NovaPlugin>> {
    let plugin = ExamplePlugin::new()?;
    Ok(Box::new(plugin))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_plugin_api::{EventBus, PluginConfig, PluginCapabilities};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_example_plugin_creation() {
        let plugin = ExamplePlugin::new().unwrap();
        assert_eq!(plugin.descriptor().id, "example-plugin");
        assert_eq!(plugin.descriptor().name, "Example Plugin");
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = ExamplePlugin::new().unwrap();
        
        // Create mock context
        let context = PluginContext {
            config: Arc::new(RwLock::new(PluginConfig::new())),
            event_bus: Arc::new(EventBus::new()),
            capabilities: PluginCapabilities::default(),
        };

        // Test initialization
        plugin.init(&context).unwrap();
        
        // Test health check
        let health = plugin.health_check().unwrap();
        matches!(health, PluginHealth::Healthy);

        // Test shutdown
        plugin.shutdown().unwrap();
    }
}
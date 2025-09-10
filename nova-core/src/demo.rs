use anyhow::Result;
use nova_plugin_api::{
    EventBus, PluginConfig, PluginContext, PluginRegistry, PluginCapabilities,
    NovaEvent, EventType, NovaPlugin,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("=== NovaPcSuite Plugin System Demo ===");

    // Initialize plugin system
    let plugin_system = PluginSystem::new().await?;
    
    // Load and demonstrate example plugin
    demo_plugin_loading(&plugin_system).await?;
    
    // Demonstrate event system
    demo_event_system(&plugin_system).await?;
    
    // Demonstrate configuration
    demo_configuration(&plugin_system).await?;
    
    // Show plugin registry status
    demo_plugin_registry(&plugin_system).await?;
    
    // Cleanup
    plugin_system.shutdown().await?;
    
    info!("=== Demo Complete ===");
    Ok(())
}

/// Core plugin system management
pub struct PluginSystem {
    pub registry: Arc<PluginRegistry>,
    pub event_bus: Arc<EventBus>,
    pub config: Arc<RwLock<PluginConfig>>,
}

impl PluginSystem {
    pub async fn new() -> Result<Self> {
        info!("Initializing plugin system");

        // Create event bus
        let event_bus = Arc::new(EventBus::new());
        
        // Create and load plugin config
        let mut plugin_config = PluginConfig::new();
        plugin_config.load().await?;
        let config = Arc::new(RwLock::new(plugin_config));

        // Create plugin context
        let context = PluginContext {
            config: config.clone(),
            event_bus: event_bus.clone(),
            capabilities: PluginCapabilities::default(),
        };

        // Create plugin registry
        let registry = Arc::new(PluginRegistry::new(context));

        Ok(Self {
            registry,
            event_bus,
            config,
        })
    }

    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down plugin system");
        
        // Save configuration
        let config = self.config.read().await;
        config.save().await?;
        drop(config);

        // Shutdown all plugins
        self.registry.shutdown_all().await?;
        
        info!("Plugin system shutdown complete");
        Ok(())
    }
}

async fn demo_plugin_loading(system: &PluginSystem) -> Result<()> {
    info!("--- Plugin Loading Demo ---");
    
    // Create example plugin
    let example_plugin = example_plugin::ExamplePlugin::new()?;
    let plugin_id = example_plugin.descriptor().id.clone();
    let plugin_name = example_plugin.descriptor().name.clone();
    
    info!("Loading plugin: {} ({})", plugin_name, plugin_id);
    
    // Register plugin
    system.registry.register_plugin(Box::new(example_plugin)).await?;
    
    info!("Plugin loaded successfully!");
    
    // Show plugin count
    let count = system.registry.plugin_count().await;
    info!("Total plugins loaded: {}", count);
    
    Ok(())
}

async fn demo_event_system(system: &PluginSystem) -> Result<()> {
    info!("--- Event System Demo ---");
    
    // Subscribe to events
    let filter = nova_plugin_api::EventFilter {
        event_types: vec![EventType::BackupStarted, EventType::BackupCompleted],
        include_system: true,
        include_user: true,
    };
    
    let mut subscription = system.event_bus.subscribe("demo".to_string(), filter).await;
    
    // Spawn background task to handle events
    let event_handler = tokio::spawn(async move {
        let mut event_count = 0;
        while let Ok(event) = subscription.receiver.recv().await {
            event_count += 1;
            info!("Received event #{}: {:?} from {}", event_count, event.event_type, event.source);
            if event_count >= 2 {
                break;
            }
        }
    });
    
    // Publish some events
    info!("Publishing backup started event...");
    let event = NovaEvent::backup_started("demo".to_string(), "backup_001".to_string());
    system.event_bus.publish(event).await?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    info!("Publishing backup completed event...");
    let event = NovaEvent::backup_completed("demo".to_string(), "backup_001".to_string(), 150);
    system.event_bus.publish(event).await?;
    
    // Wait for events to be processed
    event_handler.await?;
    
    info!("Event system demo complete!");
    Ok(())
}

async fn demo_configuration(system: &PluginSystem) -> Result<()> {
    info!("--- Configuration Demo ---");
    
    // Set some plugin configuration
    let demo_config = serde_json::json!({
        "enabled": true,
        "interval_minutes": 30,
        "backup_path": "/tmp/backups",
        "max_files": 1000
    });
    
    {
        let mut config = system.config.write().await;
        config.set_plugin_config("demo-plugin".to_string(), demo_config);
        info!("Set configuration for demo-plugin");
    }
    
    // Read back configuration
    {
        let config = system.config.read().await;
        if let Some(plugin_config) = config.get_plugin_config("demo-plugin") {
            info!("Retrieved configuration: {}", serde_json::to_string_pretty(plugin_config)?);
        }
        
        let configured_plugins = config.configured_plugins();
        info!("Plugins with configuration: {:?}", configured_plugins);
    }
    
    info!("Configuration demo complete!");
    Ok(())
}

async fn demo_plugin_registry(system: &PluginSystem) -> Result<()> {
    info!("--- Plugin Registry Demo ---");
    
    // List all plugins
    let plugins = system.registry.list_plugins().await;
    info!("Registered plugins:");
    
    for plugin in &plugins {
        info!("  - {} v{} ({})", plugin.name, plugin.version, plugin.id);
        info!("    Description: {}", plugin.description);
        info!("    Categories: {:?}", plugin.categories);
        info!("    Capabilities: file_system={}, network={}, backup_events={}", 
              plugin.capabilities.file_system_access,
              plugin.capabilities.network_access,
              plugin.capabilities.backup_events);
    }
    
    // Check plugin health
    let health_status = system.registry.health_check_all().await;
    info!("Plugin health status:");
    
    for (plugin_id, health) in health_status {
        match health {
            nova_plugin_api::PluginHealth::Healthy => {
                info!("  ✅ {}: Healthy", plugin_id);
            }
            nova_plugin_api::PluginHealth::Warning { message } => {
                warn!("  ⚠️  {}: Warning - {}", plugin_id, message);
            }
            nova_plugin_api::PluginHealth::Error { message } => {
                warn!("  ❌ {}: Error - {}", plugin_id, message);
            }
        }
    }
    
    info!("Plugin registry demo complete!");
    Ok(())
}
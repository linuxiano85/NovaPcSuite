use anyhow::Result;
use nova_plugin_api::{
    EventBus, PluginConfig, PluginContext, PluginRegistry, PluginCapabilities,
};
use nova_ui::NovaApp;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting NovaPcSuite v{}", env!("CARGO_PKG_VERSION"));

    // Initialize plugin system
    let plugin_system = PluginSystem::new().await?;
    let registry_clone = plugin_system.registry.clone();
    
    // Run UI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("NovaPcSuite"),
        ..Default::default()
    };

    eframe::run_native(
        "NovaPcSuite",
        options,
        Box::new(move |_cc| {
            Box::new(NovaApp::new(registry_clone))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run UI: {}", e))?;

    // Cleanup
    plugin_system.shutdown().await?;
    
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

        // Load static plugins (would be loaded from workspace members)
        Self::load_static_plugins(&registry).await?;

        Ok(Self {
            registry,
            event_bus,
            config,
        })
    }

    async fn load_static_plugins(_registry: &PluginRegistry) -> Result<()> {
        info!("Loading static plugins from workspace");
        
        // In a real implementation, this would discover and load plugins
        // from the workspace members or a plugins directory
        // For now, we'll just log that the system is ready
        
        info!("Plugin system initialized successfully");
        Ok(())
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
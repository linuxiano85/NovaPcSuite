# Plugin Development Guide

This guide will help you develop plugins for NovaPcSuite. Plugins extend the core functionality and allow the community to add new features in a safe, modular way.

## Overview

NovaPcSuite plugins are Rust libraries that implement the `NovaPlugin` trait. They can:

- React to system events (backup completion, file changes, etc.)
- Provide custom UI components
- Integrate with external services
- Analyze system data
- Implement custom backup strategies

## Plugin Structure

### Directory Layout

```
my-plugin/
├── Cargo.toml
├── nova_plugin.toml       # Plugin descriptor
├── src/
│   └── lib.rs
└── README.md
```

### Cargo.toml

```toml
[package]
name = "my-plugin"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
nova-plugin-api = { version = "0.1" }
serde = { version = "1.0", features = ["derive"] }
```

### nova_plugin.toml

This file describes your plugin's metadata and capabilities:

```toml
id = "my-plugin"
name = "My Awesome Plugin"
version = "1.0.0"
api_version = 1
authors = ["Your Name <your.email@example.com>"]
description = "A plugin that does amazing things"
categories = ["backup", "analysis"]

[capabilities]
file_system_access = true
network_access = false
system_info_access = true
backup_events = true
ui_panels = false
config_ui = true

[dependencies]
# External dependencies your plugin needs
```

## Plugin Categories

### Backup Plugins
Handle backup-related functionality:
- Custom backup strategies
- Backup analysis and optimization
- Backup verification
- Storage management

### UI Plugins
Extend the user interface:
- Custom dashboards
- Configuration panels
- Data visualizations
- Status displays

### Analysis Plugins
Analyze system and data:
- Performance monitoring
- File analysis
- System health checks
- Usage statistics

### Transport Plugins
Handle data movement:
- Cloud sync providers
- Network protocols
- Data compression
- Encryption

### Integration Plugins
Connect with external services:
- Third-party APIs
- Database connections
- Messaging systems
- Notification services

## Core Plugin Implementation

### Basic Plugin Structure

```rust
use nova_plugin_api::{
    NovaPlugin, PluginDescriptor, PluginContext, PluginResult, 
    PluginHealth, parse_plugin_descriptor
};
use std::any::Any;

pub struct MyPlugin {
    descriptor: PluginDescriptor,
    // Plugin state
}

impl MyPlugin {
    pub fn new() -> PluginResult<Self> {
        let toml_content = include_str!("../nova_plugin.toml");
        let descriptor = parse_plugin_descriptor(toml_content)?;
        
        Ok(Self {
            descriptor,
        })
    }
}

impl NovaPlugin for MyPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        // Initialize your plugin
        // Subscribe to events, set up resources, etc.
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        // Clean up resources
        Ok(())
    }

    fn health_check(&self) -> PluginResult<PluginHealth> {
        // Report plugin health
        Ok(PluginHealth::Healthy)
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// Factory function for dynamic loading (future feature)
pub fn create_plugin() -> PluginResult<Box<dyn NovaPlugin>> {
    let plugin = MyPlugin::new()?;
    Ok(Box::new(plugin))
}
```

## Event System

Plugins can subscribe to and publish events through the event bus:

### Subscribing to Events

```rust
use nova_plugin_api::{EventType, EventFilter, NovaEvent};

impl NovaPlugin for MyPlugin {
    fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        // Subscribe to backup events
        let event_filter = EventFilter {
            event_types: vec![
                EventType::BackupStarted,
                EventType::BackupCompleted,
                EventType::BackupFailed,
            ],
            include_system: true,
            include_user: true,
        };

        // In a real implementation, you'd spawn a task to handle events
        let mut subscription = ctx.event_bus.subscribe(
            self.descriptor.id.clone(),
            event_filter
        ).await;

        // Handle events in a background task
        tokio::spawn(async move {
            while let Ok(event) = subscription.receiver.recv().await {
                handle_event(event).await;
            }
        });

        Ok(())
    }
}

async fn handle_event(event: NovaEvent) {
    match event.event_type {
        EventType::BackupStarted => {
            // Handle backup start
        }
        EventType::BackupCompleted => {
            // Handle backup completion
        }
        _ => {}
    }
}
```

### Publishing Events

```rust
use nova_plugin_api::NovaEvent;

// Publish a custom event
let event = NovaEvent::new(
    EventType::SystemInfo,
    "my-plugin".to_string(),
    serde_json::json!({
        "cpu_usage": 75.5,
        "memory_usage": 60.2
    })
);

ctx.event_bus.publish(event).await?;
```

## Configuration Management

Plugins can persist configuration data:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MyPluginConfig {
    enabled: bool,
    interval_seconds: u64,
    api_key: Option<String>,
}

impl NovaPlugin for MyPlugin {
    fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        // Load configuration
        let config = ctx.config.read().await;
        if let Some(plugin_config) = config.get_plugin_config(&self.descriptor.id) {
            let my_config: MyPluginConfig = serde_json::from_value(plugin_config.clone())?;
            // Use configuration
        }

        Ok(())
    }
}

// Save configuration
async fn save_config(ctx: &PluginContext, config: MyPluginConfig) -> PluginResult<()> {
    let config_value = serde_json::to_value(config)?;
    let mut plugin_config = ctx.config.write().await;
    plugin_config.set_plugin_config("my-plugin".to_string(), config_value);
    plugin_config.save().await?;
    Ok(())
}
```

## Security and Capabilities

### Capability Declaration

Declare what your plugin needs in `nova_plugin.toml`:

```toml
[capabilities]
file_system_access = true      # Read/write files
network_access = false         # Network requests
system_info_access = true     # System metrics
backup_events = true          # Backup-related events
ui_panels = false             # Custom UI components
config_ui = true              # Configuration interface
```

### Best Practices

1. **Minimal Permissions**: Only request capabilities you actually need
2. **Error Handling**: Use `PluginResult<T>` for all fallible operations
3. **Resource Cleanup**: Implement proper cleanup in `shutdown()`
4. **Health Monitoring**: Provide meaningful health check responses
5. **Documentation**: Document your plugin's purpose and configuration

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use nova_plugin_api::{EventBus, PluginConfig, PluginCapabilities};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_context() -> PluginContext {
        PluginContext {
            config: Arc::new(RwLock::new(PluginConfig::new())),
            event_bus: Arc::new(EventBus::new()),
            capabilities: PluginCapabilities::default(),
        }
    }

    #[tokio::test]
    async fn test_plugin_initialization() {
        let mut plugin = MyPlugin::new().unwrap();
        let context = create_test_context();
        
        assert!(plugin.init(&context).await.is_ok());
        assert!(matches!(plugin.health_check().unwrap(), PluginHealth::Healthy));
        assert!(plugin.shutdown().await.is_ok());
    }
}
```

### Integration Tests

Create integration tests that verify your plugin works with the actual plugin system.

## Building and Distribution

### Static Plugins (Current)

Add your plugin to the workspace `Cargo.toml`:

```toml
[workspace]
members = [
    "nova-core",
    "nova-plugin-api", 
    "nova-ui",
    "plugins/my-plugin"
]
```

### Dynamic Plugins (Future)

Dynamic loading will be supported in future versions with proper security constraints.

## Example: Backup Analyzer Plugin

Here's a complete example of a backup analyzer plugin:

```rust
use nova_plugin_api::{
    NovaPlugin, PluginDescriptor, PluginContext, PluginResult, PluginHealth,
    EventType, EventFilter, NovaEvent, parse_plugin_descriptor
};
use serde_json::Value;
use std::any::Any;

pub struct BackupAnalyzerPlugin {
    descriptor: PluginDescriptor,
    total_backups: u64,
    total_files: u64,
}

impl BackupAnalyzerPlugin {
    pub fn new() -> PluginResult<Self> {
        let toml_content = include_str!("../nova_plugin.toml");
        let descriptor = parse_plugin_descriptor(toml_content)?;
        
        Ok(Self {
            descriptor,
            total_backups: 0,
            total_files: 0,
        })
    }

    async fn analyze_backup(&mut self, backup_data: &Value) -> PluginResult<()> {
        if let Some(files_count) = backup_data.get("files_count").and_then(|v| v.as_u64()) {
            self.total_backups += 1;
            self.total_files += files_count;
            
            tracing::info!(
                "Backup analysis: {} total backups, {} total files processed",
                self.total_backups,
                self.total_files
            );
            
            // Generate efficiency report
            let avg_files_per_backup = self.total_files as f64 / self.total_backups as f64;
            if avg_files_per_backup < 100.0 {
                tracing::warn!("Low backup efficiency detected: avg {} files per backup", avg_files_per_backup);
            }
        }
        
        Ok(())
    }
}

impl NovaPlugin for BackupAnalyzerPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        tracing::info!("Initializing backup analyzer plugin");
        
        // Subscribe to backup completion events
        let event_filter = EventFilter {
            event_types: vec![EventType::BackupCompleted],
            include_system: true,
            include_user: true,
        };
        
        // In a real implementation, spawn a task to handle events
        // For this example, we'll just log that we're ready
        tracing::info!("Backup analyzer ready to receive events");
        
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        tracing::info!("Shutting down backup analyzer plugin");
        tracing::info!("Final stats: {} backups analyzed, {} total files", 
                      self.total_backups, self.total_files);
        Ok(())
    }

    fn health_check(&self) -> PluginResult<PluginHealth> {
        Ok(PluginHealth::Healthy)
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
```

## Getting Help

- Check the [API documentation](../docs/api/)
- Look at the [example plugin](../plugins/example-plugin/)
- Join our [community discussions](https://github.com/linuxiano85/NovaPcSuite/discussions)
- Open an [issue](https://github.com/linuxiano85/NovaPcSuite/issues) for bugs or questions

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement your plugin
4. Add tests
5. Submit a pull request

Make sure to follow our [coding standards](CONTRIBUTING.md) and include proper documentation.
# NovaPcSuite

A modular, extensible PC suite application with a first-class plugin architecture built in Rust.

## Features

- **Modular Plugin Architecture**: Extend functionality through community-driven plugins
- **Safe Plugin Execution**: Sandbox capabilities with future WASM support
- **Modern UI**: Built with egui for cross-platform compatibility
- **Event-Driven**: Comprehensive event bus for plugin communication
- **Configuration Management**: Per-plugin settings persistence
- **API Versioning**: Ensure plugin compatibility across versions

## Architecture Overview

NovaPcSuite is built around a core plugin system that allows developers to extend functionality in several categories:

- **Backup**: Backup analyzers, efficiency optimizers, custom backup strategies
- **UI**: Custom panels, dashboards, configuration interfaces
- **Analysis**: System analyzers, performance monitors, file analyzers
- **Transport**: Cloud sync providers, network protocols, data transfer mechanisms
- **Crypto**: Encryption strategies, key management, security plugins
- **Integration**: Third-party service integrations, API connectors

## Quick Start

### Running the Application

```bash
# Clone the repository
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite

# Build and run
cargo run --bin nova
```

### Building from Source

```bash
# Build all workspace members
cargo build

# Run tests
cargo test

# Build in release mode
cargo build --release
```

## Plugin Development

See [CONTRIBUTING-PLUGINS.md](CONTRIBUTING-PLUGINS.md) for detailed information on developing plugins for NovaPcSuite.

### Quick Plugin Example

```rust
use nova_plugin_api::{NovaPlugin, PluginDescriptor, PluginContext, PluginResult, PluginHealth};

struct MyPlugin {
    descriptor: PluginDescriptor,
}

impl NovaPlugin for MyPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        // Plugin initialization logic
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        // Cleanup logic
        Ok(())
    }

    fn health_check(&self) -> PluginResult<PluginHealth> {
        Ok(PluginHealth::Healthy)
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

## Workspace Structure

- **nova-core**: Main application binary
- **nova-plugin-api**: Plugin framework and API definitions
- **nova-ui**: User interface components
- **plugins/**: Example and community plugins
  - **example-plugin**: Reference implementation demonstrating plugin architecture

## Plugin System Features

### Current (v0.1.0)

- ✅ Core plugin traits and lifecycle management
- ✅ Static plugin loading (workspace members)
- ✅ Plugin descriptor format (`nova_plugin.toml`)
- ✅ Plugin registry with dependency resolution
- ✅ Event bus for plugin communication
- ✅ Configuration persistence
- ✅ Extensions UI for plugin management
- ✅ Capability-based security model (declarative)

### Planned (Future Releases)

- 🔄 Dynamic plugin loading (.so/.dylib) with security constraints
- 🔄 WASM-based plugin execution sandbox
- 🔄 Network permission gating enforcement
- 🔄 Plugin store/marketplace UI
- 🔄 Digital signature verification
- 🔄 Hot plugin reloading
- 🔄 Plugin dependency management

## API Versioning

The plugin API uses semantic versioning to ensure compatibility:

- **Current API Version**: 1
- Plugins must declare their required API version in `nova_plugin.toml`
- Breaking changes will increment the major API version
- Backward compatibility is maintained within major versions

## Configuration

Plugin configurations are stored in:
- **Linux/macOS**: `~/.config/nova-pc-suite/plugins/`
- **Windows**: `%APPDATA%/nova-pc-suite/plugins/`

## Contributing

We welcome contributions! Please see:
- [CONTRIBUTING.md](CONTRIBUTING.md) for general contribution guidelines
- [CONTRIBUTING-PLUGINS.md](CONTRIBUTING-PLUGINS.md) for plugin development

## Automazione Repository

NovaPcSuite utilizza un sistema di automazione avanzato per semplificare il processo di sviluppo:

- **Gestione Label**: Tassonomia centralizzata con sincronizzazione automatica
- **Auto-Merge**: Merge automatico condizionale basato su label e stato
- **Code Ownership**: Revisioni strutturate per componenti critici
- **Quality Gates**: Controlli e validazioni automatiche

Per informazioni dettagliate su come utilizzare il sistema di automazione, consultare la [Guida all'Automazione](docs/automation.md).

## License

MIT License - see [LICENSE](LICENSE) for details.

## Getting Help

- 📖 [Documentation](docs/)
- 🐛 [Issues](https://github.com/linuxiano85/NovaPcSuite/issues)
- 💬 [Discussions](https://github.com/linuxiano85/NovaPcSuite/discussions)
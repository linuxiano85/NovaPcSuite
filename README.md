# NovaPcSuite

A modular, extensible PC suite application with both Rust plugin architecture and Python-based Android device management capabilities.

## Dual Architecture

NovaPcSuite now features two complementary implementations:

### 1. Rust Plugin System (Original)
- **Modular Plugin Architecture**: Extend functionality through community-driven plugins
- **Safe Plugin Execution**: Sandbox capabilities with future WASM support
- **Modern UI**: Built with egui for cross-platform compatibility
- **Event-Driven**: Comprehensive event bus for plugin communication
- **Configuration Management**: Per-plugin settings persistence
- **API Versioning**: Ensure plugin compatibility across versions

### 2. Python Android Management Suite (NEW)
- **Linux-centric Android Device Management**: Complete backup and restore solution
- **ADB Integration**: Direct Android Debug Bridge communication
- **Incremental Backups**: Efficient file-level backup with hash verification
- **Data Export**: Contacts (vCard/CSV), call logs, SMS messages
- **APK Backup**: Application package extraction and management
- **CLI Interface**: Rich command-line interface with progress tracking

## Python Implementation Features

### Core Capabilities
- **Device Detection**: Automatic Android device discovery and information retrieval
- **File Backup**: Incremental backup with configurable paths and exclusions
- **APK Management**: Backup and restore of installed applications
- **Data Export**: Export contacts, call logs, and SMS in multiple formats
- **Manifest System**: YAML/JSON backup metadata with integrity verification
- **Progress Tracking**: Real-time progress bars and detailed logging

### Target Devices
- Primary target: Redmi Note 12 Pro Plus 5G
- Generic Android device support via ADB
- No root required for basic functionality (enhanced features with root)

### Command Line Interface

#### Device Management
```bash
# List connected devices
novapcsuite device list

# Show device information
novapcsuite device info --serial <device_serial>

# Show bootloader/OEM information
novapcsuite device oem-info
```

#### Backup Operations
```bash
# Run complete backup
novapcsuite backup run --serial <device_serial>

# Run backup with custom options
novapcsuite backup run --include /sdcard/DCIM --exclude "*.tmp" --no-apk

# List available backups
novapcsuite backup list

# Show backup details
novapcsuite backup show <backup_id>
```

#### Data Export
```bash
# Export contacts
novapcsuite export contacts --format vcf,csv --output ./exports

# Export call logs and SMS
novapcsuite export logs --calls --sms --output ./exports
```

#### Application Management
```bash
# List installed applications
novapcsuite apps list --system

# Backup specific applications
novapcsuite apps backup --packages com.example.app1 com.example.app2
```

#### Restore Operations
```bash
# Restore files to local directory
novapcsuite restore files <backup_id> --target-dir ./restored_files
```

## Architecture Overview (Rust Plugin System)

NovaPcSuite's Rust implementation is built around a core plugin system that allows developers to extend functionality in several categories:

- **Backup**: Backup analyzers, efficiency optimizers, custom backup strategies
- **UI**: Custom panels, dashboards, configuration interfaces
- **Analysis**: System analyzers, performance monitors, file analyzers
- **Transport**: Cloud sync providers, network protocols, data transfer mechanisms
- **Crypto**: Encryption strategies, key management, security plugins
- **Integration**: Third-party service integrations, API connectors

## Quick Start

### Python Android Management

#### Prerequisites
- Python 3.11+
- Android Debug Bridge (ADB) installed and in PATH
- Android device with USB debugging enabled

#### Installation
```bash
# Clone the repository
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite

# Install Python package
pip install -e .

# Verify installation
novapcsuite --help
```

#### Basic Usage
```bash
# Check connected devices
novapcsuite device list

# Run a backup (will prompt for device if multiple)
novapcsuite backup run

# Export contacts
novapcsuite export contacts
```

### Rust Plugin System

#### Running the Application
```bash
# Build and run
cargo run --bin nova
```

#### Building from Source
```bash
# Build all workspace members
cargo build

# Run tests
cargo test

# Build in release mode
cargo build --release
```

## Workspace Structure

```
NovaPcSuite/
├── nova-core/              # Main Rust application binary
├── nova-plugin-api/        # Plugin framework and API definitions
├── nova-ui/               # User interface components
├── plugins/               # Example and community plugins
│   └── example-plugin/    # Reference implementation
├── nova/                  # Python Android management suite
│   ├── adb/              # ADB interaction modules
│   ├── backup/           # Backup and restore engine
│   ├── data/             # Data export modules
│   ├── apps/             # Application management
│   └── util/             # Utility functions
├── tests/                # Python test suite
├── pyproject.toml        # Python package configuration
├── Cargo.toml           # Rust workspace configuration
└── README.md            # This file
```

## Python Package Structure

- **nova.adb**: Android Debug Bridge interaction
  - Device detection and information
  - Shell command execution
  - File pulling and package management
  - Content provider access
- **nova.backup**: Backup and restore system
  - Device scanning and file discovery
  - Incremental backup execution
  - Manifest management
  - Restore functionality
- **nova.data**: Data export modules
  - Contacts export (vCard/CSV)
  - Call log export
  - SMS message export
- **nova.apps**: Application management
  - APK backup and metadata
  - Package enumeration
- **nova.util**: Utility functions
  - Logging and configuration
  - Hashing and compression
  - Path and time utilities

## Configuration

### Python Configuration
Configuration is stored in `~/.config/novapcsuite/config.yaml`:

```yaml
backup_root: ~/.local/share/novapcsuite/backups
scanner:
  include_paths:
    - /sdcard/DCIM
    - /sdcard/Pictures
    - /sdcard/Documents
  exclude_patterns:
    - "*.tmp"
    - "*.cache"
  max_file_size_mb: 1024
backup:
  incremental: true
  hash_algorithm: sha256
export:
  contact_formats: [vcf, csv]
  include_call_logs: true
  include_sms: true
```

### Rust Plugin Configuration
Plugin configurations are stored in:
- **Linux/macOS**: `~/.config/nova-pc-suite/plugins/`
- **Windows**: `%APPDATA%/nova-pc-suite/plugins/`

## Plugin System Features (Rust)

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

## Python Implementation Roadmap

### Completed (v0.1.0)
- ✅ Core ADB interaction and device management
- ✅ File backup with incremental support
- ✅ APK backup and management
- ✅ Data export (contacts, call logs, SMS)
- ✅ CLI interface with rich formatting
- ✅ Configuration management
- ✅ Backup manifest system

### Future Releases
- 🔄 Advanced backup rules engine (#67)
- 🔄 REST API server (#64)
- 🔄 Terminal UI (TUI) interface (#63)
- 🔄 Live streaming backup (#66)
- 🔄 APK signature verification (#68)
- 🔄 Forensics mode (#69)
- 🔄 Multi-user profile support (#70)
- 🔄 Distributed backup queue (#65)

## Security Considerations

### Python Implementation
- Hash-based file integrity verification (SHA-256)
- Configurable file size limits
- Path validation and sanitization
- Future: Backup encryption and signing

### Rust Plugin System
- Capability-based permission model
- Sandbox execution environment
- API versioning for compatibility
- Future: WASM isolation and signature verification

## Device Compatibility

### Redmi Note 12 Pro Plus 5G Notes
- ✅ Standard ADB backup supported
- ⚠️ Full application data backup requires root access
- ✅ Contacts, call logs, SMS export supported (with permissions)
- ✅ APK backup fully supported
- ⚠️ Some system paths may require root access

### General Android Compatibility
- Android 8.0+ recommended
- ADB debugging must be enabled
- USB debugging permissions required
- Some features require specific Android permissions

## Contributing

We welcome contributions! Please see:
- [CONTRIBUTING.md](CONTRIBUTING.md) for general contribution guidelines
- [CONTRIBUTING-PLUGINS.md](CONTRIBUTING-PLUGINS.md) for plugin development

## API Versioning

The plugin API uses semantic versioning to ensure compatibility:

- **Current API Version**: 1
- Plugins must declare their required API version in `nova_plugin.toml`
- Breaking changes will increment the major API version
- Backward compatibility is maintained within major versions

## Dependencies

### Python Dependencies
- **rich**: Enhanced console output and logging
- **click**: Command-line interface framework
- **pydantic**: Data validation and settings management
- **ruamel.yaml**: YAML configuration parsing
- **tenacity**: Retry logic for ADB operations
- **tqdm**: Progress bars
- **cryptography**: Future encryption support

### Development Dependencies
- **pytest**: Testing framework
- **black**: Code formatting
- **mypy**: Type checking

## License

MIT License - see [LICENSE](LICENSE) for details.

## Getting Help

- 📖 [Documentation](docs/)
- 🐛 [Issues](https://github.com/linuxiano85/NovaPcSuite/issues)
- 💬 [Discussions](https://github.com/linuxiano85/NovaPcSuite/discussions)
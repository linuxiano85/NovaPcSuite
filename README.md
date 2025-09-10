# NovaPcSuite

Advanced PC backup and maintenance suite with chunked Merkle-based snapshots.

## Features

- **Backup Engine**: Adaptive chunking with BLAKE3 hashing and Merkle trees for integrity verification
- **Deduplication**: Content-addressed storage with perceptual hashing for media files
- **Plugin System**: WASM-based extensible architecture for custom functionality
- **Telephony Integration**: Remote notifications and companion app support
- **Scheduling**: Automated backup scheduling with systemd integration
- **Restore System**: Reliable data restoration from chunked snapshots
- **CLI Interface**: Comprehensive command-line tools for backup management

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

### Basic Usage

```bash
# Create a backup
nova-pc-suite backup --source /home/user/documents --output /backup/storage --label "documents-backup"

# Generate a report
nova-pc-suite backup --source /home/user/documents --output /backup/storage --generate-report

# List available backups
nova-pc-suite manifest --backup-dir /backup/storage --list

# Restore from a backup
nova-pc-suite restore --backup-dir /backup/storage --manifest-id <id> --target /restore/location
```

### Quick Example

```rust
use nova_pc_suite::backup::{BackupEngine, LocalFsSource};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let engine = BackupEngine::new(Path::new("./backup-output"));
    let source = LocalFsSource::new(Path::new("./my-data"));

    let manifest = engine.create_snapshot(&source, "initial-backup").await?;
    println!("Backup completed: {}", manifest.id());
    Ok(())
}
```

## Architecture

NovaPcSuite uses a modular architecture with the following components:

### Backup Engine
- **Adaptive Chunking**: Default 2 MiB chunks with small-file fast path
- **BLAKE3 Hashing**: Fast cryptographic hashing per chunk
- **Merkle Trees**: File integrity verification using chunk hash folding
- **Content-Addressed Storage**: Automatic deduplication through hash-based storage

### Deduplication System
- **Content Deduplication**: Identical chunks stored only once
- **Perceptual Hashing**: Similar image detection using simplified pHash
- **Audio Fingerprinting**: Placeholder for future audio similarity detection
- **Similarity Clustering**: Grouping of similar media files

### Plugin Architecture
- **WASM Runtime**: Secure plugin execution in sandboxed environment
- **Event System**: Platform-wide event bus for plugin communication
- **Host Functions**: Controlled API access for plugins
- **Resource Limits**: Memory and CPU quotas for plugin safety

### CLI Interface
- **Backup Operations**: Create, schedule, and manage backups
- **Scan & Analysis**: File analysis and similarity detection
- **Report Generation**: JSON and HTML backup reports
- **Manifest Management**: View and verify backup manifests
- **Device Management**: Future companion device integration

## Feature Flags

NovaPcSuite uses feature flags to enable optional functionality:

```toml
# Default features
default = ["encryption", "telephony"]

# Optional features
encryption = ["chacha20poly1305", "age", "zeroize"]  # File encryption
wasm-plugins = ["wasmtime"]                          # WASM plugin support
telephony = ["async-trait"]                          # Companion app integration
```

## Advanced Features

### Automated Scheduling

Generate systemd units for automated backups:

```bash
# Create a scheduled backup
nova-pc-suite schedule create --name "daily-docs" \
  --source /home/user/documents \
  --output /backup/storage \
  --schedule "daily"
```

### Similarity Detection

Find duplicate and similar files:

```bash
# Scan for similar files
nova-pc-suite scan --path /home/user/photos --find-similar
```

### Report Generation

Generate comprehensive backup reports:

```bash
# Generate HTML report
nova-pc-suite report --backup-dir /backup/storage --format html
```

## Documentation

- [Extended Backup Architecture](README-EXTENDED-BACKUP.md) - Deep dive into backup engine design
- [Plugin Development Guide](CONTRIBUTING-PLUGINS.md) - How to develop plugins
- [Companion API Specification](docs/companion_api.md) - API for companion apps

## Development

### Building from Source

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with specific features
cargo build --features "encryption,wasm-plugins"
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test backup::tests

# Run integration tests
cargo test --test integration
```

## Roadmap

### Current Status (v0.1.0)
- ✅ Core backup engine with chunking and Merkle trees
- ✅ Basic deduplication and content-addressed storage
- ✅ CLI interface for backup operations
- ✅ Report generation (JSON/HTML)
- ✅ Plugin system foundation
- ✅ Restore system skeleton

### Upcoming Features
- 🔄 Full encryption implementation
- 🔄 Advanced restore with integrity verification
- 🔄 Real-time companion app integration
- 🔄 WebSocket API server
- 🔄 Advanced deduplication algorithms
- 🔄 Performance optimizations

### Future Enhancements
- 📅 GUI application
- 📅 Cloud storage backends
- 📅 Multi-device synchronization
- 📅 Machine learning for intelligent backup strategies
- 📅 Enterprise features

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on:

- Code style and standards
- Pull request process
- Issue reporting
- Plugin development

## Support

- 📖 Documentation: Check the docs/ directory
- 🐛 Bug Reports: Create an issue on GitHub
- 💡 Feature Requests: Discuss in GitHub Issues
- 💬 Questions: Start a GitHub Discussion

## Acknowledgments

- BLAKE3 hashing algorithm
- Tokio async runtime
- Clap CLI framework
- All contributors and testers

---

**Note**: This project is in active development. APIs may change between versions until v1.0.0.
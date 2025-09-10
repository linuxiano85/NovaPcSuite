# NovaPcSuite

**Linux-first Android device management suite** written in Rust for backup, restore, and data management.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.77+-orange.svg)](https://www.rust-lang.org/)

## Overview

NovaPcSuite is a modern, Linux-focused alternative to proprietary Windows-only phone management suites. Built with Rust for safety and performance, it provides:

- **Device Detection**: Automatic discovery of connected Android devices via ADB
- **File System Scanning**: Categorized analysis of photos, videos, documents, and other files
- **Duplicate Detection**: Find and manage duplicate files to save storage space  
- **Backup Planning**: Generate structured backup plans with compression support
- **Contact Export**: Export contacts to standard formats (vCard .vcf, CSV)
- **CLI Interface**: Full command-line interface for automation and scripting
- **GUI Interface**: Minimal Tauri-based desktop application (optional)

### Target Devices

While designed to work with any Android device accessible via ADB, development focuses on devices like:
- Xiaomi Redmi Note 12 Pro Plus 5G
- Other Android devices with USB debugging enabled

## Features

### ✅ Implemented (v0.1.0)
- [x] Cargo workspace with modular crates
- [x] Device detection via ADB
- [x] Basic device capability detection (root access, etc.)
- [x] File system scanning with categorization
- [x] Duplicate detection by size and filename
- [x] Backup plan generation (JSON format)
- [x] Contact export (vCard, CSV) - basic implementation
- [x] CLI interface with core commands
- [x] Tauri UI framework (requires system dependencies)
- [x] Structured logging and configuration
- [x] Error handling with meaningful messages

### 🚧 Planned Features
- [ ] Full backup execution with file transfer
- [ ] Restore functionality
- [ ] Advanced contact parsing
- [ ] App APK/data backup (requires root)
- [ ] SMS and call log export
- [ ] Scheduled backup automation
- [ ] Encryption for backup archives
- [ ] GUI polish and theming
- [ ] Package distribution (.deb, .rpm, AppImage)

## Installation

### Prerequisites

**Minimum Requirements:**
- Rust 1.77 or later
- Linux (primary target)
- ADB (Android Debug Bridge) - install via package manager

**Install ADB:**
```bash
# Ubuntu/Debian
sudo apt install android-tools-adb

# Fedora
sudo dnf install android-tools

# Arch Linux
sudo pacman -S android-tools
```

**Optional (for GUI):**
- GTK3/4 development libraries
- Tauri prerequisites (Node.js, webkit2gtk)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite

# Build the CLI (minimal dependencies)
cargo build --release -p nova-cli

# Build all components (requires system dependencies for GUI)
cargo build --release

# Install CLI globally (optional)
cargo install --path nova-cli
```

## Usage

### Command Line Interface

#### List Connected Devices
```bash
nova-cli devices
```

#### Scan Device Files
```bash
# Basic scan
nova-cli scan

# Scan with JSON output
nova-cli scan --json-output scan-results.json

# Scan specific device
nova-cli scan --device-serial ABC123 --compute-hashes
```

#### Create Backup Plan
```bash
# Create backup plan for common directories
nova-cli plan --include /storage/emulated/0/DCIM \
              --include /storage/emulated/0/Pictures \
              --out backup-plan.json \
              --compression

# Specific device
nova-cli plan --device-serial ABC123 --include /storage/emulated/0/Documents --out docs-plan.json
```

#### Export Contacts
```bash
# Export to vCard format
nova-cli contacts export --format vcf --out contacts.vcf

# Export to CSV
nova-cli contacts export --format csv --out contacts.csv
```

### Device Setup

1. **Enable USB Debugging** on your Android device:
   - Go to Settings → About Phone
   - Tap "Build Number" 7 times to enable Developer Options
   - Go to Settings → Developer Options
   - Enable "USB Debugging"

2. **Connect device** via USB and authorize the computer when prompted

3. **Verify connection**:
   ```bash
   adb devices
   nova-cli devices
   ```

### GUI Application (Optional)

If Tauri dependencies are available:

```bash
# Development mode
cd nova-ui
cargo tauri dev

# Build for production
cargo tauri build
```

## Architecture

NovaPcSuite is organized as a Cargo workspace with the following crates:

- **`nova-core`** - Device abstraction, logging, configuration, error types
- **`nova-adb`** - Safe wrapper around ADB commands
- **`nova-mtp`** - MTP file system access (future enhancement)
- **`nova-backup`** - Scanning logic, duplicate detection, backup planning
- **`nova-formats`** - Contact export formats and data structure definitions
- **`nova-cli`** - Command-line interface
- **`nova-ui`** - Tauri-based desktop application

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

## Configuration

Configuration file location: `~/.config/novapcsuite/config.toml`

Example configuration:
```toml
[backup]
default_backup_dir = "/home/user/NovaBackups"
compression_enabled = true
verify_checksums = true
max_parallel_operations = 4

[ui]
theme = "dark"
remember_window_size = true
auto_scan_on_device_connect = true

[logging]
level = "info"
file_enabled = true
console_enabled = true
```

## Development

### Building
```bash
# Check all crates
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

### Running Tests
```bash
# All tests
cargo test

# Specific crate
cargo test -p nova-core
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Ensure clippy passes (`cargo clippy`)
- Add tests for new functionality
- Update documentation as needed

## Security Considerations

- ADB commands are validated before execution
- No arbitrary shell command execution
- File operations are sandboxed to intended directories
- Configuration files use safe defaults

## License

This project is dual-licensed under:
- MIT License ([LICENSE](LICENSE))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

## Roadmap

See [FUTURE_FEATURES.md](FUTURE_FEATURES.md) for detailed roadmap and planned features.

## Support

- **Issues**: [GitHub Issues](https://github.com/linuxiano85/NovaPcSuite/issues)
- **Discussions**: [GitHub Discussions](https://github.com/linuxiano85/NovaPcSuite/discussions)

## Acknowledgments

- Built with the excellent Rust ecosystem
- Tauri for cross-platform desktop applications
- Android Debug Bridge (ADB) for device communication
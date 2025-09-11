# NovaPcSuite

A modular, extensible PC suite for Android device backup and restore on Linux, built with Python and GTK4.

> **Note**: This project has pivoted from the previous Rust implementation (PR #72 canceled) to a Python-based architecture for better maintainability and cross-platform compatibility.

## Features

- **Device Management**: Connect and manage Android devices via ADB
- **Full Backup**: Complete backup of device files with manifest tracking
- **Incremental Restore**: Restore backed up files to device
- **CLI Interface**: Command-line tools for automated workflows
- **Modern GUI**: GTK4-based graphical interface with sidebar navigation
- **Extensible Architecture**: Modular design for easy feature additions

## Quick Start

### Prerequisites

- Python 3.8+
- Poetry (recommended) or pip
- Android Platform Tools (ADB)
- For GUI: GTK4 development libraries

### Installation

#### Using Poetry (Recommended)

```bash
# Clone the repository
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite

# Install dependencies
poetry install

# For GUI support (optional)
sudo apt-get install python3-gi python3-gi-cairo gir1.2-gtk-4.0 gir1.2-adwaita-1
```

#### Using pip

```bash
# Clone the repository  
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite

# Install dependencies
pip install -r requirements.txt
```

### Usage

#### Command Line Interface

```bash
# Show device information
poetry run novapcsuite device info

# Create a backup
poetry run novapcsuite backup create

# Show help
poetry run novapcsuite --help
```

#### Graphical Interface

```bash
# Launch GUI application
poetry run novapcsuite-gui
```

## Architecture Overview

NovaPcSuite is built around a modular Python architecture:

- **nova/cli.py**: Command-line interface
- **nova/adb/**: ADB client wrapper and device communication  
- **nova/backup/**: Backup and restore functionality
  - `scanner.py`: File scanning and categorization
  - `executor.py`: Backup execution engine
  - `restore.py`: Restore functionality
  - `manifest.py`: Backup metadata models
  - `storage.py`: Backup storage layout
- **nova/gui/**: GTK4 graphical interface

## Backup Features

### Current (v0.1.0)

- ✅ ADB device detection and info collection
- ✅ File scanning with whitelist support
- ✅ Full backup with SHA256 verification
- ✅ JSON manifest generation
- ✅ CLI interface for device management
- ✅ Basic GTK4 GUI framework
- ✅ Backup storage organization (backups/{device}/{timestamp}/)

### Planned (Future Releases)

- 🔄 Incremental backup support
- 🔄 Progress bars and detailed statistics
- 🔄 Backup verification and integrity checks
- 🔄 Advanced GUI file browser
- 🔄 Restore wizard interface
- 🔄 Backup comparison and diff tools
- 🔄 Encryption and manifest signing
- 🔄 Scheduled backup automation
- 🔄 Custom themes and icons

## Development

### Project Structure

```
nova/
├── __init__.py
├── cli.py              # CLI entry point
├── adb/               # ADB wrapper
│   ├── client.py      # ADB client implementation
│   └── device.py      # Device info collection
├── backup/            # Backup functionality
│   ├── scanner.py     # File scanning
│   ├── executor.py    # Backup execution
│   ├── restore.py     # Restore operations
│   ├── manifest.py    # Data models
│   └── storage.py     # Storage layout
└── gui/               # GTK4 interface
    └── app.py         # Main GUI application
```

### Building and Testing

```bash
# Run tests
poetry run pytest

# Code formatting
poetry run black .
poetry run isort .

# Type checking
poetry run mypy nova/
```

## Configuration

Backup storage is organized as follows:

```
~/NovaPcSuite/backups/
├── {device-id}/
│   ├── 20240101_120000/
│   │   ├── manifest.json
│   │   └── files/
│   │       └── [device files...]
│   └── 20240102_130000/
│       ├── manifest.json
│       └── files/
```

## Migration from Rust

This project previously used a Rust-based architecture (see closed PR #72). The decision to pivot to Python was made to:

- Improve development velocity and maintainability
- Leverage Python's rich ecosystem for GUI and data processing
- Provide better cross-platform compatibility
- Enable easier contribution from the community

The core functionality and architecture concepts remain the same, but the implementation is now Python-based with modern tooling.

## Contributing

We welcome contributions! Please see:

- Create issues for bug reports and feature requests
- Submit pull requests for improvements
- Follow the existing code style (Black, isort)
- Add tests for new functionality

## License

MIT License - see [LICENSE](LICENSE) for details.

## Getting Help

- 📖 [Documentation](docs/)
- 🐛 [Issues](https://github.com/linuxiano85/NovaPcSuite/issues)
- 💬 [Discussions](https://github.com/linuxiano85/NovaPcSuite/discussions)
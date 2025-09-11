# Changelog

## v0.1.0 - Python Bootstrap (2024-09-11)

### Major Changes

- **Architecture Pivot**: Canceled Rust implementation (PR #72) and pivoted to Python + GTK4
- **New Python Implementation**: Complete rewrite with modular Python architecture
- **CLI Interface**: Added comprehensive command-line interface with device management
- **Backup Engine**: Implemented full backup functionality with manifest tracking
- **GUI Framework**: Added GTK4-based graphical interface foundation

### Added

- Python package structure with Poetry support
- ADB client wrapper for device communication
- Device information collection and display
- File scanning with categorization by type
- Backup execution with SHA256 verification
- JSON manifest generation and storage
- Organized backup storage layout (backups/{device}/{timestamp}/)
- CLI commands: `device info`, `backup create`
- GUI launcher with device info and backup controls
- Comprehensive documentation and setup instructions

### Removed

- Rust workspace and all Rust source code
- Cargo.toml and related Rust build files
- Plugin system (will be reimplemented in Python if needed)

### Technical Details

- Built with Python 3.8+ and Poetry dependency management
- Uses Click for CLI, Rich for output formatting, Pydantic for data models
- Optional GTK4 GUI support (requires system libraries)
- Cross-platform ADB integration with timeout handling
- Modular architecture for easy extension

### Migration Notes

This release represents a complete architectural pivot from Rust to Python. The decision was made to improve development velocity, leverage Python's ecosystem, and provide better cross-platform compatibility. Users of the previous Rust version should migrate to this Python implementation.
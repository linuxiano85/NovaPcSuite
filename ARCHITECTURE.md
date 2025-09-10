# NovaPcSuite Architecture

This document describes the architecture, design patterns, and data flow of NovaPcSuite.

## Overview

NovaPcSuite is designed as a modular Rust workspace with clear separation of concerns. The architecture follows these principles:

- **Modularity**: Each crate has a specific responsibility
- **Safety**: Rust's type system prevents common errors
- **Performance**: Zero-cost abstractions and efficient algorithms
- **Extensibility**: Plugin-style architecture for new device types and formats
- **Cross-platform**: Core logic is platform-agnostic

## Crate Architecture

```
NovaPcSuite/
├── nova-core/          # Foundation crate
├── nova-adb/           # ADB communication
├── nova-mtp/           # MTP communication  
├── nova-backup/        # Backup logic
├── nova-formats/       # Data format handling
├── nova-cli/           # Command-line interface
└── nova-ui/            # GUI application
```

### Dependency Graph

```
nova-cli ────┐
             ├─→ nova-backup ────┐
nova-ui ─────┘                  ├─→ nova-adb ────┐
                                │                 ├─→ nova-core
             ┌─→ nova-formats ───┘                 │
             │                                    │
             └─→ nova-mtp ─────────────────────────┘
```

## Core Crates

### nova-core

**Purpose**: Foundation layer providing common functionality

**Responsibilities**:
- Error types and result handling
- Device abstraction and capability detection
- Configuration management
- Logging infrastructure
- Cross-crate data structures

**Key Types**:
```rust
pub struct Device {
    pub info: DeviceInfo,
    pub capabilities: DeviceCapabilities,
    pub connection_type: ConnectionType,
}

pub enum Error {
    Io(std::io::Error),
    Device(String),
    Adb(String),
    // ...
}
```

### nova-adb  

**Purpose**: Safe wrapper around Android Debug Bridge

**Responsibilities**:
- Device discovery via `adb devices`
- Property querying (`getprop`)
- Shell command execution with validation
- File transfer operations
- Root access detection

**Security Features**:
- Command validation and sanitization
- No arbitrary shell execution
- Timeout handling for long operations

### nova-mtp

**Purpose**: MTP (Media Transfer Protocol) abstraction

**Responsibilities**:
- MTP device enumeration
- File system navigation
- Metadata extraction
- Alternative to ADB for file access

**Current Status**: Placeholder implementation for future libmtp integration

### nova-backup

**Purpose**: Backup planning and execution logic

**Responsibilities**:
- File system scanning with concurrency
- File categorization by type/extension
- Duplicate detection algorithms
- Backup plan generation
- Progress reporting

**Key Algorithms**:

#### Duplicate Detection
1. **Size Grouping**: Group files by size
2. **Candidate Hashing**: Hash first 64KB for large files
3. **Full Verification**: Complete hash if needed
4. **Result Grouping**: Create DuplicateGroup objects

#### File Scanning
1. **Discovery**: Find all files via ADB shell
2. **Metadata**: Extract size, timestamps, permissions
3. **Categorization**: Classify by extension and MIME type
4. **Hash Computation**: Optional SHA256 for verification

### nova-formats

**Purpose**: Data format handling and export

**Responsibilities**:
- Contact data structures
- Export format implementations (vCard, CSV)
- Trait-based extensible design
- Future: SMS, call logs, app data formats

**Design Patterns**:
```rust
#[async_trait]
pub trait ContactSource {
    async fn fetch_contacts(&self, device: &Device) -> Result<Vec<Contact>>;
}

#[async_trait] 
pub trait ContactExporter {
    async fn export_contacts(&self, contacts: &[Contact], path: &PathBuf) -> Result<()>;
}
```

## User Interfaces

### nova-cli

**Purpose**: Command-line interface for automation

**Architecture**:
- Uses `clap` for argument parsing
- Async/await for non-blocking operations
- Structured logging output
- JSON output for scripting integration

**Commands**:
- `devices` - List connected devices
- `scan` - Analyze device files
- `plan` - Generate backup plans
- `contacts export` - Export contact data

### nova-ui

**Purpose**: Desktop GUI application

**Technology Stack**:
- **Tauri**: Rust backend with web frontend
- **HTML/CSS/JS**: Minimal responsive UI
- **Native APIs**: File dialogs, system integration

**Communication**:
- Frontend ↔ Backend via Tauri commands
- Async progress updates
- State management in Rust backend

## Data Flow

### Device Scanning Workflow

```
1. Device Discovery
   ├─ ADB devices list
   ├─ Property querying  
   └─ Capability detection

2. File System Scan
   ├─ Path enumeration
   ├─ Metadata extraction
   ├─ Categorization
   └─ Hash computation (optional)

3. Analysis
   ├─ Duplicate detection
   ├─ Size calculations
   └─ Category summaries

4. Output
   ├─ JSON results
   ├─ Backup plans
   └─ Progress reports
```

### Backup Planning Workflow

```
1. Input Processing
   ├─ Include/exclude paths
   ├─ Compression preferences
   └─ Priority settings

2. File Filtering
   ├─ Path matching
   ├─ Size thresholds
   └─ Type filtering

3. Plan Generation
   ├─ Priority assignment
   ├─ Compression analysis
   ├─ Size estimation
   └─ Entry ordering

4. Serialization
   └─ JSON plan output
```

## Error Handling Strategy

### Error Types
- **User Errors**: Invalid input, missing devices
- **System Errors**: IO failures, permission issues  
- **Device Errors**: ADB failures, disconnections
- **Parse Errors**: Malformed data, version mismatches

### Error Propagation
```rust
// Consistent error handling across crates
pub type Result<T> = std::result::Result<T, Error>;

// Error context preservation
.map_err(|e| Error::Device(format!("Failed to scan: {}", e)))
```

### User Experience
- Clear error messages
- Graceful degradation
- Retry mechanisms for transient failures
- Progress updates during long operations

## Configuration Management

### Configuration Hierarchy
1. **Defaults**: Hard-coded sensible defaults
2. **System Config**: `/etc/novapcsuite/config.toml`
3. **User Config**: `~/.config/novapcsuite/config.toml`
4. **Environment**: Environment variable overrides
5. **CLI Args**: Command-line parameter overrides

### Configuration Structure
```toml
[backup]
default_backup_dir = "/home/user/Backups"
compression_enabled = true
max_parallel_operations = 4

[ui]
theme = "dark"
auto_scan_on_connect = true

[logging]
level = "info"
file_enabled = true
```

## Concurrency Model

### Async/Await Usage
- **ADB Operations**: All device communication is async
- **File System Scanning**: Concurrent directory traversal
- **Progress Updates**: Non-blocking progress reporting
- **UI Updates**: Async Tauri commands

### Thread Safety
- **Shared State**: `Arc<Mutex<T>>` for UI state
- **Channels**: `mpsc` for progress communication
- **Immutable Data**: Prefer immutable structures

## Security Considerations

### Input Validation
- **Path Sanitization**: Prevent directory traversal
- **Command Validation**: Whitelist ADB commands
- **Size Limits**: Prevent resource exhaustion

### Privilege Management
- **No Root Required**: Normal user permissions
- **ADB Delegation**: Leverage ADB's security model
- **File Permissions**: Respect system permissions

## Testing Strategy

### Unit Tests
- **Core Logic**: Business logic in isolation
- **Error Cases**: Comprehensive error handling
- **Edge Cases**: Boundary conditions

### Integration Tests
- **Device Communication**: Mock ADB responses
- **File Operations**: Temporary filesystem
- **UI Commands**: Tauri test harness

### Test Data
- **Mock Devices**: Simulated device responses
- **Sample Files**: Representative file structures
- **Error Scenarios**: Failure condition simulation

## Performance Characteristics

### Scanning Performance
- **Concurrency**: Parallel file processing
- **Streaming**: Memory-efficient large file handling
- **Caching**: Avoid redundant operations

### Memory Usage
- **Lazy Loading**: On-demand data loading
- **Streaming Parsers**: Avoid loading entire files
- **Resource Cleanup**: Explicit resource management

### Scalability
- **Large Devices**: Handle thousands of files
- **Multiple Devices**: Concurrent device management
- **Background Processing**: Non-blocking operations

## Extension Points

### Adding New Device Types
1. Implement `DeviceCapabilities` for new protocol
2. Add protocol-specific client crate
3. Update device detection logic
4. Extend CLI/UI for new features

### Adding Export Formats
1. Implement `ContactExporter` trait
2. Add format-specific logic
3. Register in CLI argument parsing
4. Update UI format selection

### Adding Backup Sources
1. Extend `FileInfo` for new metadata
2. Implement scanning logic
3. Add categorization rules
4. Update backup planning

## Future Architecture Considerations

### Plugin System
- **Dynamic Loading**: Runtime plugin discovery
- **API Stability**: Versioned plugin interfaces
- **Sandboxing**: Isolated plugin execution

### Distributed Operations
- **Network Backup**: Remote backup destinations
- **Cloud Integration**: Cloud storage providers
- **Synchronization**: Multi-device coordination

### Advanced UI Features
- **Real-time Updates**: Live device monitoring
- **Batch Operations**: Multi-device management
- **Visualization**: Progress and statistics charts
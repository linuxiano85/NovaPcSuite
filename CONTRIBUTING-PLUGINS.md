# Plugin Development Guide for NovaPcSuite

This guide covers developing plugins for NovaPcSuite using the WASM-based plugin architecture.

## Table of Contents

1. [Plugin Architecture Overview](#plugin-architecture-overview)
2. [Development Environment Setup](#development-environment-setup)
3. [Plugin Lifecycle](#plugin-lifecycle)
4. [Host Functions API](#host-functions-api)
5. [Event System](#event-system)
6. [Security Model](#security-model)
7. [Example Plugins](#example-plugins)
8. [Best Practices](#best-practices)
9. [Testing and Debugging](#testing-and-debugging)
10. [Publishing Guidelines](#publishing-guidelines)

## Plugin Architecture Overview

NovaPcSuite uses a WASM-based plugin system that provides:

- **Security**: Sandboxed execution environment
- **Performance**: Near-native execution speed
- **Portability**: Plugins work across different platforms
- **Language Support**: Write plugins in Rust, C/C++, AssemblyScript, or other WASM-compatible languages

### Core Components

```rust
// Plugin runtime manages WASM modules
pub struct WasmRuntime {
    engine: Engine,
    plugins: HashMap<String, PluginInfo>,
}

// Host functions available to plugins
impl HostFunctions {
    pub fn plugin_log(level: i32, message: &str) -> Result<()>;
    pub fn read_file(path: &str) -> Result<Vec<u8>>;
    pub fn send_event(event: &PlatformEvent) -> Result<()>;
}
```

## Development Environment Setup

### Prerequisites

1. **Rust with WASM target**:
```bash
rustup target add wasm32-wasi
```

2. **WASM tools**:
```bash
cargo install wasm-pack
cargo install wabt  # WebAssembly Binary Toolkit
```

3. **NovaPcSuite SDK** (coming in v0.2.0):
```bash
cargo install nova-pc-suite-sdk
```

### Project Structure

```
my-plugin/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   └── plugin.rs
├── plugin.toml      # Plugin metadata
├── README.md
└── tests/
    └── integration.rs
```

### Cargo.toml Configuration

```toml
[package]
name = "my-backup-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nova-pc-suite-plugin-api = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = "s"  # Optimize for size
lto = true       # Link-time optimization
```

## Plugin Lifecycle

### 1. Plugin Metadata

Every plugin must include a `plugin.toml` file:

```toml
[plugin]
name = "my-backup-plugin"
version = "0.1.0"
description = "Custom backup processing plugin"
author = "Your Name <your.email@example.com>"
license = "MIT"
nova_pc_suite_version = ">=0.1.0"

[permissions]
read_files = true
write_files = false
network_access = false
system_commands = false

[exports]
functions = ["on_backup_start", "process_file", "on_backup_complete"]

[metadata]
category = "backup-processor"
tags = ["backup", "processing", "custom"]
```

### 2. Plugin Interface

Plugins must implement the standard interface:

```rust
use nova_pc_suite_plugin_api::*;

#[no_mangle]
pub extern "C" fn plugin_init() -> i32 {
    // Plugin initialization
    host_log(LogLevel::Info, "Plugin initialized");
    0
}

#[no_mangle]
pub extern "C" fn on_backup_start(backup_id: *const u8, backup_id_len: usize) -> i32 {
    let backup_id = unsafe { 
        std::str::from_utf8(std::slice::from_raw_parts(backup_id, backup_id_len)).unwrap()
    };
    
    host_log(LogLevel::Info, &format!("Backup started: {}", backup_id));
    0
}

#[no_mangle]
pub extern "C" fn process_file(file_path: *const u8, file_path_len: usize) -> i32 {
    // Custom file processing logic
    0
}

#[no_mangle]
pub extern "C" fn plugin_cleanup() -> i32 {
    // Plugin cleanup
    0
}
```

### 3. Build Process

```bash
# Build the WASM module
cargo build --target wasm32-wasi --release

# Optimize the WASM binary
wasm-opt -Oz target/wasm32-wasi/release/my_backup_plugin.wasm \
         -o my_backup_plugin_optimized.wasm

# Validate the module
wasm-validate my_backup_plugin_optimized.wasm
```

## Host Functions API

### Logging

```rust
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

#[link(wasm_import_module = "host")]
extern "C" {
    fn host_log(level: i32, message_ptr: *const u8, message_len: usize);
}

// Helper function
pub fn log(level: LogLevel, message: &str) {
    unsafe {
        host_log(level as i32, message.as_ptr(), message.len());
    }
}
```

### File Operations

```rust
#[link(wasm_import_module = "host")]
extern "C" {
    fn host_read_file(path_ptr: *const u8, path_len: usize) -> i32;
    fn host_write_file(path_ptr: *const u8, path_len: usize, data_ptr: *const u8, data_len: usize) -> i32;
}

pub fn read_file(path: &str) -> Result<Vec<u8>, PluginError> {
    // Implementation details...
}
```

### Event System

```rust
#[link(wasm_import_module = "host")]
extern "C" {
    fn host_send_event(event_ptr: *const u8, event_len: usize) -> i32;
    fn host_subscribe_events(event_type: i32) -> i32;
}

pub fn send_custom_event(event_data: &CustomEvent) -> Result<(), PluginError> {
    let json = serde_json::to_string(event_data)?;
    unsafe {
        let result = host_send_event(json.as_ptr(), json.len());
        if result == 0 { Ok(()) } else { Err(PluginError::HostFunction) }
    }
}
```

## Event System

### Platform Events

Plugins can subscribe to and emit platform events:

```rust
#[derive(Serialize, Deserialize)]
pub enum PlatformEvent {
    BackupStarted { backup_id: String, source_path: String },
    FileProcessing { file_path: String, progress: f64 },
    ChunkCreated { chunk_id: String, size: u64 },
    BackupCompleted { backup_id: String, file_count: usize },
    // Custom events
    CustomEvent(serde_json::Value),
}
```

### Event Handling

```rust
#[no_mangle]
pub extern "C" fn handle_event(event_ptr: *const u8, event_len: usize) -> i32 {
    let event_json = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(event_ptr, event_len)).unwrap()
    };
    
    match serde_json::from_str::<PlatformEvent>(event_json) {
        Ok(PlatformEvent::FileProcessing { file_path, progress }) => {
            // Custom processing logic
            if file_path.ends_with(".log") {
                // Special handling for log files
                compress_log_file(&file_path);
            }
        }
        Ok(PlatformEvent::BackupCompleted { backup_id, file_count }) => {
            // Send notification or update external system
            send_completion_notification(&backup_id, file_count);
        }
        _ => {}
    }
    
    0
}
```

## Security Model

### Permissions System

Plugins request permissions in `plugin.toml`:

```toml
[permissions]
read_files = true           # Read files from source
write_files = false         # Write files to destination
network_access = false      # HTTP/HTTPS requests
system_commands = false     # Execute system commands
crypto_operations = false   # Cryptographic operations
```

### Sandboxing

- **Memory Isolation**: Each plugin runs in isolated memory
- **File System Access**: Limited to permitted paths
- **Network Restrictions**: Controlled network access
- **Resource Limits**: CPU time and memory quotas
- **No Direct System Calls**: All system access through host functions

### Code Signing (Planned v0.3.0)

```toml
[signing]
required = true
public_key = "ed25519:AAAA..."
signature = "BBBB..."
```

## Example Plugins

### 1. Log File Compressor

```rust
use nova_pc_suite_plugin_api::*;

#[no_mangle]
pub extern "C" fn process_file(file_path: *const u8, file_path_len: usize) -> i32 {
    let file_path = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(file_path, file_path_len)).unwrap()
    };
    
    if file_path.ends_with(".log") {
        match compress_log_file(file_path) {
            Ok(_) => {
                log(LogLevel::Info, &format!("Compressed log file: {}", file_path));
                0
            }
            Err(e) => {
                log(LogLevel::Error, &format!("Failed to compress {}: {}", file_path, e));
                1
            }
        }
    } else {
        0 // Skip non-log files
    }
}

fn compress_log_file(path: &str) -> Result<(), PluginError> {
    // Read file content
    let content = read_file(path)?;
    
    // Compress using a simple algorithm
    let compressed = simple_compress(&content);
    
    // Write compressed version
    let compressed_path = format!("{}.gz", path);
    write_file(&compressed_path, &compressed)?;
    
    Ok(())
}
```

### 2. Database Backup Plugin

```rust
#[no_mangle]
pub extern "C" fn on_backup_start(backup_id: *const u8, backup_id_len: usize) -> i32 {
    let backup_id = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(backup_id, backup_id_len)).unwrap()
    };
    
    // Create database dump before backup
    match create_database_dump() {
        Ok(dump_path) => {
            log(LogLevel::Info, &format!("Database dump created: {}", dump_path));
            
            // Notify the system about the new file to backup
            let event = PlatformEvent::CustomEvent(json!({
                "type": "database_dump_created",
                "backup_id": backup_id,
                "dump_path": dump_path
            }));
            
            send_event(&event).unwrap();
            0
        }
        Err(e) => {
            log(LogLevel::Error, &format!("Database dump failed: {}", e));
            1
        }
    }
}

fn create_database_dump() -> Result<String, PluginError> {
    // Execute database dump command through host
    let dump_path = "/tmp/database_backup.sql";
    
    // This would require system_commands permission
    execute_command(&["pg_dump", "mydb", "-f", dump_path])?;
    
    Ok(dump_path.to_string())
}
```

### 3. Progress Reporter Plugin

```rust
use std::collections::HashMap;

static mut PROGRESS_TRACKER: Option<HashMap<String, ProgressInfo>> = None;

#[derive(Default)]
struct ProgressInfo {
    total_files: usize,
    processed_files: usize,
    total_size: u64,
    processed_size: u64,
}

#[no_mangle]
pub extern "C" fn on_backup_start(backup_id: *const u8, backup_id_len: usize) -> i32 {
    let backup_id = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(backup_id, backup_id_len)).unwrap()
    };
    
    unsafe {
        if PROGRESS_TRACKER.is_none() {
            PROGRESS_TRACKER = Some(HashMap::new());
        }
        
        PROGRESS_TRACKER.as_mut().unwrap().insert(
            backup_id.to_string(),
            ProgressInfo::default()
        );
    }
    
    0
}

#[no_mangle]
pub extern "C" fn handle_event(event_ptr: *const u8, event_len: usize) -> i32 {
    let event_json = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(event_ptr, event_len)).unwrap()
    };
    
    if let Ok(event) = serde_json::from_str::<PlatformEvent>(event_json) {
        match event {
            PlatformEvent::FileProcessing { backup_id, file_path, progress } => {
                update_progress(&backup_id, &file_path, progress);
            }
            _ => {}
        }
    }
    
    0
}

fn update_progress(backup_id: &str, file_path: &str, progress: f64) {
    // Send progress update to external monitoring system
    let notification = json!({
        "backup_id": backup_id,
        "current_file": file_path,
        "overall_progress": progress,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    // Send to webhook or message queue
    send_http_notification(&notification);
}
```

## Best Practices

### 1. Error Handling

```rust
#[derive(Debug)]
pub enum PluginError {
    HostFunction,
    InvalidInput,
    ProcessingFailed(String),
}

// Always return error codes from extern "C" functions
#[no_mangle]
pub extern "C" fn process_file(/* ... */) -> i32 {
    match internal_process_file(/* ... */) {
        Ok(_) => 0,
        Err(PluginError::HostFunction) => 1,
        Err(PluginError::InvalidInput) => 2,
        Err(PluginError::ProcessingFailed(_)) => 3,
    }
}
```

### 2. Memory Management

```rust
// Use Vec<u8> for binary data
// Use String for text data
// Avoid raw pointers except at FFI boundary

#[no_mangle]
pub extern "C" fn get_result_data(len_out: *mut usize) -> *mut u8 {
    let data = vec![1, 2, 3, 4, 5];
    unsafe {
        *len_out = data.len();
    }
    
    // Convert to raw pointer and leak (caller must free)
    let ptr = data.as_ptr() as *mut u8;
    std::mem::forget(data);
    ptr
}

#[no_mangle]
pub extern "C" fn free_data(ptr: *mut u8, len: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, len, len);
        // Vec will be dropped and memory freed
    }
}
```

### 3. Configuration

```rust
#[derive(Deserialize)]
struct PluginConfig {
    compression_level: u8,
    max_file_size: u64,
    excluded_extensions: Vec<String>,
}

#[no_mangle]
pub extern "C" fn plugin_configure(config_ptr: *const u8, config_len: usize) -> i32 {
    let config_json = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(config_ptr, config_len)).unwrap()
    };
    
    match serde_json::from_str::<PluginConfig>(config_json) {
        Ok(config) => {
            // Store configuration globally
            unsafe { PLUGIN_CONFIG = Some(config); }
            0
        }
        Err(_) => 1
    }
}
```

### 4. Performance Optimization

```rust
// Use efficient algorithms
// Minimize memory allocations
// Cache expensive computations
// Use streaming for large files

#[no_mangle]
pub extern "C" fn process_large_file(file_path: *const u8, file_path_len: usize) -> i32 {
    let file_path = /* convert from raw pointer */;
    
    // Process file in chunks to avoid loading entire file into memory
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB chunks
    let mut offset = 0;
    
    loop {
        match read_file_chunk(file_path, offset, &mut buffer) {
            Ok(0) => break, // EOF
            Ok(bytes_read) => {
                process_chunk(&buffer[..bytes_read]);
                offset += bytes_read as u64;
            }
            Err(_) => return 1,
        }
    }
    
    0
}
```

## Testing and Debugging

### 1. Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compression() {
        let input = b"Hello, world!".repeat(100);
        let compressed = simple_compress(&input);
        assert!(compressed.len() < input.len());
        
        let decompressed = simple_decompress(&compressed).unwrap();
        assert_eq!(input, decompressed);
    }
}
```

### 2. Integration Testing

```rust
// tests/integration.rs
use nova_pc_suite_test_framework::*;

#[test]
fn test_plugin_integration() {
    let runtime = WasmRuntime::new().unwrap();
    let plugin_id = runtime.load_plugin("target/wasm32-wasi/release/my_plugin.wasm").unwrap();
    
    // Test plugin initialization
    runtime.call_function(&plugin_id, "plugin_init", &[]).unwrap();
    
    // Test file processing
    let result = runtime.call_function(&plugin_id, "process_file", &[
        Value::String("/test/file.log".to_string())
    ]).unwrap();
    
    assert_eq!(result, Value::I32(0));
}
```

### 3. Debugging

```rust
// Enable debug logging
log(LogLevel::Debug, &format!("Processing file: {}", file_path));
log(LogLevel::Debug, &format!("Buffer size: {}", buffer.len()));

// Use assertions in debug builds
debug_assert!(buffer.len() > 0);
debug_assert!(file_path.is_ascii());
```

## Publishing Guidelines

### 1. Plugin Registry (Planned v0.4.0)

```bash
# Publish to official registry
nova-plugin publish my-backup-plugin-0.1.0.wasm

# Install from registry
nova-plugin install log-compressor
```

### 2. Versioning

- Follow semantic versioning (SemVer)
- Test compatibility with NovaPcSuite versions
- Provide migration guides for breaking changes

### 3. Documentation

- Include comprehensive README
- Document all configuration options
- Provide usage examples
- Include troubleshooting guide

### 4. Security Review

- No hardcoded secrets or credentials
- Minimal permission requests
- Input validation and sanitization
- Error handling without information disclosure

---

This guide provides the foundation for developing secure, efficient plugins for NovaPcSuite. As the plugin system evolves, additional features and APIs will be added to support more advanced use cases.
# NovaPcSuite Backup Engine

A high-performance backup engine with chunk-based deduplication, Merkle hashing, and comprehensive progress reporting.

## Features

### Core Backup Engine
- **Chunk-based Deduplication**: Files are split into 64KB chunks using BLAKE3 hashing for content-addressed storage
- **Merkle Tree Hashing**: Per-file integrity verification using Merkle tree structure
- **Snapshot Manifest v2**: JSON-based backup manifests with comprehensive metadata
- **Progress Events**: Real-time progress broadcasting to plugins and UI
- **Content-Addressed Storage**: Chunks stored under `backup_root/chunks/aa/` structure (256 subdirectories)

### Key Components

#### 1. Chunk Store (`internal/chunk`)
- **BLAKE3 Hashing**: 64-character hex hashes for maximum collision resistance
- **Automatic Deduplication**: Identical chunks are stored only once
- **Hierarchical Storage**: Chunks organized in 256 subdirectories (00-ff) based on hash prefix
- **File Reconstruction**: Complete file restoration from chunk sequences

#### 2. Snapshot Management (`internal/manifest`) 
- **Manifest v2 Format**: Comprehensive backup metadata in JSON format
- **File Metadata**: Permissions, timestamps, size, and chunk references
- **Snapshot Versioning**: UUID-based snapshot identification
- **Deduplication Statistics**: Track storage efficiency and chunk reuse

#### 3. Progress Broadcasting (`internal/progress`)
- **Event-Driven Architecture**: Real-time progress events for UI integration
- **Multiple Handlers**: Support for console output and custom plugin handlers
- **Progress Tracking**: Speed calculation, ETA estimation, and completion status
- **Error Reporting**: Comprehensive error event handling

#### 4. Backup Engine (`internal/backup`)
- **Three-Phase Operations**: Scan → Plan → Run workflow
- **File Tree Walking**: Recursive directory processing with progress tracking
- **Restore Functionality**: Single file and full snapshot restoration
- **Metadata Preservation**: File permissions and timestamps maintained

## Usage

### Command Line Interface

```bash
# Scan a directory (analyze without backup)
./NovaPcSuite scan /path/to/data

# Create backup plan (analyze with deduplication stats)
./NovaPcSuite plan /path/to/data

# Execute backup
./NovaPcSuite run /path/to/data
```

### Programmatic Usage

```go
package main

import (
    "github.com/linuxiano85/NovaPcSuite/internal/backup"
    "github.com/linuxiano85/NovaPcSuite/internal/progress"
)

func main() {
    // Create backup engine
    engine := backup.NewEngine("./backups")
    
    // Add custom progress handler
    engine.AddProgressHandler(func(event *progress.Event) {
        // Handle progress events for UI updates
        fmt.Printf("Progress: %.1f%% - %s\n", event.Progress*100, event.Message)
    })
    
    // Execute backup
    err := engine.Run("/path/to/data")
    if err != nil {
        log.Fatal(err)
    }
}
```

## Architecture

### Storage Structure
```
backup_root/
├── chunks/
│   ├── 00/
│   │   └── 00a1b2c3... (BLAKE3 hash files)
│   ├── 01/
│   ├── ...
│   └── ff/
└── manifests/
    ├── latest.json
    └── [uuid].json (snapshot manifests)
```

### Manifest Format (v2)
```json
{
  "id": "uuid",
  "version": "2.0",
  "timestamp": "2025-09-10T19:29:48Z",
  "source_path": "/path/to/data",
  "files": {
    "file.txt": {
      "path": "file.txt",
      "size": 1024,
      "mod_time": "2025-09-10T19:29:48Z",
      "chunks": [
        {
          "hash": "blake3-hash-64-chars",
          "size": 1024,
          "path": "backup_root/chunks/ab/ab..."
        }
      ],
      "file_hash": "merkle-root-hash",
      "permissions": 644,
      "is_dir": false
    }
  },
  "total_size": 1024,
  "total_files": 1,
  "unique_chunks": 1,
  "metadata": {
    "deduplication_ratio": 0.95
  }
}
```

### Progress Events
- `scan_start`, `scan_progress`, `scan_complete`
- `plan_start`, `plan_progress`, `plan_complete`  
- `backup_start`, `backup_progress`, `backup_complete`
- `error`, `info`

## Performance Features

- **Deduplication**: Eliminates duplicate data across files and backups
- **Incremental Backups**: Only new/changed chunks are stored
- **Parallel Processing**: Concurrent chunk processing and event handling
- **Memory Efficient**: Streaming file processing for large files
- **Fast Hashing**: BLAKE3 provides high-speed cryptographic hashing

## Security & Integrity

- **BLAKE3 Cryptographic Hashing**: Collision-resistant content addressing
- **Merkle Tree Verification**: File-level integrity checking
- **Immutable Chunks**: Content-addressed storage prevents tampering
- **Metadata Preservation**: Complete file attribute restoration

## Testing

The backup engine includes comprehensive tests covering:

```bash
# Run all tests
go test ./... -v

# Test specific components
go test ./internal/chunk -v      # Chunk store tests
go test ./internal/backup -v     # Backup engine tests
```

## Future Enhancements

- **Encryption**: AES-256 encryption for chunk and manifest data
- **Compression**: Optional chunk compression for space efficiency
- **Network Backup**: Remote backup destinations
- **Advanced Analytics**: Backup size trends and deduplication analysis
- **Web UI**: Browser-based management interface
- **Plugin System**: Extensible architecture for custom functionality

## Dependencies

- `github.com/zeebo/blake3`: High-performance BLAKE3 hashing
- `github.com/google/uuid`: UUID generation for snapshots
- Go 1.21+ standard library

## License

MIT License - see LICENSE file for details.
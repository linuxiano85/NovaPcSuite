# NovaPcSuite

A comprehensive backup and restore system with chunk-based deduplication, integrity verification, and data recovery capabilities.

## Features

### Core Backup & Restore
- **Chunk-based deduplication**: Efficient storage using content-addressed chunks with BLAKE3 hashing
- **Manifest v2 format**: JSON-based snapshots with Merkle tree integrity verification
- **Integrity verification**: Multi-level verification with chunk hashes, Merkle roots, and file hashes
- **Incremental backups**: Automatic deduplication across snapshots reduces storage requirements

### Restore Engine
- **Full file reconstruction**: Reassemble files from chunks with integrity verification
- **Dry-run mode**: Generate detailed restore plans in JSON format without writing files
- **Path mapping**: Remap file paths during restore using TOML configuration
- **Conflict resolution**: Handle existing files with skip, overwrite, or rename policies
- **Progress reporting**: Detailed logging and statistics for restore operations

### Data Recovery (Phase 1)
- **Snapshot salvage**: Rebuild manifest index from corrupted or partial manifests
- **Orphan chunk detection**: Identify and optionally clean up unreferenced chunks
- **Validation**: Verify snapshot integrity and detect corruption
- **Recovery reports**: Detailed JSON reports for analysis and auditing

### Scheduling & Automation
- **systemd integration**: Generate and install systemd service/timer units
- **Flexible scheduling**: Support for daily, weekly, and cron-style patterns
- **User and system modes**: Install schedules for individual users or system-wide
- **CLI management**: Create, list, enable/disable, and remove schedules

### CLI Interface
- **Comprehensive commands**: Full-featured command-line interface
- **JSON output**: Machine-readable output for automation and scripting
- **Logging control**: Configurable log levels and structured JSON logging
- **Feature flags**: Optional recovery module (enabled by default)

## Installation

```bash
# Clone the repository
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite

# Build the project
cargo build --release

# Install globally (optional)
cargo install --path .
```

## Quick Start

### Basic Backup

```bash
# Create your first backup
nova-cli backup run --source /home/user/documents --name "documents-$(date +%Y%m%d)"

# List all snapshots
nova-cli backup list

# Show details of a specific snapshot
nova-cli backup show <snapshot-id>
```

### Basic Restore

```bash
# Create a restore plan (dry-run)
nova-cli restore plan <snapshot-id> --target /tmp/restore

# Perform the actual restore
nova-cli restore run <snapshot-id> --target /tmp/restore --on-conflict rename
```

### Scheduled Backups

```bash
# Create a daily backup schedule
nova-cli schedule add \
    --name "daily-documents" \
    --pattern "daily@02:00" \
    --source /home/user/documents \
    --install

# List all schedules
nova-cli schedule list

# Enable/disable a schedule
nova-cli schedule toggle <schedule-id> --enable
```

## Detailed Usage

### Backup Operations

#### Creating Backups

```bash
# Basic backup
nova-cli backup run --source /path/to/source --name "my-backup"

# Advanced backup with options
nova-cli backup run \
    --source /home/user \
    --name "user-backup-$(date +%Y%m%d)" \
    --chunk-size 2097152 \
    --follow-symlinks \
    --exclude "*.tmp" \
    --exclude ".cache" \
    --max-file-size 1073741824
```

#### Listing and Inspecting Snapshots

```bash
# List snapshots in table format
nova-cli backup list

# List snapshots in JSON format
nova-cli backup list --format json

# Show snapshot details
nova-cli backup show <snapshot-id>
```

### Restore Operations

#### Planning Restores

```bash
# Create a detailed restore plan
nova-cli restore plan <snapshot-id> --target /restore/path --format json

# Plan with path mapping
nova-cli restore plan <snapshot-id> --target /new/location --map mapping.toml
```

#### Executing Restores

```bash
# Simple restore
nova-cli restore run <snapshot-id> --target /restore/path

# Restore with conflict handling
nova-cli restore run <snapshot-id> --target /restore/path --on-conflict overwrite

# Dry-run restore (show what would be done)
nova-cli restore run <snapshot-id> --target /restore/path --dry-run

# Restore with path mapping
nova-cli restore run <snapshot-id> --target /new/location --map mapping.toml
```

#### Path Mapping Configuration

Create a `mapping.toml` file to remap paths during restore:

```toml
"/home/olduser" = "/home/newuser"
"/var/old-app" = "/opt/new-app"
```

### Scheduling

#### Creating Schedules

```bash
# Daily backup at 2 AM
nova-cli schedule add \
    --name "daily-backup" \
    --pattern "daily@02:00" \
    --source /home/user \
    --install

# Weekly backup on Monday, Wednesday, Friday at 6 PM
nova-cli schedule add \
    --name "weekly-backup" \
    --pattern "weekly@Mon,Wed,Fri@18:00" \
    --source /important/data \
    --install

# Custom snapshot naming
nova-cli schedule add \
    --name "custom-backup" \
    --pattern "daily@03:00" \
    --source /data \
    --snapshot-name "auto-{date}-{time}" \
    --install
```

#### Managing Schedules

```bash
# List all schedules
nova-cli schedule list

# Show schedule details
nova-cli schedule show <schedule-id>

# Enable/disable schedule
nova-cli schedule toggle <schedule-id> --enable
nova-cli schedule toggle <schedule-id> --disable

# Remove schedule
nova-cli schedule remove <schedule-id>

# Install/uninstall systemd units
nova-cli schedule install <schedule-id>
nova-cli schedule uninstall <schedule-id>
```

### Data Recovery

#### Orphan Chunk Detection

```bash
# Detect orphaned chunks
nova-cli recover orphan-chunks

# Get detailed report in JSON
nova-cli recover orphan-chunks --format json

# Clean up orphaned chunks (with confirmation)
nova-cli recover orphan-chunks --cleanup

# Force cleanup without confirmation
nova-cli recover orphan-chunks --cleanup --force
```

#### Snapshot Salvage

```bash
# Salvage corrupted snapshots
nova-cli recover salvage

# Get detailed salvage report
nova-cli recover salvage --format json
```

#### Snapshot Validation

```bash
# Validate a specific snapshot
nova-cli recover validate <snapshot-id>

# Get detailed validation report
nova-cli recover validate <snapshot-id> --format json
```

## Configuration

### Environment Variables

- `NOVA_BACKUP_ROOT`: Default backup root directory (defaults to `~/.nova-backup`)

### Global Options

- `--root`: Backup root directory
- `--quiet`: Reduce output verbosity
- `--log-format`: Set log format (text or json)

### Backup Configuration

The backup engine supports various configuration options:

- **Chunk size**: Configure chunk size for deduplication (default: 1MB)
- **Exclusion patterns**: Exclude files matching glob patterns
- **File size limits**: Skip files larger than specified size
- **Symlink handling**: Choose whether to follow symbolic links

### Restore Configuration

- **Conflict policies**: Skip, overwrite, or rename conflicting files
- **Path mapping**: Remap file paths during restore
- **Integrity verification**: Verify restored files (enabled by default)
- **Permission preservation**: Preserve original file permissions

## Architecture

### Storage Format

```
backup-root/
├── chunks/           # Content-addressed chunk storage
│   ├── ab/          # First 2 chars of hash as directory
│   │   └── cdef...  # Remaining hash as filename
│   └── ...
├── manifests/        # Snapshot manifests
│   ├── uuid1.json   # Snapshot manifest files
│   └── ...
└── config/          # Configuration and schedules
    └── schedules/   # Schedule definitions
```

### Manifest Format (v2)

```json
{
  "version": 2,
  "id": "uuid",
  "created": "2024-01-01T00:00:00Z",
  "name": "backup-name",
  "source_root": "/path/to/source",
  "files": [
    {
      "path": "relative/path/to/file",
      "size": 12345,
      "modified": "2024-01-01T00:00:00Z",
      "mode": 33188,
      "chunks": ["hash1", "hash2", "..."],
      "file_hash": "blake3-hash-of-complete-file",
      "merkle_root": "merkle-root-of-chunks"
    }
  ],
  "chunk_stats": {
    "total_chunks": 100,
    "total_bytes": 1048576,
    "dedup_chunks": 20,
    "dedup_savings": 209715
  }
}
```

### Integrity Verification

The system implements multiple levels of integrity verification:

1. **Chunk-level**: BLAKE3 hash verification for each chunk
2. **File-level**: Merkle tree verification of chunk sequence
3. **Manifest-level**: JSON schema validation and consistency checks

## Examples

### Complete Backup and Restore Workflow

```bash
# 1. Create initial backup
nova-cli backup run --source /home/user/projects --name "projects-initial"

# 2. List snapshots to get ID
nova-cli backup list

# 3. Create restore plan
nova-cli restore plan abc-123-def --target /tmp/restore --format json > restore-plan.json

# 4. Review the plan
cat restore-plan.json | jq .

# 5. Execute restore
nova-cli restore run abc-123-def --target /tmp/restore

# 6. Verify restore completed successfully
diff -r /home/user/projects /tmp/restore
```

### Automated Scheduled Backups

```bash
# Set up daily backups
nova-cli schedule add \
    --name "daily-home" \
    --pattern "daily@02:00" \
    --source /home/user \
    --backup-root /backup/storage \
    --install

# Set up weekly full system backup
nova-cli schedule add \
    --name "weekly-system" \
    --pattern "weekly@Sun@03:00" \
    --source / \
    --exclude "/proc" \
    --exclude "/sys" \
    --exclude "/dev" \
    --exclude "/tmp" \
    --install --system
```

### Data Recovery Scenarios

```bash
# Scenario 1: Detect and clean orphaned chunks
nova-cli recover orphan-chunks --format json > orphan-report.json
nova-cli recover orphan-chunks --cleanup --force

# Scenario 2: Recover from corrupted manifests
nova-cli recover salvage --format json > salvage-report.json

# Scenario 3: Validate backup integrity
for snapshot in $(nova-cli backup list --format json | jq -r '.[].id'); do
    nova-cli recover validate $snapshot
done
```

## Troubleshooting

### Common Issues

1. **Permission denied errors**
   - Ensure proper file permissions for backup root
   - Use `--system` flag for system-wide schedules

2. **Chunk integrity failures**
   - Run `nova-cli recover validate <snapshot-id>` to check integrity
   - Use orphan detection to find missing chunks

3. **Schedule not running**
   - Check systemd status: `systemctl --user status nova-backup.timer`
   - Verify schedule is enabled: `nova-cli schedule list`

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug nova-cli backup run --source /path --name debug-backup
```

Use JSON logging for structured output:

```bash
nova-cli --log-format json backup run --source /path --name json-backup
```

## Development

### Building from Source

```bash
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test backup::tests

# Run with verbose output
cargo test -- --nocapture
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Roadmap

### Current Features (v0.1.0)
- ✅ Chunk-based backup with BLAKE3 hashing
- ✅ Full restore engine with conflict resolution
- ✅ Data recovery primitives (orphan detection, salvage)
- ✅ systemd scheduling integration
- ✅ Comprehensive CLI interface

### Planned Features
- 🔄 Encryption support (next priority)
- 🔄 Companion app integration (telephony, contacts, SMS)
- 🔄 Advanced data recovery (partition imaging, carving)
- 🔄 Web UI for management
- 🔄 Remote storage backends (S3, etc.)
- 🔄 Compression optimization
- 🔄 Incremental backup improvements

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/linuxiano85/NovaPcSuite/issues
- Documentation: This README and inline code documentation
- Tests: Comprehensive test suite demonstrates usage patterns
# NovaPcSuite

A powerful Android device management tool built in Rust for backup, restore, and device operations via ADB.

## Features

- **Device Information**: Collect detailed device specs and bootloader status
- **File Backup**: Smart file scanning and backup from whitelisted directories
- **APK Backup**: Extract and backup user-installed applications
- **Restore Operations**: Restore files to local directory or back to device
- **Hash Verification**: SHA256 integrity checking for all backed up files
- **Structured Manifests**: YAML and JSON backup manifests with metadata
- **Bootloader Analysis**: Detect locked status and provide unlock guidance
- **Multi-format Export**: Stub implementations for contacts and logs
- **Recording Detection**: Identify audio recording locations

## Installation

### Prerequisites

- Rust 1.70 or later
- ADB (Android Debug Bridge) installed and in PATH
- Android device with USB debugging enabled

### Building from Source

```bash
git clone https://github.com/linuxiano85/NovaPcSuite.git
cd NovaPcSuite
cargo build --release
```

The binary will be available at `target/release/novapcsuite`.

## Quick Start

### Device Information

```bash
# Show device info
novapcsuite device info

# Show bootloader and OEM info  
novapcsuite device oem-info

# Use specific device serial
novapcsuite device info --serial 1234567890ABCDEF
```

### Backup Operations

```bash
# Full device backup
novapcsuite backup run --output ./backups

# Incremental backup
novapcsuite backup run --output ./backups --incremental

# List available backups
novapcsuite backup list --root ./backups

# Show backup details
novapcsuite backup show <backup-id> --root ./backups
```

### APK Backup

```bash
# Backup user-installed APKs
novapcsuite apps backup --root ./backups
```

### Restore Operations

```bash
# Restore to local directory
novapcsuite restore <backup-id> --root ./backups --target ./restore_out
```

## Backup Structure

Backups are organized as follows:

```
backups/
├── <device-serial>/
│   └── <timestamp>/
│       ├── manifest.yaml      # Primary manifest
│       ├── manifest.json      # JSON copy
│       ├── files/            # Backed up files
│       │   ├── DCIM/
│       │   ├── Pictures/
│       │   └── ...
│       ├── apks/             # User APK files
│       ├── contacts/         # Contact exports (stub)
│       └── logs/             # Call/SMS logs (stub)
```

## Manifest Format

```yaml
version: 1
id: <uuid>
created_at: <ISO8601>
device:
  serial: <device-serial>
  model: <device-model>
  brand: <device-brand>
  android_version: <version>
  sdk: <sdk-level>
strategy:
  incremental: false
  hash_algo: sha256
files:
  - path: /sdcard/DCIM/Camera/IMG_001.jpg
    category: image
    size: 2048576
    mtime: "2024-01-15 14:30:00"
    rel_dst: DCIM/Camera/IMG_001.jpg
    sha256: a1b2c3d4...
    status: success
apks:
  - package: com.example.app
    version_name: "1.0.0"
    version_code: "1"
    source_path: /data/app/com.example.app/base.apk
    sha256: e5f6g7h8...
contacts:
  status: no_permissions
  exported_vcf: null
  exported_csv: null
  exported_json: null
logs:
  status: no_permissions
  calls_json: null
  sms_json: null
recordings:
  status: success
  entries:
    - path: /sdcard/Recordings
      exists: true
    - path: /sdcard/MIUI/sound_recorder
      exists: false
```

## Supported Directories

By default, NovaPcSuite scans these directories:

- `/sdcard/DCIM` - Camera photos
- `/sdcard/Pictures` - Pictures 
- `/sdcard/Movies` - Videos
- `/sdcard/Music` - Audio files
- `/sdcard/Documents` - Documents
- `/sdcard/Download` - Downloads
- `/sdcard/WhatsApp/Media` - WhatsApp media
- `/sdcard/Telegram` - Telegram files
- `/sdcard/Recordings` - Audio recordings
- `/sdcard/MIUI/sound_recorder` - MIUI recordings

## File Categories

Files are automatically classified into:

- **Image**: jpg, jpeg, png, gif, bmp, webp, heic, heif
- **Video**: mp4, avi, mkv, mov, wmv, flv, webm, 3gp
- **Audio**: mp3, wav, flac, ogg, aac, m4a, wma
- **Document**: pdf, doc, docx, txt, rtf, odt, xls, xlsx, ppt, pptx
- **APK**: apk files
- **Other**: Everything else

## Device Support

Tested primarily on:
- **Redmi Note 12 Pro Plus 5G**

Should work with any Android device accessible via ADB. Bootloader unlock guidance is provided for:
- Xiaomi/Redmi devices
- Samsung devices  
- OnePlus devices
- Generic Android devices

## Configuration

Optional configuration file: `~/.config/novapcsuite/config.yaml`

```yaml
include:
  - /sdcard/DCIM
  - /sdcard/Pictures
  # ... custom directories
exclude:
  - "**/.thumbdata*"
  - "**/.thumbnails/*"
  - "**/cache/*"
backup:
  default_output_dir: ~/Documents/NovaPcSuite/backups
  incremental: false
  verify_hashes: true
  preserve_timestamps: true
adb:
  timeout_seconds: 30
  retry_attempts: 3
```

## Architecture

- **crates/core**: Core library with device management logic
- **crates/cli**: Command-line interface binary

### Core Modules

- `adb`: ADB wrapper for device communication
- `device`: Device information and bootloader analysis
- `scanner`: File discovery and classification
- `backup`: Backup execution and manifest creation
- `restore`: Restore operations and file recovery
- `manifest`: Backup manifest structure and serialization
- `config`: Configuration management

## Security Features

- SHA256 hash verification for all files
- No automatic root access attempts
- Controlled directory access via whitelist
- Manifest integrity with structured metadata

## Limitations

- **Contacts/Logs**: Currently stub implementations (requires content provider access)
- **Root Access**: No root-specific features (can be added later)
- **Incremental Backup**: Planned feature (basic structure present)
- **UI**: Command-line only (TUI/GUI planned for future)

## Future Roadmap

1. **Enhanced Scanner**: Fallback ls -lR parsing
2. **Incremental Backup**: Skip unchanged files using manifest comparison
3. **Progress Indicators**: Real-time progress bars and space estimation
4. **Parallel Operations**: Concurrent file hashing and transfer
5. **Advanced Rules**: DSL for include/exclude patterns
6. **Manifest Signing**: Cryptographic verification
7. **REST API**: HTTP interface for remote operations
8. **TUI Interface**: Terminal-based user interface
9. **Forensics Mode**: Advanced data recovery features
10. **APK Metadata**: Version extraction from APK files

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Troubleshooting

### Device Not Found

```bash
# Check ADB connection
adb devices

# Enable USB debugging on device
# Settings → Developer Options → USB Debugging
```

### Permission Denied

```bash
# Check USB debugging authorization
# Accept the prompt on device screen
```

### Large Backup Sizes

- Use `--incremental` flag for subsequent backups
- Check file categories in manifest to identify large files
- Consider excluding cache directories in config

## Platform Support

- ✅ Linux (primary)
- ✅ macOS  
- ✅ Windows (with ADB in PATH)

## Dependencies

- **tokio**: Async runtime
- **clap**: Command-line parsing
- **serde**: Serialization framework
- **sha2**: Hash calculation
- **walkdir**: Directory traversal
- **chrono**: Date/time handling
- **uuid**: Unique ID generation
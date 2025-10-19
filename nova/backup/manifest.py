"""Backup manifest model and utilities."""

import json
import uuid
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

from pydantic import BaseModel, Field
from ruamel.yaml import YAML

from ..adb.device import DeviceInfo
from ..util.logging import get_logger
from ..util.timeutil import now_iso

logger = get_logger(__name__)


class FileEntry(BaseModel):
    """Individual file entry in backup manifest."""
    
    path: str = Field(description="Original path on device")
    category: str = Field(description="File category (image, video, audio, document, etc.)")
    size: int = Field(description="File size in bytes")
    mtime: int = Field(description="Modification time (Unix timestamp)")
    hash: str = Field(description="File hash (SHA256)")
    rel_dst: str = Field(description="Relative destination path in backup")


class ApkEntry(BaseModel):
    """APK entry in backup manifest."""
    
    package: str = Field(description="Package name")
    version_name: str = Field(description="Version name")
    version_code: str = Field(description="Version code")
    source_path: str = Field(description="Original APK path on device")
    sha256: str = Field(description="APK file SHA256 hash")
    size: int = Field(default=0, description="APK file size in bytes")


class DeviceBackupInfo(BaseModel):
    """Device information in backup manifest."""
    
    serial: str = Field(description="Device serial number")
    model: str = Field(description="Device model")
    brand: str = Field(description="Device brand")
    android_version: str = Field(description="Android version")
    sdk: str = Field(description="SDK version")


class BackupStrategy(BaseModel):
    """Backup strategy configuration."""
    
    incremental: bool = Field(default=True, description="Incremental backup enabled")
    hash_algo: str = Field(default="sha256", description="Hash algorithm used")
    compression: bool = Field(default=False, description="Compression enabled")
    encryption: bool = Field(default=False, description="Encryption enabled")


class ExportedData(BaseModel):
    """Information about exported data."""
    
    contacts: Optional[str] = Field(default=None, description="Contacts export file path")
    calls: Optional[str] = Field(default=None, description="Call log export file path")
    sms: Optional[str] = Field(default=None, description="SMS export file path")


class BackupManifest(BaseModel):
    """Main backup manifest model."""
    
    version: int = Field(default=1, description="Manifest format version")
    id: str = Field(default_factory=lambda: str(uuid.uuid4()), description="Unique backup ID")
    created_at: str = Field(default_factory=now_iso, description="Backup creation timestamp")
    
    device: DeviceBackupInfo = Field(description="Device information")
    strategy: BackupStrategy = Field(default_factory=BackupStrategy, description="Backup strategy")
    
    files: List[FileEntry] = Field(default_factory=list, description="Backed up files")
    apk: List[ApkEntry] = Field(default_factory=list, description="Backed up APKs")
    
    exported: ExportedData = Field(default_factory=ExportedData, description="Exported data files")
    
    # Statistics
    total_files: int = Field(default=0, description="Total number of files")
    total_size: int = Field(default=0, description="Total size in bytes")
    total_apks: int = Field(default=0, description="Total number of APKs")
    
    class Config:
        """Pydantic configuration."""
        validate_assignment = True


class ManifestManager:
    """Utility for managing backup manifests."""
    
    def __init__(self, backup_path: Path):
        self.backup_path = backup_path
        self.manifest_path = backup_path / "manifest.yaml"
        self.manifest_json_path = backup_path / "manifest.json"
    
    def create_manifest(self, device_info: DeviceInfo) -> BackupManifest:
        """Create a new backup manifest."""
        device_backup_info = DeviceBackupInfo(
            serial=device_info.serial,
            model=device_info.model,
            brand=device_info.brand,
            android_version=device_info.android_version,
            sdk=device_info.sdk_version
        )
        
        manifest = BackupManifest(device=device_backup_info)
        logger.info(f"Created new backup manifest: {manifest.id}")
        
        return manifest
    
    def save_manifest(self, manifest: BackupManifest) -> None:
        """Save manifest to both YAML and JSON formats."""
        # Update statistics
        manifest.total_files = len(manifest.files)
        manifest.total_size = sum(f.size for f in manifest.files)
        manifest.total_apks = len(manifest.apk)
        
        # Ensure backup directory exists
        self.backup_path.mkdir(parents=True, exist_ok=True)
        
        # Save as YAML (primary format)
        yaml = YAML()
        yaml.default_flow_style = False
        yaml.width = 120
        
        with open(self.manifest_path, "w") as f:
            yaml.dump(manifest.model_dump(), f)
        
        # Save as JSON (redundancy)
        with open(self.manifest_json_path, "w") as f:
            json.dump(manifest.model_dump(), f, indent=2, default=str)
        
        logger.debug(f"Saved manifest to {self.manifest_path}")
    
    def load_manifest(self) -> Optional[BackupManifest]:
        """Load manifest from file."""
        # Try YAML first
        if self.manifest_path.exists():
            try:
                yaml = YAML(typ="safe")
                with open(self.manifest_path, "r") as f:
                    data = yaml.load(f)
                
                manifest = BackupManifest(**data)
                logger.debug(f"Loaded manifest from {self.manifest_path}")
                return manifest
                
            except Exception as e:
                logger.warning(f"Failed to load YAML manifest: {e}")
        
        # Fallback to JSON
        if self.manifest_json_path.exists():
            try:
                with open(self.manifest_json_path, "r") as f:
                    data = json.load(f)
                
                manifest = BackupManifest(**data)
                logger.debug(f"Loaded manifest from {self.manifest_json_path}")
                return manifest
                
            except Exception as e:
                logger.warning(f"Failed to load JSON manifest: {e}")
        
        return None
    
    def add_file_entry(self, manifest: BackupManifest, entry: FileEntry) -> None:
        """Add a file entry to the manifest."""
        manifest.files.append(entry)
    
    def add_apk_entry(self, manifest: BackupManifest, entry: ApkEntry) -> None:
        """Add an APK entry to the manifest."""
        manifest.apk.append(entry)
    
    def set_exported_data(self, manifest: BackupManifest, data_type: str, file_path: str) -> None:
        """Set exported data file path."""
        if data_type == "contacts":
            manifest.exported.contacts = file_path
        elif data_type == "calls":
            manifest.exported.calls = file_path
        elif data_type == "sms":
            manifest.exported.sms = file_path
    
    def get_file_by_path(self, manifest: BackupManifest, device_path: str) -> Optional[FileEntry]:
        """Find a file entry by device path."""
        for file_entry in manifest.files:
            if file_entry.path == device_path:
                return file_entry
        return None
    
    def get_apk_by_package(self, manifest: BackupManifest, package_name: str) -> Optional[ApkEntry]:
        """Find an APK entry by package name."""
        for apk_entry in manifest.apk:
            if apk_entry.package == package_name:
                return apk_entry
        return None


def categorize_file(file_path: str) -> str:
    """Categorize a file based on its path and extension."""
    path_lower = file_path.lower()
    
    # Image files
    image_extensions = ['.jpg', '.jpeg', '.png', '.gif', '.bmp', '.webp', '.heic', '.heif']
    if any(path_lower.endswith(ext) for ext in image_extensions):
        return "image"
    
    # Video files
    video_extensions = ['.mp4', '.mkv', '.avi', '.mov', '.wmv', '.flv', '.webm', '.m4v']
    if any(path_lower.endswith(ext) for ext in video_extensions):
        return "video"
    
    # Audio files
    audio_extensions = ['.mp3', '.wav', '.flac', '.aac', '.ogg', '.wma', '.m4a']
    if any(path_lower.endswith(ext) for ext in audio_extensions):
        return "audio"
    
    # Document files
    document_extensions = ['.pdf', '.doc', '.docx', '.txt', '.rtf', '.odt']
    if any(path_lower.endswith(ext) for ext in document_extensions):
        return "document"
    
    # Archive files
    archive_extensions = ['.zip', '.rar', '.7z', '.tar', '.gz', '.bz2']
    if any(path_lower.endswith(ext) for ext in archive_extensions):
        return "archive"
    
    # Check by directory path
    if any(folder in path_lower for folder in ['/dcim/', '/camera/', '/pictures/']):
        return "image"
    elif any(folder in path_lower for folder in ['/movies/', '/video/']):
        return "video"
    elif any(folder in path_lower for folder in ['/music/', '/audio/']):
        return "audio"
    elif any(folder in path_lower for folder in ['/documents/', '/download/']):
        return "document"
    elif 'whatsapp' in path_lower or 'telegram' in path_lower:
        return "messaging"
    
    return "other"
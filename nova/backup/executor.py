"""Backup execution engine."""

import hashlib
import typing as t
from datetime import datetime
from pathlib import Path

from nova.adb.client import ADBClient
from nova.adb.device import get_device_info
from nova.backup.manifest import BackupManifest, FileEntry
from nova.backup.scanner import BackupScanner, FileInfo
from nova.backup.storage import BackupStorage


class BackupExecutor:
    """Executes backup operations."""
    
    def __init__(self, adb_client: ADBClient, storage: BackupStorage) -> None:
        """Initialize backup executor.
        
        Args:
            adb_client: ADB client for device communication
            storage: Backup storage manager
        """
        self.adb_client = adb_client
        self.storage = storage
    
    def calculate_file_hash(self, file_path: Path) -> str:
        """Calculate SHA256 hash of a file.
        
        Args:
            file_path: Path to the file
            
        Returns:
            SHA256 hash as hex string
        """
        sha256_hash = hashlib.sha256()
        
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                sha256_hash.update(chunk)
        
        return sha256_hash.hexdigest()
    
    def backup_file(self, device_id: str, file_info: FileInfo, backup_files_dir: Path) -> FileEntry:
        """Backup a single file from device.
        
        Args:
            device_id: Device identifier
            file_info: Information about the file to backup
            backup_files_dir: Directory to store backed up files
            
        Returns:
            FileEntry with backup metadata
        """
        # Create relative path preserving directory structure
        # Remove leading slash and convert to relative path
        relative_path = file_info.path.lstrip('/')
        local_path = backup_files_dir / relative_path
        
        # Pull file from device
        self.adb_client.pull(device_id, file_info.path, local_path)
        
        # Calculate hash
        file_hash = self.calculate_file_hash(local_path)
        
        return FileEntry(
            relative_path=relative_path,
            original_path=file_info.path,
            size=file_info.size,
            sha256=file_hash,
            modified_time=datetime.now()  # We could get actual mtime from device
        )
    
    def execute_backup(
        self,
        device_id: str,
        whitelist: t.Optional[t.List[str]] = None,
        progress_callback: t.Optional[t.Callable[[str, int, int], None]] = None
    ) -> BackupManifest:
        """Execute a full backup of device.
        
        Args:
            device_id: Device identifier
            whitelist: List of directories to backup (uses default if None)
            progress_callback: Optional callback for progress updates (message, current, total)
            
        Returns:
            BackupManifest with backup metadata
        """
        if progress_callback:
            progress_callback("Getting device information...", 0, 100)
        
        # Get device information
        device_info = get_device_info(self.adb_client, device_id)
        
        if progress_callback:
            progress_callback("Scanning device for files...", 10, 100)
        
        # Scan device for files
        scanner = BackupScanner(self.adb_client, device_id)
        categorized_files = scanner.scan_whitelist(whitelist)
        
        # Flatten file list
        all_files = []
        for files in categorized_files.values():
            all_files.extend(files)
        
        if not all_files:
            if progress_callback:
                progress_callback("No files found to backup", 100, 100)
            
            # Create empty manifest
            return BackupManifest(
                device_id=device_id,
                device_info=device_info,
                total_files=0,
                total_size=0,
                files=[]
            )
        
        if progress_callback:
            progress_callback(f"Found {len(all_files)} files to backup", 20, 100)
        
        # Create backup session
        backup_dir = self.storage.create_backup_session(device_id)
        files_dir = self.storage.get_files_dir(backup_dir)
        files_dir.mkdir(parents=True, exist_ok=True)
        
        if progress_callback:
            progress_callback("Starting file backup...", 30, 100)
        
        # Backup files
        backed_up_files = []
        total_size = 0
        
        for i, file_info in enumerate(all_files):
            if progress_callback:
                progress = 30 + int((i / len(all_files)) * 60)  # 30% to 90%
                progress_callback(f"Backing up: {file_info.path}", progress, 100)
            
            try:
                file_entry = self.backup_file(device_id, file_info, files_dir)
                backed_up_files.append(file_entry)
                total_size += file_entry.size
            except Exception as e:
                # Log error but continue with other files
                print(f"Failed to backup {file_info.path}: {e}")
        
        if progress_callback:
            progress_callback("Creating backup manifest...", 90, 100)
        
        # Create manifest
        manifest = BackupManifest(
            device_id=device_id,
            device_info=device_info,
            total_files=len(backed_up_files),
            total_size=total_size,
            files=backed_up_files
        )
        
        # Save manifest
        manifest_path = self.storage.get_manifest_path(backup_dir)
        manifest.save(manifest_path)
        
        if progress_callback:
            progress_callback("Backup completed successfully!", 100, 100)
        
        return manifest
"""Backup restore functionality."""

import typing as t
from pathlib import Path

from nova.adb.client import ADBClient
from nova.backup.manifest import BackupManifest
from nova.backup.storage import BackupStorage


class RestoreExecutor:
    """Executes restore operations."""
    
    def __init__(self, adb_client: ADBClient, storage: BackupStorage) -> None:
        """Initialize restore executor.
        
        Args:
            adb_client: ADB client for device communication
            storage: Backup storage manager
        """
        self.adb_client = adb_client
        self.storage = storage
    
    def restore_backup(
        self,
        device_id: str,
        backup_dir: Path,
        target_path: str = "/storage/emulated/0/Restored",
        progress_callback: t.Optional[t.Callable[[str, int, int], None]] = None
    ) -> bool:
        """Restore files from backup to device.
        
        Args:
            device_id: Device identifier
            backup_dir: Backup directory to restore from
            target_path: Target path on device for restored files
            progress_callback: Optional callback for progress updates
            
        Returns:
            True if restore was successful
        """
        if progress_callback:
            progress_callback("Loading backup manifest...", 0, 100)
        
        # Load manifest
        manifest_path = self.storage.get_manifest_path(backup_dir)
        if not manifest_path.exists():
            raise ValueError(f"Manifest not found: {manifest_path}")
        
        manifest = BackupManifest.load(manifest_path)
        files_dir = self.storage.get_files_dir(backup_dir)
        
        if not manifest.files:
            if progress_callback:
                progress_callback("No files to restore", 100, 100)
            return True
        
        if progress_callback:
            progress_callback(f"Restoring {len(manifest.files)} files...", 10, 100)
        
        # Create target directory on device
        try:
            self.adb_client.shell(device_id, f"mkdir -p '{target_path}'")
        except Exception as e:
            print(f"Warning: Could not create target directory: {e}")
        
        # Restore files
        successful_restores = 0
        
        for i, file_entry in enumerate(manifest.files):
            if progress_callback:
                progress = 10 + int((i / len(manifest.files)) * 80)  # 10% to 90%
                progress_callback(f"Restoring: {file_entry.relative_path}", progress, 100)
            
            local_file_path = files_dir / file_entry.relative_path
            remote_file_path = f"{target_path}/{file_entry.relative_path}"
            
            try:
                # Create parent directory on device if needed
                remote_parent = str(Path(remote_file_path).parent)
                self.adb_client.shell(device_id, f"mkdir -p '{remote_parent}'")
                
                # Push file to device
                self.adb_client._run_command(["-s", device_id, "push", str(local_file_path), remote_file_path])
                successful_restores += 1
                
            except Exception as e:
                print(f"Failed to restore {file_entry.relative_path}: {e}")
        
        if progress_callback:
            progress_callback(f"Restore completed: {successful_restores}/{len(manifest.files)} files", 100, 100)
        
        return successful_restores > 0
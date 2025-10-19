"""Backup storage layout management."""

import typing as t
from datetime import datetime
from pathlib import Path


class BackupStorage:
    """Manages backup storage layout and organization."""
    
    def __init__(self, base_path: Path) -> None:
        """Initialize backup storage.
        
        Args:
            base_path: Base directory for all backups
        """
        self.base_path = Path(base_path)
        self.base_path.mkdir(parents=True, exist_ok=True)
    
    def get_device_backup_dir(self, device_id: str) -> Path:
        """Get backup directory for a specific device.
        
        Args:
            device_id: Device identifier
            
        Returns:
            Path to device backup directory
        """
        # Sanitize device ID for filesystem
        safe_device_id = "".join(c for c in device_id if c.isalnum() or c in "._-")
        return self.base_path / safe_device_id
    
    def create_backup_session(self, device_id: str, timestamp: t.Optional[datetime] = None) -> Path:
        """Create a new backup session directory.
        
        Args:
            device_id: Device identifier
            timestamp: Backup timestamp (uses current time if None)
            
        Returns:
            Path to backup session directory
        """
        if timestamp is None:
            timestamp = datetime.now()
        
        device_dir = self.get_device_backup_dir(device_id)
        session_name = timestamp.strftime("%Y%m%d_%H%M%S")
        session_dir = device_dir / session_name
        
        session_dir.mkdir(parents=True, exist_ok=True)
        return session_dir
    
    def get_latest_backup(self, device_id: str) -> t.Optional[Path]:
        """Get the latest backup directory for a device.
        
        Args:
            device_id: Device identifier
            
        Returns:
            Path to latest backup or None if no backups exist
        """
        device_dir = self.get_device_backup_dir(device_id)
        
        if not device_dir.exists():
            return None
        
        backup_dirs = [
            d for d in device_dir.iterdir()
            if d.is_dir() and len(d.name) == 15  # YYYYMMDD_HHMMSS format
        ]
        
        if not backup_dirs:
            return None
        
        return max(backup_dirs, key=lambda d: d.name)
    
    def list_backups(self, device_id: str) -> t.List[Path]:
        """List all backup directories for a device.
        
        Args:
            device_id: Device identifier
            
        Returns:
            List of backup directories sorted by date (newest first)
        """
        device_dir = self.get_device_backup_dir(device_id)
        
        if not device_dir.exists():
            return []
        
        backup_dirs = [
            d for d in device_dir.iterdir()
            if d.is_dir() and len(d.name) == 15  # YYYYMMDD_HHMMSS format
        ]
        
        return sorted(backup_dirs, key=lambda d: d.name, reverse=True)
    
    def get_manifest_path(self, backup_dir: Path) -> Path:
        """Get path to manifest file for a backup.
        
        Args:
            backup_dir: Backup directory
            
        Returns:
            Path to manifest.json file
        """
        return backup_dir / "manifest.json"
    
    def get_files_dir(self, backup_dir: Path) -> Path:
        """Get path to files directory for a backup.
        
        Args:
            backup_dir: Backup directory
            
        Returns:
            Path to files directory
        """
        return backup_dir / "files"
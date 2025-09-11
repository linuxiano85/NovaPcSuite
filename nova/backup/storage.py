"""Backup storage management utilities."""

from pathlib import Path
from typing import Dict, List, Optional

from ..config import get_config
from ..util.logging import get_logger
from ..util.paths import format_size
from .manifest import BackupManifest, ManifestManager

logger = get_logger(__name__)


class BackupStorage:
    """Manages backup storage and metadata."""
    
    def __init__(self):
        self.config = get_config()
        self.backup_root = self.config.backup_root
    
    def list_device_backups(self, device_serial: str) -> List[Dict[str, any]]:
        """List all backups for a specific device."""
        device_backup_dir = self.backup_root / device_serial
        
        if not device_backup_dir.exists():
            return []
        
        backups = []
        
        for backup_dir in device_backup_dir.iterdir():
            if backup_dir.is_dir():
                backup_info = self._get_backup_info(backup_dir)
                if backup_info:
                    backups.append(backup_info)
        
        # Sort by creation time (newest first)
        backups.sort(key=lambda x: x["created_at"], reverse=True)
        
        return backups
    
    def list_all_backups(self) -> List[Dict[str, any]]:
        """List all backups from all devices."""
        if not self.backup_root.exists():
            return []
        
        all_backups = []
        
        for device_dir in self.backup_root.iterdir():
            if device_dir.is_dir():
                device_backups = self.list_device_backups(device_dir.name)
                all_backups.extend(device_backups)
        
        # Sort by creation time (newest first)
        all_backups.sort(key=lambda x: x["created_at"], reverse=True)
        
        return all_backups
    
    def get_backup_by_id(self, backup_id: str) -> Optional[Dict[str, any]]:
        """Find a backup by its ID."""
        all_backups = self.list_all_backups()
        
        for backup in all_backups:
            if backup["id"] == backup_id:
                return backup
        
        return None
    
    def get_backup_manifest(self, device_serial: str, backup_timestamp: str) -> Optional[BackupManifest]:
        """Load a backup manifest."""
        backup_path = self.backup_root / device_serial / backup_timestamp
        
        if not backup_path.exists():
            return None
        
        manifest_manager = ManifestManager(backup_path)
        return manifest_manager.load_manifest()
    
    def _get_backup_info(self, backup_path: Path) -> Optional[Dict[str, any]]:
        """Get backup information from a backup directory."""
        try:
            manifest_manager = ManifestManager(backup_path)
            manifest = manifest_manager.load_manifest()
            
            if not manifest:
                return None
            
            # Calculate actual backup size
            actual_size = self._calculate_backup_size(backup_path)
            
            return {
                "id": manifest.id,
                "timestamp": backup_path.name,
                "device_serial": manifest.device.serial,
                "device_name": f"{manifest.device.brand} {manifest.device.model}",
                "created_at": manifest.created_at,
                "total_files": manifest.total_files,
                "total_size": manifest.total_size,
                "total_apks": manifest.total_apks,
                "actual_size": actual_size,
                "path": str(backup_path),
                "incremental": manifest.strategy.incremental,
            }
            
        except Exception as e:
            logger.warning(f"Failed to get backup info for {backup_path}: {e}")
            return None
    
    def _calculate_backup_size(self, backup_path: Path) -> int:
        """Calculate actual size of backup directory."""
        total_size = 0
        
        try:
            for file_path in backup_path.rglob("*"):
                if file_path.is_file():
                    total_size += file_path.stat().st_size
        except Exception as e:
            logger.debug(f"Error calculating backup size for {backup_path}: {e}")
        
        return total_size
    
    def delete_backup(self, device_serial: str, backup_timestamp: str) -> bool:
        """Delete a backup."""
        backup_path = self.backup_root / device_serial / backup_timestamp
        
        if not backup_path.exists():
            logger.warning(f"Backup does not exist: {backup_path}")
            return False
        
        try:
            import shutil
            shutil.rmtree(backup_path)
            logger.info(f"Deleted backup: {backup_path}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to delete backup {backup_path}: {e}")
            return False
    
    def get_storage_usage(self) -> Dict[str, any]:
        """Get storage usage information."""
        if not self.backup_root.exists():
            return {
                "total_size": 0,
                "total_backups": 0,
                "devices": {},
                "formatted_size": "0 B"
            }
        
        total_size = 0
        total_backups = 0
        devices = {}
        
        for device_dir in self.backup_root.iterdir():
            if device_dir.is_dir():
                device_serial = device_dir.name
                device_backups = self.list_device_backups(device_serial)
                
                device_size = sum(b.get("actual_size", 0) for b in device_backups)
                device_count = len(device_backups)
                
                devices[device_serial] = {
                    "size": device_size,
                    "count": device_count,
                    "formatted_size": format_size(device_size)
                }
                
                total_size += device_size
                total_backups += device_count
        
        return {
            "total_size": total_size,
            "total_backups": total_backups,
            "devices": devices,
            "formatted_size": format_size(total_size)
        }
    
    def cleanup_old_backups(self, device_serial: str, keep_count: int = 5) -> int:
        """Clean up old backups, keeping only the most recent ones."""
        backups = self.list_device_backups(device_serial)
        
        if len(backups) <= keep_count:
            return 0
        
        # Keep the most recent backups, delete the rest
        backups_to_delete = backups[keep_count:]
        deleted_count = 0
        
        for backup in backups_to_delete:
            timestamp = backup["timestamp"]
            if self.delete_backup(device_serial, timestamp):
                deleted_count += 1
        
        logger.info(f"Cleaned up {deleted_count} old backups for device {device_serial}")
        return deleted_count
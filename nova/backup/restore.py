"""Backup restore functionality."""

import os
import shutil
from pathlib import Path
from typing import Callable, List, Optional

from tqdm import tqdm

from ..adb.device import ADBDevice
from ..util.hashing import verify_file_integrity
from ..util.logging import get_logger
from ..util.paths import ensure_directory
from .manifest import BackupManifest, FileEntry
from .storage import BackupStorage

logger = get_logger(__name__)


class RestoreExecutor:
    """Executes restore operations."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.storage = BackupStorage()
    
    def restore_files(
        self,
        backup_id: str,
        target_dir: Optional[Path] = None,
        file_patterns: Optional[List[str]] = None,
        verify_integrity: bool = True,
        progress_callback: Optional[Callable[[int, int, str], None]] = None
    ) -> Dict[str, any]:
        """Restore files from a backup."""
        
        # Find backup
        backup_info = self.storage.get_backup_by_id(backup_id)
        if not backup_info:
            raise ValueError(f"Backup not found: {backup_id}")
        
        # Load manifest
        manifest = self.storage.get_backup_manifest(
            backup_info["device_serial"],
            backup_info["timestamp"]
        )
        if not manifest:
            raise ValueError(f"Could not load manifest for backup: {backup_id}")
        
        backup_path = Path(backup_info["path"])
        files_dir = backup_path / "files"
        
        # Filter files to restore
        files_to_restore = self._filter_files_for_restore(manifest.files, file_patterns)
        
        logger.info(f"Restoring {len(files_to_restore)} files from backup {backup_id}")
        
        if target_dir:
            logger.info(f"Restore target: {target_dir}")
        else:
            logger.info("Restore target: original locations")
        
        success_count = 0
        failed_files = []
        total_files = len(files_to_restore)
        
        with tqdm(total=total_files, desc="Restoring files", unit="file") as pbar:
            for i, file_entry in enumerate(files_to_restore):
                if progress_callback:
                    progress_callback(i + 1, total_files, file_entry.path)
                
                pbar.set_postfix_str(os.path.basename(file_entry.path))
                
                if self._restore_single_file(file_entry, files_dir, target_dir, verify_integrity):
                    success_count += 1
                else:
                    failed_files.append(file_entry.path)
                
                pbar.update(1)
        
        result = {
            "total_files": total_files,
            "success_count": success_count,
            "failed_count": len(failed_files),
            "failed_files": failed_files,
        }
        
        logger.info(f"Restore completed: {success_count}/{total_files} files successful")
        
        return result
    
    def restore_to_local(
        self,
        backup_id: str,
        local_target_dir: Path,
        file_patterns: Optional[List[str]] = None,
        verify_integrity: bool = True,
        progress_callback: Optional[Callable[[int, int, str], None]] = None
    ) -> Dict[str, any]:
        """Restore files to a local directory (not to device)."""
        
        return self.restore_files(
            backup_id=backup_id,
            target_dir=local_target_dir,
            file_patterns=file_patterns,
            verify_integrity=verify_integrity,
            progress_callback=progress_callback
        )
    
    def _restore_single_file(
        self,
        file_entry: FileEntry,
        backup_files_dir: Path,
        target_dir: Optional[Path],
        verify_integrity: bool
    ) -> bool:
        """Restore a single file."""
        try:
            # Source file in backup
            source_file = backup_files_dir / file_entry.rel_dst
            
            if not source_file.exists():
                logger.error(f"Backup file not found: {source_file}")
                return False
            
            # Determine destination
            if target_dir:
                # Restore to custom directory, preserving relative structure
                dest_file = target_dir / file_entry.rel_dst
            else:
                # For local restore, we need a target directory
                logger.error("Target directory required for file restore")
                return False
            
            # Ensure destination directory exists
            ensure_directory(dest_file.parent)
            
            # Copy file
            shutil.copy2(source_file, dest_file)
            
            # Verify integrity if requested
            if verify_integrity:
                if not verify_file_integrity(dest_file, file_entry.hash):
                    logger.error(f"Integrity verification failed: {dest_file}")
                    return False
            
            logger.debug(f"Restored file: {file_entry.path} -> {dest_file}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to restore file {file_entry.path}: {e}")
            return False
    
    def _filter_files_for_restore(
        self,
        all_files: List[FileEntry],
        file_patterns: Optional[List[str]]
    ) -> List[FileEntry]:
        """Filter files based on patterns."""
        if not file_patterns:
            return all_files
        
        import fnmatch
        
        filtered_files = []
        
        for file_entry in all_files:
            file_path = file_entry.path
            file_name = os.path.basename(file_path)
            
            # Check if file matches any pattern
            for pattern in file_patterns:
                if (fnmatch.fnmatch(file_name, pattern) or 
                    fnmatch.fnmatch(file_path, pattern) or
                    pattern in file_path):
                    filtered_files.append(file_entry)
                    break
        
        return filtered_files
    
    def list_backup_files(self, backup_id: str) -> List[Dict[str, any]]:
        """List files in a backup."""
        backup_info = self.storage.get_backup_by_id(backup_id)
        if not backup_info:
            return []
        
        manifest = self.storage.get_backup_manifest(
            backup_info["device_serial"],
            backup_info["timestamp"]
        )
        if not manifest:
            return []
        
        files = []
        for file_entry in manifest.files:
            files.append({
                "path": file_entry.path,
                "category": file_entry.category,
                "size": file_entry.size,
                "mtime": file_entry.mtime,
                "hash": file_entry.hash,
            })
        
        return files
    
    def restore_apks(
        self,
        backup_id: str,
        output_dir: Path,
        package_patterns: Optional[List[str]] = None,
        progress_callback: Optional[Callable[[int, int, str], None]] = None
    ) -> Dict[str, any]:
        """Restore APK files from backup."""
        
        backup_info = self.storage.get_backup_by_id(backup_id)
        if not backup_info:
            raise ValueError(f"Backup not found: {backup_id}")
        
        manifest = self.storage.get_backup_manifest(
            backup_info["device_serial"],
            backup_info["timestamp"]
        )
        if not manifest:
            raise ValueError(f"Could not load manifest for backup: {backup_id}")
        
        backup_path = Path(backup_info["path"])
        apk_dir = backup_path / "apk"
        
        # Filter APKs to restore
        apks_to_restore = manifest.apk
        if package_patterns:
            import fnmatch
            apks_to_restore = [
                apk for apk in manifest.apk
                if any(fnmatch.fnmatch(apk.package, pattern) for pattern in package_patterns)
            ]
        
        logger.info(f"Restoring {len(apks_to_restore)} APKs to {output_dir}")
        
        ensure_directory(output_dir)
        success_count = 0
        failed_apks = []
        
        with tqdm(total=len(apks_to_restore), desc="Restoring APKs", unit="apk") as pbar:
            for i, apk_entry in enumerate(apks_to_restore):
                if progress_callback:
                    progress_callback(i + 1, len(apks_to_restore), apk_entry.package)
                
                pbar.set_postfix_str(apk_entry.package)
                
                source_apk = apk_dir / f"{apk_entry.package}.apk"
                dest_apk = output_dir / f"{apk_entry.package}.apk"
                
                try:
                    if source_apk.exists():
                        shutil.copy2(source_apk, dest_apk)
                        success_count += 1
                        logger.debug(f"Restored APK: {apk_entry.package}")
                    else:
                        failed_apks.append(apk_entry.package)
                        logger.error(f"APK file not found: {source_apk}")
                        
                except Exception as e:
                    failed_apks.append(apk_entry.package)
                    logger.error(f"Failed to restore APK {apk_entry.package}: {e}")
                
                pbar.update(1)
        
        result = {
            "total_apks": len(apks_to_restore),
            "success_count": success_count,
            "failed_count": len(failed_apks),
            "failed_apks": failed_apks,
        }
        
        logger.info(f"APK restore completed: {success_count}/{len(apks_to_restore)} APKs successful")
        
        return result
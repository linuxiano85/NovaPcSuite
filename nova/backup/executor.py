"""Backup execution engine."""

import os
from pathlib import Path
from typing import Callable, Dict, List, Optional

from tqdm import tqdm

from ..adb.device import ADBDevice
from ..adb.pull import FilePuller
from ..util.hashing import calculate_file_hash
from ..util.logging import get_logger
from ..util.paths import ensure_directory, get_backup_path, relative_to_device_root
from ..util.timeutil import generate_backup_id
from .manifest import (
    ApkEntry,
    BackupManifest,
    FileEntry,
    ManifestManager,
    categorize_file,
)
from .scanner import ScanResult

logger = get_logger(__name__)


class BackupExecutor:
    """Executes backup operations."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.puller = FilePuller(device)
    
    def execute_backup(
        self,
        scan_result: ScanResult,
        backup_id: Optional[str] = None,
        incremental: bool = True,
        progress_callback: Optional[Callable[[int, int, str], None]] = None
    ) -> Optional[BackupManifest]:
        """Execute a backup operation."""
        
        if backup_id is None:
            backup_id = generate_backup_id()
        
        device_info = self.device.get_device_info()
        backup_path = get_backup_path(device_info.serial, backup_id)
        
        logger.info(f"Starting backup to {backup_path}")
        logger.info(f"Files to backup: {len(scan_result.files)}")
        
        # Create backup directory
        ensure_directory(backup_path)
        files_dir = backup_path / "files"
        ensure_directory(files_dir)
        
        # Initialize manifest
        manifest_manager = ManifestManager(backup_path)
        manifest = manifest_manager.create_manifest(device_info)
        
        # Load previous manifest for incremental backup
        previous_manifest = None
        if incremental:
            previous_manifest = self._find_previous_manifest(device_info.serial)
        
        # Execute file backup
        success_count = 0
        total_files = len(scan_result.files)
        
        with tqdm(total=total_files, desc="Backing up files", unit="file") as pbar:
            for i, file_info in enumerate(scan_result.files):
                device_path = file_info["path"]
                
                if progress_callback:
                    progress_callback(i + 1, total_files, device_path)
                
                pbar.set_postfix_str(os.path.basename(device_path))
                
                # Check if file needs backup (incremental)
                if incremental and previous_manifest:
                    previous_entry = manifest_manager.get_file_by_path(previous_manifest, device_path)
                    if self._should_skip_file(file_info, previous_entry):
                        logger.debug(f"Skipping unchanged file: {device_path}")
                        # Copy entry from previous manifest
                        if previous_entry:
                            manifest_manager.add_file_entry(manifest, previous_entry)
                        pbar.update(1)
                        continue
                
                # Backup file
                if self._backup_single_file(file_info, files_dir, manifest, manifest_manager):
                    success_count += 1
                
                pbar.update(1)
        
        logger.info(f"File backup completed: {success_count}/{total_files} files successful")
        
        # Save manifest
        manifest_manager.save_manifest(manifest)
        
        return manifest
    
    def _backup_single_file(
        self,
        file_info: Dict[str, any],
        files_dir: Path,
        manifest: BackupManifest,
        manifest_manager: ManifestManager
    ) -> bool:
        """Backup a single file."""
        device_path = file_info["path"]
        
        try:
            # Determine relative destination path
            rel_path = relative_to_device_root(device_path)
            local_path = files_dir / rel_path
            
            # Ensure destination directory exists
            ensure_directory(local_path.parent)
            
            # Pull file from device
            if not self.puller.pull_file(device_path, local_path, verify_hash=False):
                logger.error(f"Failed to pull file: {device_path}")
                return False
            
            # Calculate hash of local file
            file_hash = calculate_file_hash(local_path)
            if not file_hash:
                logger.error(f"Failed to calculate hash for: {local_path}")
                return False
            
            # Create file entry
            file_entry = FileEntry(
                path=device_path,
                category=categorize_file(device_path),
                size=file_info.get("size", 0),
                mtime=file_info.get("mtime", 0),
                hash=file_hash,
                rel_dst=str(rel_path)
            )
            
            # Add to manifest
            manifest_manager.add_file_entry(manifest, file_entry)
            
            logger.debug(f"Successfully backed up: {device_path}")
            return True
            
        except Exception as e:
            logger.error(f"Error backing up {device_path}: {e}")
            return False
    
    def _should_skip_file(
        self,
        file_info: Dict[str, any],
        previous_entry: Optional[FileEntry]
    ) -> bool:
        """Determine if file should be skipped in incremental backup."""
        if not previous_entry:
            return False
        
        # Compare size and modification time
        current_size = file_info.get("size", 0)
        current_mtime = file_info.get("mtime", 0)
        
        if (current_size == previous_entry.size and 
            current_mtime == previous_entry.mtime):
            return True
        
        return False
    
    def _find_previous_manifest(self, device_serial: str) -> Optional[BackupManifest]:
        """Find the most recent backup manifest for incremental backup."""
        try:
            from ..config import get_config
            config = get_config()
            device_backup_root = config.backup_root / device_serial
            
            if not device_backup_root.exists():
                return None
            
            # Find the most recent backup directory
            backup_dirs = [d for d in device_backup_root.iterdir() if d.is_dir()]
            if not backup_dirs:
                return None
            
            # Sort by name (timestamp format ensures chronological order)
            backup_dirs.sort(reverse=True)
            
            for backup_dir in backup_dirs:
                manifest_manager = ManifestManager(backup_dir)
                manifest = manifest_manager.load_manifest()
                if manifest:
                    logger.info(f"Found previous manifest: {backup_dir.name}")
                    return manifest
            
            return None
            
        except Exception as e:
            logger.warning(f"Could not find previous manifest: {e}")
            return None
    
    def backup_apks(
        self,
        package_names: List[str],
        backup_path: Path,
        manifest: BackupManifest,
        manifest_manager: ManifestManager,
        progress_callback: Optional[Callable[[int, int, str], None]] = None
    ) -> int:
        """Backup APK files for specified packages."""
        from ..adb.package import PackageManager
        
        package_manager = PackageManager(self.device)
        apk_dir = backup_path / "apk"
        ensure_directory(apk_dir)
        
        logger.info(f"Starting APK backup for {len(package_names)} packages")
        
        success_count = 0
        
        with tqdm(total=len(package_names), desc="Backing up APKs", unit="apk") as pbar:
            for i, package_name in enumerate(package_names):
                if progress_callback:
                    progress_callback(i + 1, len(package_names), package_name)
                
                pbar.set_postfix_str(package_name)
                
                if self._backup_single_apk(package_name, package_manager, apk_dir, manifest, manifest_manager):
                    success_count += 1
                
                pbar.update(1)
        
        logger.info(f"APK backup completed: {success_count}/{len(package_names)} packages successful")
        return success_count
    
    def _backup_single_apk(
        self,
        package_name: str,
        package_manager,
        apk_dir: Path,
        manifest: BackupManifest,
        manifest_manager: ManifestManager
    ) -> bool:
        """Backup a single APK file."""
        try:
            # Get package info
            package_info = package_manager.get_package_info(package_name)
            if not package_info or not package_info.apk_path:
                logger.warning(f"Could not get APK path for package: {package_name}")
                return False
            
            # Determine local APK path
            apk_filename = f"{package_name}.apk"
            local_apk_path = apk_dir / apk_filename
            
            # Pull APK file
            if not self.puller.pull_file(package_info.apk_path, local_apk_path):
                logger.error(f"Failed to pull APK: {package_info.apk_path}")
                return False
            
            # Calculate APK hash
            apk_hash = calculate_file_hash(local_apk_path)
            if not apk_hash:
                logger.error(f"Failed to calculate hash for APK: {local_apk_path}")
                return False
            
            # Get APK size
            apk_size = local_apk_path.stat().st_size
            
            # Create APK entry
            apk_entry = ApkEntry(
                package=package_name,
                version_name=package_info.version_name,
                version_code=package_info.version_code,
                source_path=package_info.apk_path,
                sha256=apk_hash,
                size=apk_size
            )
            
            # Add to manifest
            manifest_manager.add_apk_entry(manifest, apk_entry)
            
            logger.debug(f"Successfully backed up APK: {package_name}")
            return True
            
        except Exception as e:
            logger.error(f"Error backing up APK {package_name}: {e}")
            return False
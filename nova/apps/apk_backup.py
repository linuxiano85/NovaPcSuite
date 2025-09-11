"""APK backup functionality."""

import json
from pathlib import Path
from typing import Dict, List, Optional

from tqdm import tqdm

from ..adb.device import ADBDevice
from ..adb.package import PackageManager
from ..adb.pull import FilePuller
from ..util.hashing import calculate_file_hash
from ..util.logging import get_logger
from ..util.paths import ensure_directory, format_size

logger = get_logger(__name__)


class APKBackup:
    """Handles APK backup operations."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.package_manager = PackageManager(device)
        self.puller = FilePuller(device)
    
    def backup_all_user_apps(
        self,
        output_dir: Path,
        progress_callback: Optional[callable] = None
    ) -> Dict[str, any]:
        """Backup all user-installed applications."""
        
        logger.info("Starting backup of all user applications...")
        
        # Get list of user packages
        packages = self.package_manager.list_packages(include_system=False)
        
        if not packages:
            logger.warning("No user packages found")
            return {"total": 0, "success": 0, "failed": 0, "packages": []}
        
        return self.backup_packages(packages, output_dir, progress_callback)
    
    def backup_packages(
        self,
        package_names: List[str],
        output_dir: Path,
        progress_callback: Optional[callable] = None
    ) -> Dict[str, any]:
        """Backup specific packages."""
        
        logger.info(f"Starting backup of {len(package_names)} packages...")
        
        ensure_directory(output_dir)
        
        results = {
            "total": len(package_names),
            "success": 0,
            "failed": 0,
            "packages": [],
            "errors": []
        }
        
        with tqdm(total=len(package_names), desc="Backing up APKs", unit="apk") as pbar:
            for i, package_name in enumerate(package_names):
                if progress_callback:
                    progress_callback(i + 1, len(package_names), package_name)
                
                pbar.set_postfix_str(package_name)
                
                result = self._backup_single_package(package_name, output_dir)
                
                if result["success"]:
                    results["success"] += 1
                    results["packages"].append(result)
                else:
                    results["failed"] += 1
                    results["errors"].append(f"{package_name}: {result.get('error', 'Unknown error')}")
                
                pbar.update(1)
        
        # Save backup metadata
        self._save_backup_metadata(results, output_dir)
        
        logger.info(f"APK backup completed: {results['success']}/{results['total']} packages successful")
        
        return results
    
    def _backup_single_package(self, package_name: str, output_dir: Path) -> Dict[str, any]:
        """Backup a single package."""
        result = {
            "package": package_name,
            "success": False,
            "apk_path": "",
            "size": 0,
            "hash": "",
            "version_name": "",
            "version_code": "",
            "error": ""
        }
        
        try:
            # Get package information
            package_info = self.package_manager.get_package_info(package_name)
            if not package_info:
                result["error"] = "Could not get package information"
                return result
            
            if not package_info.apk_path:
                result["error"] = "APK path not found"
                return result
            
            # Determine output file path
            apk_filename = f"{package_name}.apk"
            output_file = output_dir / apk_filename
            
            # Pull APK file
            if not self.puller.pull_file(package_info.apk_path, output_file):
                result["error"] = "Failed to pull APK file"
                return result
            
            # Calculate file hash
            file_hash = calculate_file_hash(output_file)
            if not file_hash:
                result["error"] = "Failed to calculate APK hash"
                return result
            
            # Get file size
            file_size = output_file.stat().st_size
            
            # Update result
            result.update({
                "success": True,
                "apk_path": str(output_file),
                "source_path": package_info.apk_path,
                "size": file_size,
                "hash": file_hash,
                "version_name": package_info.version_name,
                "version_code": package_info.version_code,
            })
            
            logger.debug(f"Successfully backed up APK: {package_name} ({format_size(file_size)})")
            
        except Exception as e:
            result["error"] = str(e)
            logger.error(f"Error backing up APK {package_name}: {e}")
        
        return result
    
    def _save_backup_metadata(self, results: Dict[str, any], output_dir: Path) -> None:
        """Save backup metadata to JSON file."""
        metadata_file = output_dir / "apk_backup_metadata.json"
        
        metadata = {
            "device_serial": self.device.serial,
            "device_info": self.device.get_device_info().__dict__,
            "backup_timestamp": self._get_current_timestamp(),
            "summary": {
                "total_packages": results["total"],
                "successful": results["success"],
                "failed": results["failed"],
                "total_size": sum(pkg.get("size", 0) for pkg in results["packages"]),
            },
            "packages": results["packages"],
            "errors": results["errors"]
        }
        
        try:
            with open(metadata_file, "w", encoding="utf-8") as f:
                json.dump(metadata, f, indent=2, default=str)
            
            logger.info(f"Saved backup metadata to {metadata_file}")
            
        except Exception as e:
            logger.error(f"Failed to save backup metadata: {e}")
    
    def list_installed_packages(self, include_system: bool = False) -> List[Dict[str, any]]:
        """List all installed packages with details."""
        
        logger.info("Retrieving installed packages...")
        
        packages = self.package_manager.get_all_package_info(include_system=include_system)
        
        package_list = []
        for pkg in packages:
            package_list.append({
                "package": pkg.package_name,
                "version_name": pkg.version_name,
                "version_code": pkg.version_code,
                "apk_path": pkg.apk_path,
                "is_system": pkg.is_system,
                "is_enabled": pkg.is_enabled,
            })
        
        logger.info(f"Found {len(package_list)} packages")
        
        return package_list
    
    def get_package_info(self, package_name: str) -> Optional[Dict[str, any]]:
        """Get detailed information about a specific package."""
        
        package_info = self.package_manager.get_package_info(package_name)
        if not package_info:
            return None
        
        # Get additional information
        permissions = self.package_manager.get_package_permissions(package_name)
        size_info = self.package_manager.get_package_size(package_name)
        
        return {
            "package": package_info.package_name,
            "version_name": package_info.version_name,
            "version_code": package_info.version_code,
            "apk_path": package_info.apk_path,
            "target_sdk": package_info.target_sdk,
            "is_system": package_info.is_system,
            "is_enabled": package_info.is_enabled,
            "permissions": permissions,
            "size_info": size_info,
        }
    
    def verify_apk_backup(self, apk_file: Path, expected_hash: str) -> bool:
        """Verify the integrity of a backed up APK file."""
        
        if not apk_file.exists():
            logger.error(f"APK file not found: {apk_file}")
            return False
        
        actual_hash = calculate_file_hash(apk_file)
        if not actual_hash:
            logger.error(f"Failed to calculate hash for {apk_file}")
            return False
        
        if actual_hash.lower() != expected_hash.lower():
            logger.error(f"Hash mismatch for {apk_file}: expected {expected_hash}, got {actual_hash}")
            return False
        
        logger.debug(f"APK verification successful: {apk_file}")
        return True
    
    def _get_current_timestamp(self) -> str:
        """Get current timestamp in ISO format."""
        from datetime import datetime
        return datetime.now().isoformat()
    
    def estimate_backup_size(self, package_names: List[str]) -> Dict[str, any]:
        """Estimate the total size needed for APK backup."""
        
        total_size = 0
        package_count = 0
        failed_count = 0
        
        logger.info(f"Estimating backup size for {len(package_names)} packages...")
        
        for package_name in package_names:
            try:
                package_info = self.package_manager.get_package_info(package_name)
                if package_info and package_info.apk_path:
                    # Try to get APK file size from device
                    apk_size = self.puller.get_file_size(package_info.apk_path)
                    if apk_size:
                        total_size += apk_size
                        package_count += 1
                    else:
                        failed_count += 1
                else:
                    failed_count += 1
                    
            except Exception as e:
                logger.debug(f"Could not estimate size for {package_name}: {e}")
                failed_count += 1
        
        return {
            "total_packages": len(package_names),
            "estimated_packages": package_count,
            "failed_estimates": failed_count,
            "estimated_size": total_size,
            "formatted_size": format_size(total_size)
        }
"""ADB package management utilities."""

import re
from dataclasses import dataclass
from typing import Dict, List, Optional

from .device import ADBDevice, ADBError
from .shell import ShellCommand
from ..util.logging import get_logger

logger = get_logger(__name__)


@dataclass
class PackageInfo:
    """Information about an installed package."""
    
    package_name: str
    version_name: str = ""
    version_code: str = ""
    apk_path: str = ""
    target_sdk: str = ""
    is_system: bool = False
    is_enabled: bool = True


class PackageManager:
    """Utility for managing packages on Android device."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.shell = ShellCommand(device)
    
    def list_packages(self, include_system: bool = False, enabled_only: bool = True) -> List[str]:
        """List installed packages."""
        try:
            cmd = "pm list packages"
            
            if not include_system:
                cmd += " -3"  # Third-party packages only
            
            if not enabled_only:
                cmd += " -d"  # Include disabled packages
            
            output = self.shell.execute(cmd)
            
            packages = []
            for line in output.split("\n"):
                if line.startswith("package:"):
                    package_name = line.replace("package:", "").strip()
                    packages.append(package_name)
            
            logger.debug(f"Found {len(packages)} packages")
            return sorted(packages)
            
        except ADBError as e:
            logger.error(f"Failed to list packages: {e}")
            return []
    
    def get_package_info(self, package_name: str) -> Optional[PackageInfo]:
        """Get detailed information about a package."""
        try:
            # Get package path
            path_output = self.shell.execute(f"pm path {package_name}")
            apk_path = ""
            
            for line in path_output.split("\n"):
                if line.startswith("package:"):
                    apk_path = line.replace("package:", "").strip()
                    break
            
            # Get package info from dumpsys
            info_dict = self.shell.get_package_info(package_name)
            if not info_dict:
                return None
            
            # Check if system package
            is_system = self._is_system_package(apk_path)
            
            return PackageInfo(
                package_name=package_name,
                version_name=info_dict.get("version_name", ""),
                version_code=info_dict.get("version_code", ""),
                apk_path=apk_path,
                target_sdk=info_dict.get("target_sdk", ""),
                is_system=is_system,
                is_enabled=True  # If we can get info, it's likely enabled
            )
            
        except ADBError as e:
            logger.error(f"Failed to get package info for {package_name}: {e}")
            return None
    
    def get_all_package_info(self, include_system: bool = False) -> List[PackageInfo]:
        """Get detailed information for all packages."""
        packages = self.list_packages(include_system=include_system)
        package_info = []
        
        logger.info(f"Getting detailed info for {len(packages)} packages...")
        
        for package_name in packages:
            info = self.get_package_info(package_name)
            if info:
                package_info.append(info)
        
        logger.info(f"Retrieved info for {len(package_info)} packages")
        return package_info
    
    def get_apk_paths(self, package_names: Optional[List[str]] = None) -> Dict[str, str]:
        """Get APK file paths for packages."""
        if package_names is None:
            package_names = self.list_packages(include_system=False)
        
        apk_paths = {}
        
        for package_name in package_names:
            try:
                output = self.shell.execute(f"pm path {package_name}")
                
                for line in output.split("\n"):
                    if line.startswith("package:"):
                        apk_path = line.replace("package:", "").strip()
                        apk_paths[package_name] = apk_path
                        break
                        
            except ADBError as e:
                logger.warning(f"Failed to get APK path for {package_name}: {e}")
        
        return apk_paths
    
    def is_package_installed(self, package_name: str) -> bool:
        """Check if a package is installed."""
        try:
            output = self.shell.execute(f"pm list packages {package_name}")
            return f"package:{package_name}" in output
        except ADBError:
            return False
    
    def get_package_permissions(self, package_name: str) -> List[str]:
        """Get permissions requested by a package."""
        try:
            output = self.shell.execute(f"dumpsys package {package_name}")
            permissions = []
            
            in_permissions_section = False
            for line in output.split("\n"):
                line = line.strip()
                
                if "requested permissions:" in line.lower():
                    in_permissions_section = True
                    continue
                
                if in_permissions_section:
                    if line.startswith("android.permission.") or line.startswith("com."):
                        permissions.append(line)
                    elif not line or line.startswith("install permissions:"):
                        break
            
            return permissions
            
        except ADBError as e:
            logger.error(f"Failed to get permissions for {package_name}: {e}")
            return []
    
    def _is_system_package(self, apk_path: str) -> bool:
        """Determine if package is a system package based on path."""
        system_paths = [
            "/system/app/",
            "/system/priv-app/",
            "/vendor/app/",
            "/oem/app/",
        ]
        
        return any(apk_path.startswith(path) for path in system_paths)
    
    def get_package_size(self, package_name: str) -> Optional[Dict[str, int]]:
        """Get package size information."""
        try:
            output = self.shell.execute(f"dumpsys package {package_name}")
            
            # Look for size information in dumpsys output
            for line in output.split("\n"):
                line = line.strip()
                if "code=" in line and "data=" in line:
                    # Parse size information
                    parts = line.split()
                    sizes = {}
                    
                    for part in parts:
                        if "=" in part:
                            key, value = part.split("=", 1)
                            try:
                                sizes[key] = int(value)
                            except ValueError:
                                pass
                    
                    if sizes:
                        return sizes
            
            return None
            
        except ADBError as e:
            logger.error(f"Failed to get package size for {package_name}: {e}")
            return None
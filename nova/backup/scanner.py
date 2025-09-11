"""Device file scanner for backup operations."""

import fnmatch
import os
from pathlib import Path
from typing import Dict, List, Optional, Set

from ..adb.device import ADBDevice
from ..adb.shell import ShellCommand
from ..config import get_config
from ..util.logging import get_logger

logger = get_logger(__name__)


class ScanResult:
    """Result of a device scan operation."""
    
    def __init__(self):
        self.files: List[Dict[str, any]] = []
        self.total_size: int = 0
        self.total_files: int = 0
        self.errors: List[str] = []
        self.scanned_paths: Set[str] = set()
    
    def add_file(self, file_info: Dict[str, any]) -> None:
        """Add a file to the scan result."""
        self.files.append(file_info)
        self.total_files += 1
        if "size" in file_info:
            try:
                self.total_size += int(file_info["size"])
            except (ValueError, TypeError):
                pass
    
    def add_error(self, error: str) -> None:
        """Add an error to the scan result."""
        self.errors.append(error)
        logger.warning(f"Scan error: {error}")


class DeviceScanner:
    """Scanner for discovering files on Android device."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.shell = ShellCommand(device)
        self.config = get_config()
    
    def scan_device(
        self,
        include_paths: Optional[List[str]] = None,
        exclude_patterns: Optional[List[str]] = None,
        max_file_size_mb: Optional[int] = None
    ) -> ScanResult:
        """Scan device for files to backup."""
        if include_paths is None:
            include_paths = self.config.scanner.include_paths
        
        if exclude_patterns is None:
            exclude_patterns = self.config.scanner.exclude_patterns
        
        if max_file_size_mb is None:
            max_file_size_mb = self.config.scanner.max_file_size_mb
        
        max_file_size_bytes = max_file_size_mb * 1024 * 1024
        
        logger.info(f"Starting device scan with {len(include_paths)} paths")
        
        result = ScanResult()
        
        for path in include_paths:
            try:
                self._scan_path(path, result, exclude_patterns, max_file_size_bytes)
            except Exception as e:
                result.add_error(f"Failed to scan {path}: {e}")
        
        logger.info(
            f"Scan completed: {result.total_files} files, "
            f"{self._format_size(result.total_size)}, {len(result.errors)} errors"
        )
        
        return result
    
    def _scan_path(
        self,
        path: str,
        result: ScanResult,
        exclude_patterns: List[str],
        max_file_size_bytes: int
    ) -> None:
        """Scan a specific path on the device."""
        if not self.shell.file_exists(path):
            result.add_error(f"Path does not exist: {path}")
            return
        
        if not self.shell.is_directory(path):
            # Single file
            file_info = self._get_file_info(path)
            if file_info and self._should_include_file(file_info, exclude_patterns, max_file_size_bytes):
                result.add_file(file_info)
            return
        
        result.scanned_paths.add(path)
        logger.debug(f"Scanning directory: {path}")
        
        try:
            # Use find command for efficient directory traversal
            files = self.shell.find_files(path, "*")
            
            for file_path in files:
                if self._should_skip_path(file_path, exclude_patterns):
                    continue
                
                file_info = self._get_file_info(file_path)
                if file_info and self._should_include_file(file_info, exclude_patterns, max_file_size_bytes):
                    result.add_file(file_info)
                    
        except Exception as e:
            result.add_error(f"Error scanning {path}: {e}")
    
    def _get_file_info(self, file_path: str) -> Optional[Dict[str, any]]:
        """Get detailed information about a file."""
        try:
            stats = self.shell.get_file_stats(file_path)
            if not stats:
                return None
            
            # Check if it's a regular file
            if not stats.get("name") or "size" not in stats:
                return None
            
            file_info = {
                "path": file_path,
                "name": os.path.basename(file_path),
                "size": int(stats["size"]) if stats["size"].isdigit() else 0,
                "mtime": int(stats["mtime"]) if stats.get("mtime", "").isdigit() else 0,
                "mode": stats.get("mode", ""),
                "uid": stats.get("uid", ""),
                "gid": stats.get("gid", ""),
            }
            
            return file_info
            
        except Exception as e:
            logger.debug(f"Failed to get file info for {file_path}: {e}")
            return None
    
    def _should_include_file(
        self,
        file_info: Dict[str, any],
        exclude_patterns: List[str],
        max_file_size_bytes: int
    ) -> bool:
        """Determine if a file should be included in backup."""
        file_path = file_info["path"]
        file_size = file_info.get("size", 0)
        
        # Check file size limit
        if file_size > max_file_size_bytes:
            logger.debug(f"Skipping large file: {file_path} ({self._format_size(file_size)})")
            return False
        
        # Check exclude patterns
        if self._should_skip_path(file_path, exclude_patterns):
            return False
        
        # Skip empty files (usually system files)
        if file_size == 0:
            return False
        
        return True
    
    def _should_skip_path(self, file_path: str, exclude_patterns: List[str]) -> bool:
        """Check if a path should be skipped based on exclude patterns."""
        file_name = os.path.basename(file_path)
        
        for pattern in exclude_patterns:
            if fnmatch.fnmatch(file_name, pattern) or fnmatch.fnmatch(file_path, pattern):
                logger.debug(f"Excluding file matching pattern '{pattern}': {file_path}")
                return True
        
        # Skip hidden files and directories
        if file_name.startswith("."):
            return True
        
        # Skip system directories
        system_dirs = ["/proc", "/sys", "/dev", "/system", "/vendor"]
        if any(file_path.startswith(d) for d in system_dirs):
            return True
        
        return False
    
    def scan_specific_files(self, file_paths: List[str]) -> ScanResult:
        """Scan specific files provided by user."""
        logger.info(f"Scanning {len(file_paths)} specific files")
        
        result = ScanResult()
        
        for file_path in file_paths:
            if self.shell.file_exists(file_path):
                file_info = self._get_file_info(file_path)
                if file_info:
                    result.add_file(file_info)
                else:
                    result.add_error(f"Could not get info for: {file_path}")
            else:
                result.add_error(f"File not found: {file_path}")
        
        return result
    
    def get_storage_usage(self) -> Dict[str, int]:
        """Get storage usage information for common directories."""
        directories = [
            "/sdcard/DCIM",
            "/sdcard/Pictures", 
            "/sdcard/Movies",
            "/sdcard/Music",
            "/sdcard/Documents",
            "/sdcard/Download",
        ]
        
        usage = {}
        
        for directory in directories:
            try:
                if self.shell.file_exists(directory):
                    # Use du command to get directory size
                    output = self.shell.execute(f"du -s {directory} 2>/dev/null || echo '0'")
                    size_kb = output.split()[0] if output.split() else "0"
                    usage[directory] = int(size_kb) * 1024  # Convert to bytes
                else:
                    usage[directory] = 0
            except Exception as e:
                logger.debug(f"Failed to get usage for {directory}: {e}")
                usage[directory] = 0
        
        return usage
    
    def _format_size(self, size_bytes: int) -> str:
        """Format file size in human readable format."""
        from ..util.paths import format_size
        return format_size(size_bytes)
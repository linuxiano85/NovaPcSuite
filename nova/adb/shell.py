"""ADB shell command execution utilities."""

import re
import shlex
from typing import Dict, List, Optional, Tuple

from .device import ADBDevice, ADBError
from ..util.logging import get_logger

logger = get_logger(__name__)


class ShellCommand:
    """Utility for executing shell commands on Android device."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
    
    def execute(self, command: str, timeout: int = 30) -> str:
        """Execute a shell command on the device."""
        return self.device._run_command(["shell", command], timeout=timeout)
    
    def execute_as_root(self, command: str, timeout: int = 30) -> str:
        """Execute a command as root (if available)."""
        if self.device.has_root():
            return self.execute(f"su -c {shlex.quote(command)}", timeout)
        else:
            raise ADBError("Root access not available")
    
    def file_exists(self, path: str) -> bool:
        """Check if a file or directory exists on the device."""
        try:
            result = self.execute(f"test -e {shlex.quote(path)} && echo exists")
            return "exists" in result
        except ADBError:
            return False
    
    def is_directory(self, path: str) -> bool:
        """Check if path is a directory."""
        try:
            result = self.execute(f"test -d {shlex.quote(path)} && echo directory")
            return "directory" in result
        except ADBError:
            return False
    
    def list_directory(self, path: str, long_format: bool = True) -> List[Dict[str, str]]:
        """List directory contents with file information."""
        try:
            if long_format:
                output = self.execute(f"ls -la {shlex.quote(path)}")
                return self._parse_ls_output(output)
            else:
                output = self.execute(f"ls {shlex.quote(path)}")
                return [{"name": name.strip()} for name in output.split("\n") if name.strip()]
        except ADBError as e:
            logger.error(f"Failed to list directory {path}: {e}")
            return []
    
    def _parse_ls_output(self, output: str) -> List[Dict[str, str]]:
        """Parse ls -la output into structured data."""
        files = []
        lines = output.strip().split("\n")
        
        for line in lines:
            if not line or line.startswith("total"):
                continue
            
            # Parse ls -la format: permissions user group size date time name
            parts = line.split(None, 8)
            if len(parts) >= 9:
                permissions = parts[0]
                user = parts[2]
                group = parts[3]
                size = parts[4]
                date_parts = parts[5:8]
                name = parts[8]
                
                files.append({
                    "permissions": permissions,
                    "user": user,
                    "group": group,
                    "size": size,
                    "date": " ".join(date_parts),
                    "name": name,
                    "is_directory": permissions.startswith("d"),
                    "is_file": permissions.startswith("-"),
                })
        
        return files
    
    def find_files(self, path: str, pattern: str = "*", max_depth: Optional[int] = None) -> List[str]:
        """Find files matching pattern in directory tree."""
        try:
            cmd = f"find {shlex.quote(path)} -name {shlex.quote(pattern)}"
            
            if max_depth is not None:
                cmd += f" -maxdepth {max_depth}"
            
            output = self.execute(cmd, timeout=60)
            return [line.strip() for line in output.split("\n") if line.strip()]
            
        except ADBError as e:
            logger.error(f"Failed to find files in {path}: {e}")
            return []
    
    def get_file_stats(self, path: str) -> Optional[Dict[str, str]]:
        """Get detailed file statistics."""
        try:
            # Try stat command with custom format
            output = self.execute(f"stat -c '%n|%s|%Y|%X|%Z|%f|%u|%g' {shlex.quote(path)}")
            
            parts = output.split("|")
            if len(parts) >= 8:
                return {
                    "name": parts[0],
                    "size": parts[1],
                    "mtime": parts[2],  # modification time
                    "atime": parts[3],  # access time
                    "ctime": parts[4],  # status change time
                    "mode": parts[5],   # file mode
                    "uid": parts[6],    # user ID
                    "gid": parts[7],    # group ID
                }
        except ADBError:
            # Fallback to ls -la for basic info
            try:
                output = self.execute(f"ls -la {shlex.quote(path)}")
                files = self._parse_ls_output(output)
                if files:
                    return files[0]
            except ADBError:
                pass
        
        return None
    
    def get_package_info(self, package_name: str) -> Optional[Dict[str, str]]:
        """Get information about an installed package."""
        try:
            # Get package info using dumpsys
            output = self.execute(f"dumpsys package {package_name}")
            
            info = {}
            for line in output.split("\n"):
                line = line.strip()
                
                if "versionName=" in line:
                    info["version_name"] = line.split("versionName=")[1]
                elif "versionCode=" in line:
                    info["version_code"] = line.split("versionCode=")[1].split()[0]
                elif "codePath=" in line:
                    info["code_path"] = line.split("codePath=")[1]
                elif "targetSdk=" in line:
                    info["target_sdk"] = line.split("targetSdk=")[1]
            
            return info if info else None
            
        except ADBError as e:
            logger.error(f"Failed to get package info for {package_name}: {e}")
            return None
    
    def check_permissions(self, permission: str) -> bool:
        """Check if a specific permission is available."""
        try:
            # This is a simplified check - full permission checking is complex
            output = self.execute(f"pm list permissions | grep {permission}")
            return bool(output.strip())
        except ADBError:
            return False
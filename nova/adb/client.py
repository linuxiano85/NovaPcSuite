"""ADB client wrapper for device communication."""

import subprocess
import typing as t
from pathlib import Path


class ADBError(Exception):
    """ADB operation error."""
    pass


class ADBClient:
    """Simple ADB client wrapper."""
    
    def __init__(self, adb_path: str = "adb", timeout: int = 30) -> None:
        """Initialize ADB client.
        
        Args:
            adb_path: Path to adb executable
            timeout: Command timeout in seconds
        """
        self.adb_path = adb_path
        self.timeout = timeout
    
    def _run_command(self, args: t.List[str]) -> str:
        """Run ADB command and return output.
        
        Args:
            args: Command arguments
            
        Returns:
            Command output
            
        Raises:
            ADBError: If command fails
        """
        try:
            result = subprocess.run(
                [self.adb_path] + args,
                capture_output=True,
                text=True,
                timeout=self.timeout,
                check=True
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError as e:
            raise ADBError(f"ADB command failed: {e.stderr}") from e
        except subprocess.TimeoutExpired as e:
            raise ADBError(f"ADB command timed out after {self.timeout}s") from e
        except FileNotFoundError as e:
            raise ADBError("ADB not found. Please install Android platform tools.") from e
    
    def list_devices(self) -> t.List[str]:
        """List connected devices.
        
        Returns:
            List of device IDs
        """
        output = self._run_command(["devices"])
        devices = []
        
        for line in output.split('\n')[1:]:  # Skip header
            if line.strip() and '\t' in line:
                device_id, status = line.split('\t')
                if status == "device":
                    devices.append(device_id)
        
        return devices
    
    def shell(self, device_id: str, command: str) -> str:
        """Execute shell command on device.
        
        Args:
            device_id: Device identifier
            command: Shell command to execute
            
        Returns:
            Command output
        """
        return self._run_command(["-s", device_id, "shell", command])
    
    def pull(self, device_id: str, remote_path: str, local_path: Path) -> None:
        """Pull file from device.
        
        Args:
            device_id: Device identifier
            remote_path: Remote file path
            local_path: Local destination path
        """
        # Ensure parent directory exists
        local_path.parent.mkdir(parents=True, exist_ok=True)
        
        self._run_command(["-s", device_id, "pull", remote_path, str(local_path)])
    
    def get_property(self, device_id: str, prop: str) -> str:
        """Get device property.
        
        Args:
            device_id: Device identifier
            prop: Property name
            
        Returns:
            Property value
        """
        return self.shell(device_id, f"getprop {prop}")
"""ADB device management and communication."""

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional

from tenacity import retry, stop_after_attempt, wait_exponential

from ..util.logging import get_logger

logger = get_logger(__name__)


@dataclass
class DeviceInfo:
    """Information about an Android device."""
    
    serial: str
    model: str
    brand: str
    android_version: str
    sdk_version: str
    state: str = "device"
    
    @property
    def display_name(self) -> str:
        """Get a human-readable device name."""
        return f"{self.brand} {self.model} ({self.serial})"


class ADBError(Exception):
    """ADB command execution error."""
    pass


class ADBDevice:
    """Represents an ADB-connected Android device."""
    
    def __init__(self, serial: str, adb_path: str = "adb"):
        self.serial = serial
        self.adb_path = adb_path
        self._device_info: Optional[DeviceInfo] = None
    
    @retry(stop=stop_after_attempt(3), wait=wait_exponential(multiplier=1, min=1, max=10))
    def _run_command(self, command: List[str], timeout: int = 30) -> str:
        """Run an ADB command with retry logic."""
        cmd = [self.adb_path, "-s", self.serial] + command
        
        try:
            logger.debug(f"Running ADB command: {' '.join(cmd)}")
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                check=True
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError as e:
            error_msg = f"ADB command failed: {' '.join(cmd)}\nError: {e.stderr}"
            logger.error(error_msg)
            raise ADBError(error_msg) from e
        except subprocess.TimeoutExpired as e:
            error_msg = f"ADB command timed out: {' '.join(cmd)}"
            logger.error(error_msg)
            raise ADBError(error_msg) from e
    
    def get_device_info(self) -> DeviceInfo:
        """Get detailed device information."""
        if self._device_info is not None:
            return self._device_info
        
        try:
            # Get basic properties
            props = {
                "model": self._run_command(["shell", "getprop", "ro.product.model"]),
                "brand": self._run_command(["shell", "getprop", "ro.product.brand"]),
                "android_version": self._run_command(["shell", "getprop", "ro.build.version.release"]),
                "sdk_version": self._run_command(["shell", "getprop", "ro.build.version.sdk"]),
            }
            
            self._device_info = DeviceInfo(
                serial=self.serial,
                model=props["model"],
                brand=props["brand"],
                android_version=props["android_version"],
                sdk_version=props["sdk_version"],
                state="device"
            )
            
            logger.info(f"Device info: {self._device_info.display_name}")
            return self._device_info
            
        except ADBError as e:
            logger.error(f"Failed to get device info for {self.serial}: {e}")
            raise
    
    def is_online(self) -> bool:
        """Check if device is online and accessible."""
        try:
            self._run_command(["shell", "echo", "test"], timeout=10)
            return True
        except ADBError:
            return False
    
    def has_root(self) -> bool:
        """Check if device has root access."""
        try:
            output = self._run_command(["shell", "id"], timeout=10)
            return "uid=0(root)" in output
        except ADBError:
            return False
    
    def get_storage_info(self) -> Dict[str, int]:
        """Get storage information (used/available space)."""
        try:
            # Get storage info for main storage
            output = self._run_command(["shell", "df", "/sdcard"])
            lines = output.strip().split("\n")
            
            if len(lines) >= 2:
                # Parse df output (filesystem, size, used, available, use%, mount)
                parts = lines[1].split()
                if len(parts) >= 4:
                    return {
                        "total": int(parts[1]) * 1024,  # Convert KB to bytes
                        "used": int(parts[2]) * 1024,
                        "available": int(parts[3]) * 1024,
                    }
            
            logger.warning("Could not parse storage info")
            return {"total": 0, "used": 0, "available": 0}
            
        except ADBError as e:
            logger.error(f"Failed to get storage info: {e}")
            return {"total": 0, "used": 0, "available": 0}


def check_adb_available(adb_path: str = "adb") -> bool:
    """Check if ADB is available and working."""
    try:
        result = subprocess.run(
            [adb_path, "version"],
            capture_output=True,
            text=True,
            timeout=10
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False


def list_devices(adb_path: str = "adb") -> List[ADBDevice]:
    """List all connected ADB devices."""
    if not check_adb_available(adb_path):
        raise ADBError("ADB is not available or not in PATH")
    
    try:
        result = subprocess.run(
            [adb_path, "devices"],
            capture_output=True,
            text=True,
            timeout=10,
            check=True
        )
        
        devices = []
        lines = result.stdout.strip().split("\n")[1:]  # Skip header
        
        for line in lines:
            if line.strip():
                parts = line.split("\t")
                if len(parts) >= 2 and parts[1] == "device":
                    serial = parts[0]
                    devices.append(ADBDevice(serial, adb_path))
        
        return devices
        
    except subprocess.CalledProcessError as e:
        raise ADBError(f"Failed to list devices: {e.stderr}") from e


def get_device_by_serial(serial: str, adb_path: str = "adb") -> Optional[ADBDevice]:
    """Get a specific device by serial number."""
    devices = list_devices(adb_path)
    
    for device in devices:
        if device.serial == serial:
            return device
    
    return None
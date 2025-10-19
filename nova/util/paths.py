"""Utility functions for path operations."""

import os
import shutil
from pathlib import Path
from typing import List, Optional

from ..util.logging import get_logger

logger = get_logger(__name__)


def ensure_directory(path: Path) -> Path:
    """Ensure directory exists, creating it if necessary."""
    path.mkdir(parents=True, exist_ok=True)
    return path


def safe_filename(filename: str) -> str:
    """Create a safe filename by removing/replacing problematic characters."""
    # Replace problematic characters
    replacements = {
        "/": "_",
        "\\": "_",
        ":": "_",
        "*": "_",
        "?": "_",
        '"': "_",
        "<": "_",
        ">": "_",
        "|": "_",
        "\n": "_",
        "\r": "_",
        "\t": "_",
    }
    
    safe_name = filename
    for old, new in replacements.items():
        safe_name = safe_name.replace(old, new)
    
    # Remove leading/trailing whitespace and dots
    safe_name = safe_name.strip(" .")
    
    # Ensure we have a name (fallback)
    if not safe_name:
        safe_name = "unknown"
    
    return safe_name


def get_backup_path(device_serial: str, timestamp: Optional[str] = None) -> Path:
    """Get backup directory path for a device."""
    from ..config import get_config
    
    config = get_config()
    backup_root = config.backup_root
    
    if timestamp is None:
        from datetime import datetime
        timestamp = datetime.now().strftime("%Y-%m-%d_%H%M%S")
    
    device_dir = safe_filename(device_serial)
    return backup_root / device_dir / timestamp


def relative_to_device_root(device_path: str) -> Path:
    """Convert device absolute path to relative path for backup storage."""
    # Remove leading slash and convert to Path
    if device_path.startswith("/"):
        device_path = device_path[1:]
    
    return Path(device_path)


def find_files_by_pattern(directory: Path, patterns: List[str]) -> List[Path]:
    """Find files matching patterns in directory."""
    files = []
    
    for pattern in patterns:
        files.extend(directory.rglob(pattern))
    
    return sorted(set(files))


def get_available_space(path: Path) -> int:
    """Get available space in bytes for the given path."""
    try:
        stat = shutil.disk_usage(path)
        return stat.free
    except Exception as e:
        logger.warning(f"Could not get disk usage for {path}: {e}")
        return 0


def format_size(size_bytes: int) -> str:
    """Format file size in human readable format."""
    if size_bytes == 0:
        return "0 B"
    
    size_names = ["B", "KB", "MB", "GB", "TB"]
    i = 0
    while size_bytes >= 1024.0 and i < len(size_names) - 1:
        size_bytes /= 1024.0
        i += 1
    
    return f"{size_bytes:.1f} {size_names[i]}"
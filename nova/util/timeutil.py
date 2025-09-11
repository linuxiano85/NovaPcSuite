"""Utility functions for time operations."""

from datetime import datetime, timezone
from typing import Union


def now_iso() -> str:
    """Get current timestamp in ISO 8601 format."""
    return datetime.now(timezone.utc).isoformat()


def timestamp_to_iso(timestamp: Union[int, float]) -> str:
    """Convert Unix timestamp to ISO 8601 format."""
    dt = datetime.fromtimestamp(timestamp, timezone.utc)
    return dt.isoformat()


def iso_to_timestamp(iso_string: str) -> float:
    """Convert ISO 8601 string to Unix timestamp."""
    dt = datetime.fromisoformat(iso_string.replace("Z", "+00:00"))
    return dt.timestamp()


def format_duration(seconds: float) -> str:
    """Format duration in human readable format."""
    if seconds < 60:
        return f"{seconds:.1f}s"
    elif seconds < 3600:
        minutes = seconds / 60
        return f"{minutes:.1f}m"
    else:
        hours = seconds / 3600
        return f"{hours:.1f}h"


def parse_timestamp(timestamp_str: str) -> datetime:
    """Parse various timestamp formats to datetime."""
    # Try different common formats
    formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d_%H%M%S", 
        "%Y%m%d_%H%M%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S.%f",
        "%Y-%m-%dT%H:%M:%S.%fZ",
    ]
    
    for fmt in formats:
        try:
            return datetime.strptime(timestamp_str, fmt)
        except ValueError:
            continue
    
    # Try ISO format parsing
    try:
        return datetime.fromisoformat(timestamp_str.replace("Z", "+00:00"))
    except ValueError:
        pass
    
    raise ValueError(f"Unable to parse timestamp: {timestamp_str}")


def generate_backup_id() -> str:
    """Generate a backup ID based on current timestamp."""
    return datetime.now().strftime("%Y-%m-%d_%H%M%S")
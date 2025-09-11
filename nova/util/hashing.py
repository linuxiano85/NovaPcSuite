"""Utility functions for hashing operations."""

import hashlib
from pathlib import Path
from typing import BinaryIO, Optional

from ..util.logging import get_logger

logger = get_logger(__name__)


def calculate_file_hash(
    file_path: Path, 
    algorithm: str = "sha256", 
    chunk_size: int = 8192
) -> Optional[str]:
    """Calculate hash of a file."""
    try:
        hasher = hashlib.new(algorithm)
        
        with open(file_path, "rb") as f:
            while chunk := f.read(chunk_size):
                hasher.update(chunk)
        
        return hasher.hexdigest()
    except Exception as e:
        logger.error(f"Failed to calculate hash for {file_path}: {e}")
        return None


def calculate_stream_hash(
    stream: BinaryIO, 
    algorithm: str = "sha256", 
    chunk_size: int = 8192
) -> str:
    """Calculate hash of a binary stream."""
    hasher = hashlib.new(algorithm)
    
    while chunk := stream.read(chunk_size):
        hasher.update(chunk)
    
    return hasher.hexdigest()


def verify_file_integrity(file_path: Path, expected_hash: str, algorithm: str = "sha256") -> bool:
    """Verify file integrity against expected hash."""
    actual_hash = calculate_file_hash(file_path, algorithm)
    
    if actual_hash is None:
        return False
    
    return actual_hash.lower() == expected_hash.lower()


def calculate_bytes_hash(data: bytes, algorithm: str = "sha256") -> str:
    """Calculate hash of bytes data."""
    hasher = hashlib.new(algorithm)
    hasher.update(data)
    return hasher.hexdigest()
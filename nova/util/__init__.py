"""Utility module initialization."""

from .compression import compress_file, decompress_file, estimate_compression_ratio
from .hashing import (
    calculate_bytes_hash,
    calculate_file_hash,
    calculate_stream_hash,
    verify_file_integrity,
)
from .logging import get_logger, setup_logging
from .paths import (
    ensure_directory,
    find_files_by_pattern,
    format_size,
    get_available_space,
    get_backup_path,
    relative_to_device_root,
    safe_filename,
)
from .timeutil import (
    format_duration,
    generate_backup_id,
    iso_to_timestamp,
    now_iso,
    parse_timestamp,
    timestamp_to_iso,
)

__all__ = [
    # compression
    "compress_file",
    "decompress_file", 
    "estimate_compression_ratio",
    # hashing
    "calculate_bytes_hash",
    "calculate_file_hash",
    "calculate_stream_hash",
    "verify_file_integrity",
    # logging
    "get_logger",
    "setup_logging",
    # paths
    "ensure_directory",
    "find_files_by_pattern",
    "format_size",
    "get_available_space",
    "get_backup_path",
    "relative_to_device_root",
    "safe_filename",
    # timeutil
    "format_duration",
    "generate_backup_id",
    "iso_to_timestamp",
    "now_iso",
    "parse_timestamp",
    "timestamp_to_iso",
]
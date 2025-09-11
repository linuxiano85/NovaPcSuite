"""Backup module initialization."""

from .executor import BackupExecutor
from .manifest import (
    ApkEntry,
    BackupManifest,
    DeviceBackupInfo,
    ExportedData,
    FileEntry,
    ManifestManager,
    categorize_file,
)
from .restore import RestoreExecutor
from .rules import BackupRule, RuleEngine
from .scanner import DeviceScanner, ScanResult
from .storage import BackupStorage

__all__ = [
    # scanner
    "DeviceScanner",
    "ScanResult",
    # executor
    "BackupExecutor",
    # restore
    "RestoreExecutor", 
    # storage
    "BackupStorage",
    # manifest
    "BackupManifest",
    "FileEntry",
    "ApkEntry",
    "DeviceBackupInfo",
    "ExportedData",
    "ManifestManager",
    "categorize_file",
    # rules
    "BackupRule",
    "RuleEngine",
]
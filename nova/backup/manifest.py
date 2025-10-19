"""Backup manifest data models."""

import typing as t
from datetime import datetime
from pathlib import Path

from pydantic import BaseModel, Field


class FileEntry(BaseModel):
    """Represents a backed up file."""
    
    relative_path: str = Field(description="Relative path from backup root")
    original_path: str = Field(description="Original path on device")
    size: int = Field(description="File size in bytes")
    sha256: str = Field(description="SHA256 hash of file content")
    modified_time: t.Optional[datetime] = Field(None, description="Last modified time")
    permissions: t.Optional[str] = Field(None, description="File permissions")


class BackupManifest(BaseModel):
    """Backup manifest containing metadata and file list."""
    
    version: str = Field(default="1.0", description="Manifest format version")
    created_at: datetime = Field(default_factory=datetime.now, description="Backup creation time")
    device_id: str = Field(description="Device identifier")
    device_info: t.Dict[str, str] = Field(description="Device information")
    backup_type: str = Field(default="full", description="Type of backup (full, incremental)")
    total_files: int = Field(description="Total number of files backed up")
    total_size: int = Field(description="Total size of backed up data in bytes")
    files: t.List[FileEntry] = Field(description="List of backed up files")
    
    def save(self, path: Path) -> None:
        """Save manifest to JSON file.
        
        Args:
            path: Path to save manifest file
        """
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, 'w', encoding='utf-8') as f:
            f.write(self.model_dump_json(indent=2))
    
    @classmethod
    def load(cls, path: Path) -> 'BackupManifest':
        """Load manifest from JSON file.
        
        Args:
            path: Path to manifest file
            
        Returns:
            BackupManifest instance
        """
        with open(path, 'r', encoding='utf-8') as f:
            return cls.model_validate_json(f.read())
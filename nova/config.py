"""Configuration management for NovaPcSuite."""

import os
from pathlib import Path
from typing import Dict, List, Optional

from pydantic import BaseModel, Field
from ruamel.yaml import YAML


class ScannerConfig(BaseModel):
    """Configuration for directory scanning."""
    
    include_paths: List[str] = Field(
        default=[
            "/sdcard/DCIM",
            "/sdcard/Pictures", 
            "/sdcard/Movies",
            "/sdcard/Music",
            "/sdcard/Documents",
            "/sdcard/Download",
            "/sdcard/WhatsApp/Media",
            "/sdcard/Telegram",
        ],
        description="Default paths to scan on device"
    )
    exclude_patterns: List[str] = Field(
        default=[
            "*.tmp",
            "*.cache",
            ".thumbnails",
            ".trash",
        ],
        description="File patterns to exclude from backup"
    )
    max_file_size_mb: int = Field(default=1024, description="Maximum file size in MB")


class BackupConfig(BaseModel):
    """Configuration for backup operations."""
    
    incremental: bool = Field(default=True, description="Enable incremental backups")
    hash_algorithm: str = Field(default="sha256", description="Hash algorithm for file integrity")
    compression: bool = Field(default=False, description="Enable compression (future)")
    encryption: bool = Field(default=False, description="Enable encryption (future)")
    verify_integrity: bool = Field(default=True, description="Verify file integrity after backup")


class ExportConfig(BaseModel):
    """Configuration for data export."""
    
    contact_formats: List[str] = Field(default=["vcf", "csv"], description="Contact export formats")
    include_call_logs: bool = Field(default=True, description="Include call logs in exports")
    include_sms: bool = Field(default=True, description="Include SMS in exports")
    date_format: str = Field(default="%Y-%m-%d %H:%M:%S", description="Date format for exports")


class NovaConfig(BaseModel):
    """Main configuration for NovaPcSuite."""
    
    backup_root: Path = Field(
        default_factory=lambda: Path.home() / ".local/share/novapcsuite/backups",
        description="Root directory for backups"
    )
    config_dir: Path = Field(
        default_factory=lambda: Path.home() / ".config/novapcsuite",
        description="Configuration directory"
    )
    
    scanner: ScannerConfig = Field(default_factory=ScannerConfig)
    backup: BackupConfig = Field(default_factory=BackupConfig)
    export: ExportConfig = Field(default_factory=ExportConfig)
    
    # Runtime settings
    adb_path: str = Field(default="adb", description="Path to ADB binary")
    fastboot_path: str = Field(default="fastboot", description="Path to fastboot binary")
    log_level: str = Field(default="INFO", description="Logging level")
    max_concurrent_operations: int = Field(default=4, description="Max concurrent operations")
    
    class Config:
        """Pydantic configuration."""
        
        validate_assignment = True
        use_enum_values = True


def load_config(config_path: Optional[Path] = None) -> NovaConfig:
    """Load configuration from file or create default."""
    
    if config_path is None:
        config_path = Path.home() / ".config/novapcsuite/config.yaml"
    
    if config_path.exists():
        yaml = YAML(typ="safe")
        with open(config_path, "r") as f:
            data = yaml.load(f) or {}
        return NovaConfig(**data)
    else:
        # Create default config
        config = NovaConfig()
        save_config(config, config_path)
        return config


def save_config(config: NovaConfig, config_path: Optional[Path] = None) -> None:
    """Save configuration to file."""
    
    if config_path is None:
        config_path = Path.home() / ".config/novapcsuite/config.yaml"
    
    # Ensure directory exists
    config_path.parent.mkdir(parents=True, exist_ok=True)
    
    yaml = YAML()
    yaml.default_flow_style = False
    
    with open(config_path, "w") as f:
        yaml.dump(config.model_dump(), f)


def get_config() -> NovaConfig:
    """Get the global configuration instance."""
    
    if not hasattr(get_config, "_config"):
        get_config._config = load_config()
    
    return get_config._config
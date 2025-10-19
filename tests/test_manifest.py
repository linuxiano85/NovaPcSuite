"""Tests for backup manifest functionality."""

import json
import tempfile
from pathlib import Path

import pytest

from nova.backup.manifest import (
    BackupManifest,
    DeviceBackupInfo,
    FileEntry,
    ManifestManager,
    categorize_file,
)


class TestBackupManifest:
    """Test backup manifest creation and management."""
    
    def test_manifest_creation(self):
        """Test creating a new manifest."""
        device_info = DeviceBackupInfo(
            serial="test123",
            model="Test Model",
            brand="Test Brand",
            android_version="12",
            sdk="31"
        )
        
        manifest = BackupManifest(device=device_info)
        
        assert manifest.version == 1
        assert manifest.device.serial == "test123"
        assert manifest.device.model == "Test Model"
        assert len(manifest.files) == 0
        assert len(manifest.apk) == 0
    
    def test_file_entry_creation(self):
        """Test creating file entries."""
        file_entry = FileEntry(
            path="/sdcard/test.jpg",
            category="image",
            size=12345,
            mtime=1640995200,
            hash="abcdef123456",
            rel_dst="files/sdcard/test.jpg"
        )
        
        assert file_entry.path == "/sdcard/test.jpg"
        assert file_entry.category == "image"
        assert file_entry.size == 12345
    
    def test_manifest_save_load(self):
        """Test saving and loading manifest."""
        with tempfile.TemporaryDirectory() as temp_dir:
            backup_path = Path(temp_dir)
            
            # Create manifest
            device_info = DeviceBackupInfo(
                serial="test123",
                model="Test Model", 
                brand="Test Brand",
                android_version="12",
                sdk="31"
            )
            
            manifest = BackupManifest(device=device_info)
            
            # Add file entry
            file_entry = FileEntry(
                path="/sdcard/test.jpg",
                category="image",
                size=12345,
                mtime=1640995200,
                hash="abcdef123456",
                rel_dst="files/sdcard/test.jpg"
            )
            manifest.files.append(file_entry)
            
            # Save manifest
            manager = ManifestManager(backup_path)
            manager.save_manifest(manifest)
            
            # Load manifest
            loaded_manifest = manager.load_manifest()
            
            assert loaded_manifest is not None
            assert loaded_manifest.device.serial == "test123"
            assert len(loaded_manifest.files) == 1
            assert loaded_manifest.files[0].path == "/sdcard/test.jpg"


class TestFileCategorization:
    """Test file categorization logic."""
    
    def test_image_categorization(self):
        """Test image file categorization."""
        assert categorize_file("/sdcard/DCIM/photo.jpg") == "image"
        assert categorize_file("/sdcard/Pictures/image.png") == "image"
        assert categorize_file("/data/test.jpeg") == "image"
    
    def test_video_categorization(self):
        """Test video file categorization."""
        assert categorize_file("/sdcard/Movies/video.mp4") == "video"
        assert categorize_file("/sdcard/test.mkv") == "video"
    
    def test_audio_categorization(self):
        """Test audio file categorization."""
        assert categorize_file("/sdcard/Music/song.mp3") == "audio"
        assert categorize_file("/sdcard/test.wav") == "audio"
    
    def test_document_categorization(self):
        """Test document file categorization."""
        assert categorize_file("/sdcard/Documents/file.pdf") == "document"
        assert categorize_file("/sdcard/test.docx") == "document"
    
    def test_messaging_categorization(self):
        """Test messaging app file categorization."""
        assert categorize_file("/sdcard/WhatsApp/Media/photo.jpg") == "messaging"
        assert categorize_file("/sdcard/Telegram/video.mp4") == "messaging"
    
    def test_other_categorization(self):
        """Test unknown file categorization."""
        assert categorize_file("/sdcard/unknown.xyz") == "other"
        assert categorize_file("/system/app/test.apk") == "other"
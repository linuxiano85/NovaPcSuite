"""Tests for device scanner functionality."""

from unittest.mock import MagicMock, patch

import pytest

from nova.backup.scanner import DeviceScanner, ScanResult


class TestScanResult:
    """Test scan result management."""
    
    def test_scan_result_creation(self):
        """Test creating a scan result."""
        result = ScanResult()
        
        assert result.total_files == 0
        assert result.total_size == 0
        assert len(result.files) == 0
        assert len(result.errors) == 0
    
    def test_add_file(self):
        """Test adding files to scan result."""
        result = ScanResult()
        
        file_info = {
            "path": "/sdcard/test.jpg",
            "size": 12345,
            "name": "test.jpg"
        }
        
        result.add_file(file_info)
        
        assert result.total_files == 1
        assert result.total_size == 12345
        assert len(result.files) == 1
        assert result.files[0]["path"] == "/sdcard/test.jpg"
    
    def test_add_error(self):
        """Test adding errors to scan result."""
        result = ScanResult()
        
        result.add_error("Test error")
        
        assert len(result.errors) == 1
        assert result.errors[0] == "Test error"


class TestDeviceScanner:
    """Test device scanner functionality."""
    
    def test_scanner_creation(self):
        """Test creating a device scanner."""
        mock_device = MagicMock()
        scanner = DeviceScanner(mock_device)
        
        assert scanner.device == mock_device
    
    @patch('nova.backup.scanner.ShellCommand')
    def test_should_skip_path(self, mock_shell):
        """Test path skipping logic."""
        mock_device = MagicMock()
        scanner = DeviceScanner(mock_device)
        
        # Test exclude patterns
        exclude_patterns = ["*.tmp", "*.cache"]
        
        assert scanner._should_skip_path("/sdcard/test.tmp", exclude_patterns) == True
        assert scanner._should_skip_path("/sdcard/test.cache", exclude_patterns) == True
        assert scanner._should_skip_path("/sdcard/test.jpg", exclude_patterns) == False
        
        # Test hidden files
        assert scanner._should_skip_path("/sdcard/.hidden", []) == True
        assert scanner._should_skip_path("/sdcard/normal.jpg", []) == False
        
        # Test system directories
        assert scanner._should_skip_path("/system/app/test.apk", []) == True
        assert scanner._should_skip_path("/sdcard/test.jpg", []) == False
    
    @patch('nova.backup.scanner.ShellCommand')
    def test_should_include_file(self, mock_shell):
        """Test file inclusion logic."""
        mock_device = MagicMock()
        scanner = DeviceScanner(mock_device)
        
        # Test file size limit
        file_info = {
            "path": "/sdcard/test.jpg",
            "size": 100 * 1024 * 1024  # 100MB
        }
        
        # Should include file under size limit
        assert scanner._should_include_file(file_info, [], 200 * 1024 * 1024) == True
        
        # Should exclude file over size limit
        assert scanner._should_include_file(file_info, [], 50 * 1024 * 1024) == False
        
        # Should exclude empty files
        file_info["size"] = 0
        assert scanner._should_include_file(file_info, [], 200 * 1024 * 1024) == False
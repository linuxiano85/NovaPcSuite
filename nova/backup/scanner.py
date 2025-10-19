"""File scanning and categorization for backups."""

import typing as t
from pathlib import Path

from nova.adb.client import ADBClient


# Default whitelist of important paths to scan
DEFAULT_WHITELIST = [
    "/storage/emulated/0/DCIM",  # Camera photos
    "/storage/emulated/0/Pictures",  # Pictures
    "/storage/emulated/0/Download",  # Downloads
    "/storage/emulated/0/Documents",  # Documents
    "/storage/emulated/0/Music",  # Music
    "/storage/emulated/0/Movies",  # Videos
    "/storage/emulated/0/Android/data",  # App data (if accessible)
]

# File extensions by category
FILE_CATEGORIES = {
    "images": {".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".tiff", ".heic"},
    "videos": {".mp4", ".avi", ".mov", ".mkv", ".wmv", ".flv", ".webm", ".3gp"},
    "audio": {".mp3", ".wav", ".flac", ".aac", ".ogg", ".m4a", ".wma"},
    "documents": {".pdf", ".doc", ".docx", ".txt", ".rtf", ".odt", ".xls", ".xlsx", ".ppt", ".pptx"},
    "archives": {".zip", ".rar", ".7z", ".tar", ".gz", ".bz2", ".xz"},
    "apk": {".apk"},
}


class FileInfo:
    """Information about a file on the device."""
    
    def __init__(self, path: str, size: int, category: str = "other") -> None:
        self.path = path
        self.size = size
        self.category = category
    
    def __repr__(self) -> str:
        return f"FileInfo(path='{self.path}', size={self.size}, category='{self.category}')"


class BackupScanner:
    """Scans device for files to backup."""
    
    def __init__(self, adb_client: ADBClient, device_id: str) -> None:
        self.adb_client = adb_client
        self.device_id = device_id
    
    def categorize_file(self, file_path: str) -> str:
        """Categorize file by extension.
        
        Args:
            file_path: Path to the file
            
        Returns:
            Category name
        """
        ext = Path(file_path).suffix.lower()
        
        for category, extensions in FILE_CATEGORIES.items():
            if ext in extensions:
                return category
        
        return "other"
    
    def scan_directory(self, directory: str) -> t.List[FileInfo]:
        """Scan a directory for files.
        
        Args:
            directory: Directory path to scan
            
        Returns:
            List of FileInfo objects
        """
        files = []
        
        try:
            # List files with size using ls -la
            output = self.adb_client.shell(self.device_id, f"find '{directory}' -type f -exec ls -l {{}} \\; 2>/dev/null")
            
            for line in output.split('\n'):
                if not line.strip():
                    continue
                
                # Parse ls -l output (simplified)
                parts = line.split()
                if len(parts) >= 9:
                    try:
                        size = int(parts[4])
                        path = ' '.join(parts[8:])  # Handle spaces in filenames
                        category = self.categorize_file(path)
                        files.append(FileInfo(path, size, category))
                    except (ValueError, IndexError):
                        continue
        
        except Exception:
            # Directory might not exist or be accessible
            pass
        
        return files
    
    def scan_whitelist(self, whitelist: t.Optional[t.List[str]] = None) -> t.Dict[str, t.List[FileInfo]]:
        """Scan whitelist directories and categorize files.
        
        Args:
            whitelist: List of directories to scan (uses default if None)
            
        Returns:
            Dictionary mapping categories to file lists
        """
        if whitelist is None:
            whitelist = DEFAULT_WHITELIST
        
        categorized_files: t.Dict[str, t.List[FileInfo]] = {}
        
        for directory in whitelist:
            files = self.scan_directory(directory)
            
            for file_info in files:
                category = file_info.category
                if category not in categorized_files:
                    categorized_files[category] = []
                categorized_files[category].append(file_info)
        
        return categorized_files
    
    def get_scan_summary(self, categorized_files: t.Dict[str, t.List[FileInfo]]) -> t.Dict[str, t.Any]:
        """Get summary statistics for scanned files.
        
        Args:
            categorized_files: Output from scan_whitelist
            
        Returns:
            Summary statistics
        """
        total_files = sum(len(files) for files in categorized_files.values())
        total_size = sum(
            sum(file_info.size for file_info in files)
            for files in categorized_files.values()
        )
        
        category_stats = {}
        for category, files in categorized_files.items():
            category_stats[category] = {
                "count": len(files),
                "size": sum(file_info.size for file_info in files)
            }
        
        return {
            "total_files": total_files,
            "total_size": total_size,
            "categories": category_stats
        }
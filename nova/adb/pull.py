"""ADB file pulling utilities."""

import os
import shutil
from pathlib import Path
from typing import Callable, List, Optional

from tqdm import tqdm

from .device import ADBDevice, ADBError
from ..util.hashing import calculate_file_hash
from ..util.logging import get_logger
from ..util.paths import ensure_directory, format_size

logger = get_logger(__name__)


class FilePuller:
    """Utility for pulling files from Android device via ADB."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
    
    def pull_file(
        self, 
        device_path: str, 
        local_path: Path,
        verify_hash: bool = True,
        progress_callback: Optional[Callable[[int, int], None]] = None
    ) -> bool:
        """Pull a single file from device to local storage."""
        try:
            # Ensure local directory exists
            ensure_directory(local_path.parent)
            
            # Use ADB pull command
            logger.debug(f"Pulling {device_path} -> {local_path}")
            
            cmd = ["pull", device_path, str(local_path)]
            self.device._run_command(cmd, timeout=300)  # 5 minute timeout for large files
            
            # Verify the file was pulled successfully
            if not local_path.exists():
                logger.error(f"File was not pulled successfully: {local_path}")
                return False
            
            # Optional hash verification (limited by ADB capabilities)
            if verify_hash:
                if not self._verify_pulled_file(device_path, local_path):
                    logger.warning(f"Hash verification failed for {local_path}")
                    # Don't fail the operation, just warn
            
            logger.debug(f"Successfully pulled {device_path}")
            return True
            
        except ADBError as e:
            logger.error(f"Failed to pull {device_path}: {e}")
            return False
    
    def pull_files_batch(
        self,
        file_pairs: List[tuple[str, Path]],
        progress_callback: Optional[Callable[[int, int, str], None]] = None
    ) -> List[tuple[str, bool]]:
        """Pull multiple files in batch."""
        results = []
        total_files = len(file_pairs)
        
        logger.info(f"Starting batch pull of {total_files} files")
        
        for i, (device_path, local_path) in enumerate(file_pairs):
            if progress_callback:
                progress_callback(i + 1, total_files, device_path)
            
            success = self.pull_file(device_path, local_path)
            results.append((device_path, success))
            
            if not success:
                logger.warning(f"Failed to pull file {i+1}/{total_files}: {device_path}")
        
        successful = sum(1 for _, success in results if success)
        logger.info(f"Batch pull completed: {successful}/{total_files} files successful")
        
        return results
    
    def pull_directory(
        self,
        device_path: str,
        local_path: Path,
        recursive: bool = True,
        progress_callback: Optional[Callable[[int, int, str], None]] = None
    ) -> bool:
        """Pull entire directory from device."""
        try:
            logger.info(f"Pulling directory {device_path} -> {local_path}")
            
            # Ensure local directory exists
            ensure_directory(local_path)
            
            # Use ADB pull with directory
            cmd = ["pull", device_path, str(local_path)]
            self.device._run_command(cmd, timeout=1800)  # 30 minute timeout for directories
            
            logger.info(f"Successfully pulled directory {device_path}")
            return True
            
        except ADBError as e:
            logger.error(f"Failed to pull directory {device_path}: {e}")
            return False
    
    def get_file_size(self, device_path: str) -> Optional[int]:
        """Get size of a file on the device."""
        try:
            from .shell import ShellCommand
            shell = ShellCommand(self.device)
            
            stats = shell.get_file_stats(device_path)
            if stats and "size" in stats:
                return int(stats["size"])
                
        except (ADBError, ValueError) as e:
            logger.debug(f"Could not get file size for {device_path}: {e}")
        
        return None
    
    def estimate_pull_time(self, file_paths: List[str]) -> float:
        """Estimate time needed to pull files (rough estimate)."""
        total_size = 0
        
        for path in file_paths:
            size = self.get_file_size(path)
            if size:
                total_size += size
        
        # Rough estimate: 10 MB/s transfer rate
        if total_size > 0:
            return total_size / (10 * 1024 * 1024)
        
        return 0
    
    def _verify_pulled_file(self, device_path: str, local_path: Path) -> bool:
        """Verify pulled file integrity (basic size check)."""
        try:
            device_size = self.get_file_size(device_path)
            local_size = local_path.stat().st_size
            
            if device_size is not None:
                if device_size != local_size:
                    logger.warning(
                        f"Size mismatch for {local_path}: "
                        f"device={format_size(device_size)}, local={format_size(local_size)}"
                    )
                    return False
            
            return True
            
        except Exception as e:
            logger.debug(f"Could not verify file {local_path}: {e}")
            return False


def create_pull_progress_bar(total_files: int, desc: str = "Pulling files") -> tqdm:
    """Create a progress bar for file pulling operations."""
    return tqdm(
        total=total_files,
        desc=desc,
        unit="file",
        bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt} [{elapsed}<{remaining}, {rate_fmt}{postfix}]"
    )